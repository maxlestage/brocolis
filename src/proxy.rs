//! Cœur du proxy MITM.
//!
//! - HTTP simple : la requête arrive en forme absolue (`http://hôte/chemin`),
//!   elle est relayée via reqwest.
//! - HTTPS : la requête `CONNECT hôte:443` est interceptée. Le proxy répond
//!   `200`, termine le TLS côté client avec un certificat forgé par le CA
//!   interne, lit la requête en clair, la relaie via reqwest, puis applique le
//!   chiffrement/déchiffrement sélectif.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{HeaderMap, HeaderName};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::ca::Ca;
use crate::config::Config;
use crate::crypto::{decrypt_tokens_multi, encrypt_marked_multi};
use crate::keys::KeyManager;

type ResBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

/// État partagé entre toutes les connexions.
pub struct AppState {
    /// Autorité de certification interne, qui forge à la volée les certificats
    /// feuilles servis lors de l'interception HTTPS.
    pub ca: Arc<Ca>,
    /// Trousseau de clés symétriques utilisé pour chiffrer/déchiffrer le
    /// contenu marqué ; partagé et modifiable à chaud (rotation).
    pub keys: Arc<KeyManager>,
    /// Configuration effective du proxy (adresse d'écoute, hôtes à chiffrer, …).
    pub cfg: Arc<Config>,
    /// Client HTTP sortant réutilisé pour relayer les requêtes vers les serveurs
    /// distants.
    pub http: reqwest::Client,
}

/// En-têtes « hop-by-hop » à ne jamais relayer.
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "proxy-connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

fn full(body: impl Into<Bytes>) -> ResBody {
    Full::new(body.into())
        .map_err(|never| match never {})
        .boxed()
}

fn empty() -> ResBody {
    Full::new(Bytes::new())
        .map_err(|never| match never {})
        .boxed()
}

/// Déplie la chaîne `source()` d'une erreur pour un log lisible.
fn err_chain(e: &dyn std::error::Error) -> String {
    let mut out = e.to_string();
    let mut src = e.source();
    while let Some(s) = src {
        out.push_str(" -> ");
        out.push_str(&s.to_string());
        src = s.source();
    }
    out
}

fn bad_gateway(msg: &str) -> Response<ResBody> {
    tracing::error!("502: {msg}");
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .body(full(format!("Brocolis proxy error: {msg}")))
        .expect("réponse 502 valide")
}

/// Démarre la boucle d'acceptation TCP du proxy (tourne indéfiniment).
pub async fn run(state: Arc<AppState>, listen: SocketAddr) -> anyhow::Result<()> {
    run_until(state, listen, std::future::pending::<()>()).await
}

/// Identique à [`run`], mais s'arrête proprement dès que `shutdown` se résout.
/// Utilisé par l'interface graphique pour le bouton « Stop ».
pub async fn run_until(
    state: Arc<AppState>,
    listen: SocketAddr,
    shutdown: impl std::future::Future<Output = ()>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(listen).await?;
    tracing::info!("Brocolis écoute sur http://{listen} (proxy HTTP/HTTPS)");
    tokio::pin!(shutdown);
    loop {
        let (stream, peer) = tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("Arrêt du proxy demandé");
                return Ok(());
            }
            accepted = listener.accept() => match accepted {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("accept a échoué: {e}");
                    continue;
                }
            },
        };
        let state = state.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| handle(req, state.clone()));
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .preserve_header_case(true)
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                tracing::debug!("connexion {peer} terminée: {e}");
            }
        });
    }
}

async fn handle(
    req: Request<Incoming>,
    state: Arc<AppState>,
) -> Result<Response<ResBody>, Infallible> {
    if req.method() == Method::CONNECT {
        Ok(handle_connect(req, state))
    } else {
        Ok(forward_http(req, state, None).await)
    }
}

/// Gère `CONNECT hôte:port` : répond 200 puis termine le TLS côté client.
fn handle_connect(req: Request<Incoming>, state: Arc<AppState>) -> Response<ResBody> {
    let authority = match req.uri().authority().cloned() {
        Some(a) => a,
        None => return bad_gateway("CONNECT sans autorité"),
    };
    let host = authority.host().to_string();

    tokio::spawn(async move {
        let upgraded = match hyper::upgrade::on(req).await {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!("échec de l'upgrade CONNECT pour {host}: {e}");
                return;
            }
        };
        let tls_cfg = match state.ca.server_config_for(&host) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("certificat feuille impossible pour {host}: {e}");
                return;
            }
        };
        let acceptor = TlsAcceptor::from(tls_cfg);
        let tls_stream = match acceptor.accept(TokioIo::new(upgraded)).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("handshake TLS client échoué pour {host}: {e}");
                return;
            }
        };

        let host_for_service = host.clone();
        let inner_state = state.clone();
        let service = service_fn(move |inner_req| {
            let st = inner_state.clone();
            let h = host_for_service.clone();
            async move { Ok::<_, Infallible>(forward_http(inner_req, st, Some(h)).await) }
        });

        if let Err(e) = hyper::server::conn::http1::Builder::new()
            .preserve_header_case(true)
            .serve_connection(TokioIo::new(tls_stream), service)
            .await
        {
            tracing::debug!("session HTTPS {host} terminée: {e}");
        }
    });

    // Réponse immédiate : la connexion bascule ensuite en tunnel TLS.
    Response::builder()
        .status(StatusCode::OK)
        .body(empty())
        .expect("réponse 200 CONNECT valide")
}

/// Relaie une requête HTTP(S) en clair vers le serveur distant.
///
/// `tls_host` est `Some(hôte)` quand la requête provient d'un tunnel HTTPS
/// intercepté (URI en forme d'origine), `None` pour le proxy HTTP classique
/// (URI en forme absolue).
async fn forward_http(
    req: Request<Incoming>,
    state: Arc<AppState>,
    tls_host: Option<String>,
) -> Response<ResBody> {
    let method = req.method().clone();
    let (parts, body) = req.into_parts();

    // Reconstruit l'URL cible absolue.
    let (url, host) = match build_target_url(&parts.uri, &parts.headers, tls_host.as_deref()) {
        Some(v) => v,
        None => return bad_gateway("URL cible introuvable"),
    };

    let body_bytes = match body.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return bad_gateway(&format!("lecture du corps: {e}")),
    };

    let in_scope = state.cfg.host_in_scope(&host);

    // Chiffrement sélectif du corps sortant (segments `((...))`).
    let out_body: Bytes = if in_scope && is_textual(&parts.headers) {
        match std::str::from_utf8(&body_bytes) {
            Ok(text) => {
                let (transformed, n) = encrypt_marked_multi(text, &state.keys.encryption_keys());
                if n > 0 {
                    tracing::info!("{method} {host}: {n} segment(s) chiffré(s)");
                }
                Bytes::from(transformed.into_bytes())
            }
            Err(_) => body_bytes.clone(),
        }
    } else {
        body_bytes.clone()
    };

    // Construit la requête sortante.
    let mut rb = state.http.request(method.clone(), url.clone());
    rb = rb.headers(sanitized_headers(&parts.headers, true));
    rb = rb.body(out_body);

    let upstream = match rb.send().await {
        Ok(r) => r,
        Err(e) => return bad_gateway(&format!("requête vers {url} échouée: {}", err_chain(&e))),
    };

    let status = upstream.status();
    let resp_headers = upstream.headers().clone();
    let resp_body = match upstream.bytes().await {
        Ok(b) => b,
        Err(e) => return bad_gateway(&format!("lecture réponse {url}: {e}")),
    };

    // Déchiffrement sélectif du corps entrant (jetons `[[BROCOLIS:v1:...]]`).
    let final_body: Bytes = if in_scope && is_textual(&resp_headers) {
        match std::str::from_utf8(&resp_body) {
            Ok(text) => {
                let (transformed, n) = decrypt_tokens_multi(text, &state.keys.all_keys());
                if n > 0 {
                    tracing::info!("{method} {host} -> {status}: {n} segment(s) déchiffré(s)");
                }
                Bytes::from(transformed.into_bytes())
            }
            Err(_) => resp_body.clone(),
        }
    } else {
        resp_body.clone()
    };

    tracing::debug!("{method} {url} -> {status} ({} octets)", final_body.len());

    let mut builder = Response::builder().status(status);
    if let Some(hs) = builder.headers_mut() {
        *hs = sanitized_headers(&resp_headers, false);
    }
    builder
        .body(full(final_body))
        .unwrap_or_else(|e| bad_gateway(&format!("construction réponse: {e}")))
}

/// Construit l'URL absolue + le nom d'hôte cible.
fn build_target_url(
    uri: &hyper::Uri,
    headers: &HeaderMap,
    tls_host: Option<&str>,
) -> Option<(reqwest::Url, String)> {
    if let Some(host) = tls_host {
        // Tunnel HTTPS : URI en forme d'origine (chemin seul).
        let pq = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
        let url = reqwest::Url::parse(&format!("https://{host}{pq}")).ok()?;
        Some((url, host.to_string()))
    } else if uri.authority().is_some() {
        // Proxy HTTP : URI en forme absolue.
        let host = uri.host()?.to_string();
        let url = reqwest::Url::parse(&uri.to_string()).ok()?;
        Some((url, host))
    } else {
        // Repli : reconstruit depuis l'en-tête Host.
        let host = headers.get(hyper::header::HOST)?.to_str().ok()?.to_string();
        let pq = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
        let url = reqwest::Url::parse(&format!("http://{host}{pq}")).ok()?;
        let host_only = host.split(':').next().unwrap_or(&host).to_string();
        Some((url, host_only))
    }
}

/// Recopie les en-têtes en retirant ceux qui ne doivent pas être relayés.
///
/// `outgoing` = true pour une requête sortante (retire aussi `host` et
/// `accept-encoding` afin d'obtenir une réponse non compressée déchiffrable).
fn sanitized_headers(src: &HeaderMap, outgoing: bool) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (name, value) in src.iter() {
        let n = name.as_str().to_ascii_lowercase();
        if HOP_BY_HOP.contains(&n.as_str()) {
            continue;
        }
        if !outgoing && (n == "content-length" || n == "content-encoding") {
            continue;
        }
        if outgoing && (n == "host" || n == "accept-encoding" || n == "content-length") {
            continue;
        }
        if let Ok(hn) = HeaderName::from_bytes(name.as_ref()) {
            out.append(hn, value.clone());
        }
    }
    out
}

/// Détermine si le corps est du texte transformable.
fn is_textual(headers: &HeaderMap) -> bool {
    // Ne transforme jamais un contenu encodé/compressé.
    if let Some(enc) = headers.get(hyper::header::CONTENT_ENCODING) {
        if enc
            .to_str()
            .map(|v| !v.trim().eq_ignore_ascii_case("identity"))
            .unwrap_or(true)
        {
            return false;
        }
    }
    match headers.get(hyper::header::CONTENT_TYPE) {
        None => true, // pas de type : on tente (souvent du texte court)
        Some(ct) => {
            let ct = ct.to_str().unwrap_or("").to_ascii_lowercase();
            ct.starts_with("text/")
                || ct.contains("json")
                || ct.contains("xml")
                || ct.contains("x-www-form-urlencoded")
                || ct.contains("javascript")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textual_detection() {
        let mut h = HeaderMap::new();
        assert!(is_textual(&h)); // pas de content-type

        h.insert(
            hyper::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        assert!(is_textual(&h));

        h.insert(hyper::header::CONTENT_TYPE, "image/png".parse().unwrap());
        assert!(!is_textual(&h));

        h.insert(
            hyper::header::CONTENT_TYPE,
            "text/html; charset=utf-8".parse().unwrap(),
        );
        h.insert(hyper::header::CONTENT_ENCODING, "gzip".parse().unwrap());
        assert!(!is_textual(&h)); // compressé -> non transformable
    }

    #[test]
    fn hop_by_hop_stripped() {
        let mut h = HeaderMap::new();
        h.insert(hyper::header::HOST, "example.com".parse().unwrap());
        h.insert(hyper::header::CONNECTION, "keep-alive".parse().unwrap());
        h.insert("x-keep", "1".parse().unwrap());

        let out = sanitized_headers(&h, true);
        assert!(out.get("connection").is_none());
        assert!(out.get("host").is_none()); // retiré côté sortant
        assert_eq!(out.get("x-keep").unwrap(), "1");

        let resp = sanitized_headers(&h, false);
        assert!(resp.get("host").is_some()); // conservé côté réponse
        assert!(resp.get("connection").is_none());
    }

    #[test]
    fn target_url_absolute_form() {
        let uri: hyper::Uri = "http://example.com/path?q=1".parse().unwrap();
        let (url, host) = build_target_url(&uri, &HeaderMap::new(), None).unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(url.as_str(), "http://example.com/path?q=1");
    }

    #[test]
    fn target_url_tls_origin_form() {
        let uri: hyper::Uri = "/api/tweets?x=2".parse().unwrap();
        let (url, host) = build_target_url(&uri, &HeaderMap::new(), Some("twitter.com")).unwrap();
        assert_eq!(host, "twitter.com");
        assert_eq!(url.as_str(), "https://twitter.com/api/tweets?x=2");
    }
}
