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
use crate::crypto::{decrypt_tokens_escaped, encrypt_marked_form, encrypt_marked_multi, PlainEscape};
use crate::keys::KeyManager;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

// `UnsyncBoxBody` et non `BoxBody` : un corps relayé au fil de l'eau enveloppe
// le flux de reqwest, qui n'est pas `Sync`. Hyper n'exige pas `Sync` du corps.
//
// L'erreur n'est PAS `Infallible` : une coupure amont en plein corps doit
// pouvoir remonter jusqu'à hyper, qui abandonne alors la réponse au lieu de la
// terminer proprement. Sinon le client reçoit « 200 OK » avec un corps tronqué
// et n'a aucun moyen de s'en apercevoir.
type ResBody = http_body_util::combinators::UnsyncBoxBody<Bytes, BoxError>;

/// État partagé entre toutes les connexions.
pub struct AppState {
    pub ca: Arc<Ca>,
    pub keys: Arc<KeyManager>,
    pub cfg: Arc<Config>,
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

/// Au-delà de cette taille, un corps n'est plus mis en mémoire pour être
/// réécrit : il est relayé au fil de l'eau, tel quel.
///
/// La réécriture impose de tenir le corps entier en mémoire (il faut le lire en
/// UTF-8 pour y chercher les marqueurs). Sans plafond, télécharger un fichier de
/// 2 Gio à travers le proxy réclamait 2 Gio de RAM. Les messages qu'on chiffre
/// tiennent dans quelques kilo-octets ; 8 Mio laissent une marge confortable.
const MAX_REWRITE_BYTES: u64 = 8 * 1024 * 1024;

/// Taille annoncée par `Content-Length`, si elle est exploitable.
fn declared_len(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(hyper::header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

/// Décide si un corps doit être mis en mémoire pour être réécrit. Un corps hors
/// portée ou non textuel passe en relais direct.
///
/// Une taille annoncée au-dessus du plafond tranche sans rien lire. En son
/// absence (réponse « chunked »), on ne peut pas savoir d'avance : la lecture
/// est alors plafonnée à la volée par [`collect_capped`].
fn should_rewrite(in_scope: bool, headers: &HeaderMap) -> bool {
    if !in_scope || !is_textual(headers) {
        return false;
    }
    declared_len(headers).is_none_or(|n| n <= MAX_REWRITE_BYTES)
}

fn full(body: impl Into<Bytes>) -> ResBody {
    Full::new(body.into())
        .map_err(|e: Infallible| -> BoxError { match e {} })
        .boxed_unsync()
}

/// Relaie un flux d'octets sans jamais le tenir entièrement en mémoire.
fn stream_body(
    stream: impl futures_core::Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
) -> ResBody {
    use http_body_util::StreamBody;
    use hyper::body::Frame;
    let mapped = futures_util::StreamExt::map(stream, |chunk| {
        chunk
            .map(Frame::data)
            .map_err(|e| -> BoxError { Box::new(e) })
    });
    StreamBody::new(mapped).boxed_unsync()
}

/// Résultat d'une lecture plafonnée.
enum Capped {
    /// Le corps tient sous le plafond : il est entièrement en mémoire.
    Whole(Bytes),
    /// Le corps dépasse le plafond. Il n'est PAS réécrit ; ce qui a déjà été lu
    /// est restitué en tête, puis le reste continue au fil de l'eau.
    TooBig(ResBody),
}

/// Lit un corps de réponse en s'arrêtant net au-dessus de `cap`.
///
/// Sans cela, une réponse « chunked » (donc sans `Content-Length`) échappait
/// totalement au plafond : elle était bufferisée intégralement, quelle que soit
/// sa taille.
async fn collect_capped(resp: reqwest::Response, cap: usize) -> Capped {
    use bytes::BytesMut;
    use futures_util::StreamExt;
    use http_body_util::StreamBody;
    use hyper::body::Frame;

    let mut stream = resp.bytes_stream();
    let mut buf = BytesMut::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
                buf.extend_from_slice(&c);
                if buf.len() > cap {
                    tracing::warn!(
                        "corps textuel au-delà de {} Mio sans taille annoncée : \
                         relayé sans réécriture",
                        cap / 1024 / 1024
                    );
                    let prefix = buf.freeze();
                    let head = futures_util::stream::once(async move {
                        Ok::<_, BoxError>(Frame::data(prefix))
                    });
                    let tail = stream.map(|c| {
                        c.map(Frame::data).map_err(|e| -> BoxError { Box::new(e) })
                    });
                    return Capped::TooBig(StreamBody::new(head.chain(tail)).boxed_unsync());
                }
            }
            Err(e) => {
                // Erreur amont pendant la lecture : on la propage pour que le
                // client ne prenne pas un corps partiel pour un succès.
                let err = futures_util::stream::once(async move {
                    Err::<Frame<Bytes>, BoxError>(Box::new(e) as BoxError)
                });
                return Capped::TooBig(StreamBody::new(err).boxed_unsync());
            }
        }
    }
    Capped::Whole(buf.freeze())
}

fn empty() -> ResBody {
    Full::new(Bytes::new())
        .map_err(|never| match never {})
        .boxed_unsync()
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

/// Depuis l'autorité d'un `CONNECT`, renvoie
/// `(nom d'hôte pour le certificat, autorité complète pour l'URL amont)`.
///
/// Les deux diffèrent, et c'est tout l'enjeu : un SAN de certificat ne porte
/// pas de port, alors que l'URL amont DOIT le conserver. Le port était
/// auparavant jeté ici même, si bien qu'un `CONNECT hôte:8443` repartait vers
/// le 443 de l'hôte — tout site servi en HTTPS sur un autre port était
/// silencieusement relayé au mauvais endroit.
fn tunnel_targets(authority: &hyper::http::uri::Authority) -> (String, String) {
    (authority.host().to_string(), authority.to_string())
}

/// Gère `CONNECT hôte:port` : répond 200 puis termine le TLS côté client.
fn handle_connect(req: Request<Incoming>, state: Arc<AppState>) -> Response<ResBody> {
    let authority = match req.uri().authority().cloned() {
        Some(a) => a,
        None => return bad_gateway("CONNECT sans autorité"),
    };
    let (host, target_authority) = tunnel_targets(&authority);

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

        let authority_for_service = target_authority.clone();
        let inner_state = state.clone();
        let service = service_fn(move |inner_req| {
            let st = inner_state.clone();
            let a = authority_for_service.clone();
            async move { Ok::<_, Infallible>(forward_http(inner_req, st, Some(a)).await) }
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
/// `tls_authority` est `Some(hôte:port)` quand la requête provient d'un tunnel HTTPS
/// intercepté (URI en forme d'origine), `None` pour le proxy HTTP classique
/// (URI en forme absolue).
async fn forward_http(
    req: Request<Incoming>,
    state: Arc<AppState>,
    tls_authority: Option<String>,
) -> Response<ResBody> {
    let method = req.method().clone();
    let (parts, body) = req.into_parts();

    // Reconstruit l'URL cible absolue.
    let (url, host) = match build_target_url(&parts.uri, &parts.headers, tls_authority.as_deref()) {
        Some(v) => v,
        None => return bad_gateway("URL cible introuvable"),
    };

    // Brocolis ne sait pas relayer un changement de protocole : `Connection` et
    // `Upgrade` font partie des en-têtes hop-by-hop retirés, et rien ne traite
    // une réponse 101. Une négociation WebSocket échoue donc — silencieusement
    // jusqu'ici, ce qui rendait la panne indiagnosticable côté utilisateur.
    if wants_upgrade(&parts.headers) {
        tracing::warn!(
            "{method} {host}: changement de protocole demandé ({}) — non relayé par brocolis, \
             la connexion échouera côté client",
            parts
                .headers
                .get(hyper::header::UPGRADE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("?")
        );
    }

    let in_scope = state.cfg.host_in_scope(&host);

    // Le corps n'est mis en mémoire que si on doit vraiment le réécrire ; sinon
    // il est relayé au fil de l'eau (voir MAX_REWRITE_BYTES).
    let out_body: reqwest::Body = if should_rewrite(in_scope, &parts.headers) {
        let body_bytes = match body.collect().await {
            Ok(c) => c.to_bytes(),
            Err(e) => return bad_gateway(&format!("lecture du corps: {e}")),
        };
        match std::str::from_utf8(&body_bytes) {
            Ok(text) => {
                // Un formulaire de navigateur transmet `((secret))` sous la
                // forme `%28%28secret%29%29` : les marqueurs doivent être
                // cherchés APRÈS décodage, sinon le secret part en clair.
                let (transformed, n) = if is_form_urlencoded(&parts.headers) {
                    encrypt_marked_form(text, &state.keys.encryption_keys())
                } else {
                    encrypt_marked_multi(text, &state.keys.encryption_keys())
                };
                if n > 0 {
                    tracing::info!("{method} {host}: {n} segment(s) chiffré(s)");
                }
                reqwest::Body::from(transformed.into_bytes())
            }
            Err(_) => reqwest::Body::from(body_bytes),
        }
    } else {
        let stream = futures_util::StreamExt::map(body.into_data_stream(), |r| {
            r.map_err(std::io::Error::other)
        });
        reqwest::Body::wrap_stream(stream)
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

    // Même règle au retour : on ne bufferise que ce qu'on va réellement
    // déchiffrer. Un téléchargement binaire traverse le proxy sans jamais
    // occuper la mémoire.
    let final_body: ResBody = if should_rewrite(in_scope, &resp_headers) {
        match collect_capped(upstream, MAX_REWRITE_BYTES as usize).await {
            Capped::Whole(resp_body) => {
                let out = match std::str::from_utf8(&resp_body) {
                    Ok(text) => {
                        let (transformed, n) = decrypt_tokens_escaped(
                            text,
                            &state.keys.all_keys(),
                            escape_for(&resp_headers),
                        );
                        if n > 0 {
                            tracing::info!(
                                "{method} {host} -> {status}: {n} segment(s) déchiffré(s)"
                            );
                        }
                        Bytes::from(transformed.into_bytes())
                    }
                    Err(_) => resp_body,
                };
                tracing::debug!("{method} {url} -> {status} ({} octets)", out.len());
                full(out)
            }
            Capped::TooBig(body) => body,
        }
    } else {
        tracing::debug!("{method} {url} -> {status} (relais au fil de l'eau)");
        stream_body(upstream.bytes_stream())
    };

    let mut builder = Response::builder().status(status);
    if let Some(hs) = builder.headers_mut() {
        *hs = sanitized_headers(&resp_headers, false);
    }
    builder
        .body(final_body)
        .unwrap_or_else(|e| bad_gateway(&format!("construction réponse: {e}")))
}

/// Choisit l'échappement du clair d'après le type du document qui l'accueille.
///
/// Le texte déchiffré n'est pas du balisage : sans échappement, un secret
/// contenant `<script>` s'exécutait dans l'origine du site consulté, et un
/// secret contenant `"` rendait une réponse JSON invalide.
fn escape_for(headers: &HeaderMap) -> PlainEscape {
    let ct = headers
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ct.contains("html") || ct.contains("xml") {
        PlainEscape::Html
    } else if ct.contains("json") {
        PlainEscape::Json
    } else {
        // text/plain et assimilés : le clair est déjà du texte.
        PlainEscape::None
    }
}

/// Construit l'URL absolue + le nom d'hôte cible (sans port).
///
/// `tls_authority` porte l'autorité COMPLÈTE du `CONNECT`, port compris. Elle
/// ne contenait auparavant que le nom d'hôte : le port était donc perdu et tout
/// le trafic repartait vers le 443, ce qui cassait silencieusement les sites
/// servis en HTTPS sur un autre port.
fn build_target_url(
    uri: &hyper::Uri,
    headers: &HeaderMap,
    tls_authority: Option<&str>,
) -> Option<(reqwest::Url, String)> {
    if let Some(authority) = tls_authority {
        // Tunnel HTTPS : URI en forme d'origine (chemin seul).
        let pq = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
        let url = reqwest::Url::parse(&format!("https://{authority}{pq}")).ok()?;
        // Le nom d'hôte renvoyé sert à la portée et aux journaux : sans port.
        let host = url.host_str()?.to_string();
        Some((url, host))
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

/// La requête demande-t-elle un changement de protocole (WebSocket…) ?
fn wants_upgrade(headers: &HeaderMap) -> bool {
    headers.contains_key(hyper::header::UPGRADE)
        || headers
            .get(hyper::header::CONNECTION)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_ascii_lowercase().contains("upgrade"))
            .unwrap_or(false)
}

/// Le corps est-il un formulaire urlencodé (soumission navigateur standard) ?
fn is_form_urlencoded(headers: &HeaderMap) -> bool {
    headers
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.to_ascii_lowercase().contains("x-www-form-urlencoded"))
        .unwrap_or(false)
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

    /// Le port du `CONNECT` doit survivre jusqu'à l'URL amont.
    ///
    /// Il était perdu : seul le nom d'hôte était transmis, donc `https://hôte`
    /// — c'est-à-dire le port 443 — quel que soit le port demandé. Tout site
    /// servi en HTTPS sur un autre port était silencieusement relayé au mauvais
    /// endroit.
    #[test]
    fn le_port_du_tunnel_est_conserve() {
        let a: hyper::http::uri::Authority = "example.com:8443".parse().unwrap();
        let (cert_host, cible) = tunnel_targets(&a);
        assert_eq!(cert_host, "example.com", "le certificat se fait sans port");
        assert_eq!(cible, "example.com:8443", "l'URL amont garde le port");

        // Et l'URL effectivement construite pointe bien vers ce port.
        let uri: hyper::Uri = "/chemin?a=1".parse().unwrap();
        let h = HeaderMap::new();
        let (url, host) = build_target_url(&uri, &h, Some(&cible)).unwrap();
        assert_eq!(url.as_str(), "https://example.com:8443/chemin?a=1");
        assert_eq!(url.port(), Some(8443));
        assert_eq!(host, "example.com", "la portée s'évalue sans le port");
    }

    /// Une adresse IPv6 littérale dans le `CONNECT` doit rester exploitable.
    #[test]
    fn une_autorite_ipv6_reste_valide() {
        let uri: hyper::Uri = "/".parse().unwrap();
        let h = HeaderMap::new();
        let a: hyper::http::uri::Authority = "[::1]:8443".parse().unwrap();
        let (_, cible) = tunnel_targets(&a);
        let (url, host) = build_target_url(&uri, &h, Some(&cible)).unwrap();
        assert_eq!(url.port(), Some(8443));
        assert_eq!(host, "[::1]");
    }

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
