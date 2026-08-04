//! Bout en bout à travers le VRAI proxy : un client parle à un serveur
//! d'origine en passant par brocolis.
//!
//! Ces tests couvrent ce que la suite ne couvrait pas : que le chiffrement
//! sélectif fonctionne réellement dans le chemin de données du proxy, et pas
//! seulement dans les fonctions de `crypto.rs` prises isolément. Ils fixent
//! aussi le comportement du plafond `MAX_REWRITE_BYTES` (8 Mio) : au-delà, le
//! corps est relayé tel quel plutôt que mis en mémoire.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use brocolis::{build_app_state, config::Config, SymKey};

/// Clé de test (32 octets 0x00..0x1f), la même que les vecteurs d'interop.
const KEY_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

/// Serveur d'origine : mémorise le corps reçu et renvoie `reply`.
struct Origin {
    addr: SocketAddr,
    seen: Arc<Mutex<Vec<String>>>,
}

async fn origin(reply: String, content_type: &'static str) -> Origin {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen2 = seen.clone();

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let reply = reply.clone();
            let seen = seen2.clone();
            tokio::spawn(async move {
                // Lit l'intégralité de la requête (en-têtes + corps).
                let mut raw = Vec::new();
                let mut buf = [0u8; 16 * 1024];
                loop {
                    let Ok(n) = sock.read(&mut buf).await else { return };
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&raw);
                    if let Some(pos) = text.find("\r\n\r\n") {
                        let len: usize = text
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse().ok())
                            })
                            .unwrap_or(0);
                        if raw.len() >= pos + 4 + len {
                            let body = String::from_utf8_lossy(&raw[pos + 4..]).into_owned();
                            seen.lock().unwrap().push(body);
                            break;
                        }
                    }
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n",
                    reply.len()
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(reply.as_bytes()).await;
                let _ = sock.flush().await;
            });
        }
    });
    Origin { addr, seen }
}

/// Démarre un proxy brocolis avec la clé de test et renvoie son adresse.
async fn start_proxy() -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    let dir = std::env::temp_dir().join(format!(
        "brocolis-e2e-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    let cfg = Config {
        ca_cert: dir.join("ca.crt"),
        ca_key: dir.join("ca.key"),
        key: Some(KEY_B64.to_string()),
        ..Config::default()
    };

    let (state, _keys) = build_app_state(cfg).unwrap();
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    drop(l);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let _ = brocolis::run_until(state, addr, async {
            let _ = rx.await;
        })
        .await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    (addr, tx)
}

fn client(proxy: SocketAddr) -> reqwest::Client {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{proxy}")).unwrap())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

/// Un `((secret))` envoyé par le client doit arriver CHIFFRÉ chez l'origine.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn le_segment_marque_est_chiffre_avant_de_sortir() {
    let o = origin("ok".to_string(), "text/plain").await;
    let (proxy, _stop) = start_proxy().await;

    let resp = client(proxy)
        .post(format!("http://{}/envoi", o.addr))
        .header("content-type", "text/plain")
        .body("rendez-vous ((à 18h, code 4291)) merci")
        .send()
        .await
        .expect("requête relayée");
    assert_eq!(resp.status(), 200);

    let seen = o.seen.lock().unwrap().clone();
    let recu = seen.first().expect("l'origine a reçu un corps");

    assert!(
        !recu.contains("18h, code 4291"),
        "le secret ne doit PAS sortir en clair — reçu : {recu}"
    );
    assert!(
        recu.contains("[[BROCOLIS:v1:"),
        "un jeton chiffré doit remplacer le segment — reçu : {recu}"
    );
    assert!(
        recu.contains("rendez-vous ") && recu.contains(" merci"),
        "le texte autour du marqueur doit être conservé — reçu : {recu}"
    );
}

/// Un jeton renvoyé par l'origine doit arriver DÉCHIFFRÉ chez le client.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn le_jeton_de_la_reponse_est_dechiffre_pour_le_client() {
    let key = SymKey::from_base64(KEY_B64).unwrap();
    let token = key.encrypt_token("message secret 🥦".as_bytes()).unwrap();
    let o = origin(format!("début {token} fin"), "text/plain").await;
    let (proxy, _stop) = start_proxy().await;

    let corps = client(proxy)
        .get(format!("http://{}/lecture", o.addr))
        .send()
        .await
        .expect("requête relayée")
        .text()
        .await
        .unwrap();

    assert!(
        corps.contains("message secret 🥦"),
        "le jeton doit être déchiffré pour le client — reçu : {corps}"
    );
    assert!(
        !corps.contains("[[BROCOLIS:"),
        "aucun jeton ne doit subsister — reçu : {corps}"
    );
}

/// Un corps binaire traverse le proxy intact : aucune réécriture, aucune
/// corruption. C'est le chemin « relais au fil de l'eau ».
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_corps_binaire_traverse_intact() {
    // Des octets qui ressemblent à un marqueur mais dans un binaire : ils
    // doivent ressortir tels quels.
    let charge = "((ceci n'est pas à chiffrer))".repeat(50);
    let o = origin(charge.clone(), "application/octet-stream").await;
    let (proxy, _stop) = start_proxy().await;

    let corps = client(proxy)
        .get(format!("http://{}/binaire", o.addr))
        .send()
        .await
        .expect("requête relayée")
        .text()
        .await
        .unwrap();

    assert_eq!(corps, charge, "un corps non textuel doit passer tel quel");
}

/// Au-delà du plafond de réécriture, un corps textuel est relayé tel quel
/// plutôt que d'être mis en mémoire. Le contenu doit rester intact.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_corps_textuel_enorme_passe_sans_etre_reecrit() {
    // > 8 Mio : au-dessus de MAX_REWRITE_BYTES.
    let charge = "x".repeat(9 * 1024 * 1024);
    let o = origin(charge.clone(), "text/plain").await;
    let (proxy, _stop) = start_proxy().await;

    let corps = client(proxy)
        .get(format!("http://{}/gros.txt", o.addr))
        .send()
        .await
        .expect("requête relayée")
        .text()
        .await
        .unwrap();

    assert_eq!(corps.len(), charge.len(), "le corps doit arriver entier");
    assert_eq!(corps, charge, "et intact");
}

/// Un corps textuel juste SOUS le plafond doit, lui, être bien réécrit.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_corps_textuel_sous_le_plafond_est_bien_reecrit() {
    let key = SymKey::from_base64(KEY_B64).unwrap();
    let token = key.encrypt_token(b"petit secret").unwrap();
    let bourrage = "a".repeat(1024 * 1024); // 1 Mio, sous le plafond
    let o = origin(format!("{bourrage}{token}"), "text/plain").await;
    let (proxy, _stop) = start_proxy().await;

    let corps = client(proxy)
        .get(format!("http://{}/moyen.txt", o.addr))
        .send()
        .await
        .expect("requête relayée")
        .text()
        .await
        .unwrap();

    assert!(
        corps.contains("petit secret"),
        "un corps sous le plafond doit être déchiffré"
    );
}
