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

/// Une réponse « chunked » n'annonce aucune taille : le plafond de réécriture
/// ne peut pas être décidé à l'avance. Il doit donc s'appliquer À LA LECTURE,
/// sinon un corps textuel sans `Content-Length` est bufferisé sans aucune borne.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn une_reponse_chunked_enorme_reste_plafonnee() {
    use tokio::io::AsyncWriteExt;
    const SIZE: usize = 192 * 1024 * 1024;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut b = [0u8; 8192];
                let _ = sock.read(&mut b).await;
                let head = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
                            Transfer-Encoding: chunked\r\n\r\n";
                if sock.write_all(head.as_bytes()).await.is_err() { return; }
                let payload = vec![b'a'; 64 * 1024];
                let mut sent = 0usize;
                while sent < SIZE {
                    let n = payload.len().min(SIZE - sent);
                    if sock.write_all(format!("{n:x}\r\n").as_bytes()).await.is_err() { return; }
                    if sock.write_all(&payload[..n]).await.is_err() { return; }
                    if sock.write_all(b"\r\n").await.is_err() { return; }
                    sent += n;
                }
                let _ = sock.write_all(b"0\r\n\r\n").await;
            });
        }
    });

    let (proxy, _stop) = start_proxy().await;
    let before = rss();
    let resp = client(proxy)
        .get(format!("http://{origin_addr}/page.html"))
        .send()
        .await
        .expect("requête relayée");
    assert_eq!(resp.status(), 200);

    let mut recu = 0usize;
    {
        use futures_util::StreamExt;
        let mut st = resp.bytes_stream();
        while let Some(c) = st.next().await {
            recu += c.expect("morceau").len();
        }
    }
    let growth = rss().saturating_sub(before);
    eprintln!("chunked : reçu {} Mio, croissance {} Mio", recu / 1048576, growth / 1048576);
    assert_eq!(recu, SIZE, "le corps doit arriver entier");
    assert!(
        growth < (SIZE as u64) / 3,
        "plafond contourné en chunked : croissance {} Mio pour {} Mio",
        growth / 1048576,
        SIZE / 1048576
    );
}

/// Si l'amont coupe en plein corps, le client ne doit PAS recevoir un « 200 OK »
/// tronqué qu'il prendrait pour un succès : l'erreur doit remonter.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn une_coupure_amont_ne_passe_pas_pour_un_succes() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut b = [0u8; 8192];
                let _ = sock.read(&mut b).await;
                let head = "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n\
                            Content-Length: 1000000\r\n\r\n";
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(&vec![0x42u8; 1000]).await;
                let _ = sock.flush().await;
                // Coupe net : 1000 octets livrés sur 1000000 annoncés.
                drop(sock);
            });
        }
    });

    let (proxy, _stop) = start_proxy().await;
    let resp = client(proxy)
        .get(format!("http://{origin_addr}/tronque.bin"))
        .send()
        .await
        .expect("les en-têtes arrivent");
    let status = resp.status();
    let corps = resp.bytes().await;
    eprintln!("coupure amont : status={status} corps={:?}", corps.as_ref().map(|b| b.len()));

    let succes_silencieux = status == 200 && matches!(corps.as_ref().map(|b| b.len()), Ok(1000));
    assert!(
        !succes_silencieux,
        "troncature silencieuse : 200 OK avec 1000 octets sur 1000000 annoncés"
    );
}

/// Mémoire résidente du processus, en octets.
fn rss() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = statm.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    pages * 4096
}

/// Un secret contenant du balisage ne doit PAS devenir du balisage dans la
/// page : sans échappement, `<script>` s'exécutait dans l'origine du site
/// consulté, avec les droits de la session de l'utilisateur sur ce site.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn le_clair_ne_devient_pas_du_balisage_dans_une_page_html() {
    let key = SymKey::from_base64(KEY_B64).unwrap();
    let token = key.encrypt_token(b"<script>alert(1)</script>").unwrap();
    let o = origin(
        format!("<html><body><p>{token}</p></body></html>"),
        "text/html; charset=utf-8",
    )
    .await;
    let (proxy, _stop) = start_proxy().await;

    let page = client(proxy)
        .get(format!("http://{}/page.html", o.addr))
        .send()
        .await
        .expect("requête relayée")
        .text()
        .await
        .unwrap();

    assert!(
        !page.contains("<script>alert(1)</script>"),
        "le balisage du secret a été injecté tel quel : {page}"
    );
    assert!(
        page.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
        "le secret doit rester lisible, mais échappé : {page}"
    );
}

/// Un secret contenant un guillemet ne doit pas casser une réponse JSON.
/// Sans échappement, le document devenait invalide et la page échouait.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn le_clair_ne_casse_pas_une_reponse_json() {
    let key = SymKey::from_base64(KEY_B64).unwrap();
    let token = key
        .encrypt_token(b"il a dit \"bonjour\" puis \\ fin")
        .unwrap();
    let o = origin(format!(r#"{{"message":"{token}"}}"#), "application/json").await;
    let (proxy, _stop) = start_proxy().await;

    let corps = client(proxy)
        .get(format!("http://{}/api", o.addr))
        .send()
        .await
        .expect("requête relayée")
        .text()
        .await
        .unwrap();

    let parsed: serde_json::Value =
        serde_json::from_str(&corps).unwrap_or_else(|e| panic!("JSON cassé ({e}) : {corps}"));
    assert_eq!(
        parsed["message"], "il a dit \"bonjour\" puis \\ fin",
        "le clair doit être restitué intact après analyse JSON"
    );
}

/// En text/plain, rien à échapper : le clair sort tel quel.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn en_texte_brut_le_clair_reste_intact() {
    let key = SymKey::from_base64(KEY_B64).unwrap();
    let token = key.encrypt_token(b"3 < 5 & \"vrai\"").unwrap();
    let o = origin(format!("avant {token} apres"), "text/plain").await;
    let (proxy, _stop) = start_proxy().await;

    let corps = client(proxy)
        .get(format!("http://{}/note.txt", o.addr))
        .send()
        .await
        .expect("requête relayée")
        .text()
        .await
        .unwrap();

    assert_eq!(corps, "avant 3 < 5 & \"vrai\" apres");
}

/// Le cas d'usage principal : l'utilisateur tape `((secret))` dans un champ de
/// formulaire. Le navigateur l'envoie percent-encodé ; le secret doit quand
/// même être chiffré avant de sortir.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_secret_saisi_dans_un_formulaire_est_chiffre() {
    let o = origin("ok".to_string(), "text/plain").await;
    let (proxy, _stop) = start_proxy().await;

    // Exactement ce qu'émet un navigateur pour <input name="t" value="((mon secret))">
    let resp = client(proxy)
        .post(format!("http://{}/post", o.addr))
        .header("content-type", "application/x-www-form-urlencoded")
        .body("t=%28%28mon+secret%29%29&autre=bonjour")
        .send()
        .await
        .expect("requête relayée");
    assert_eq!(resp.status(), 200);

    let recu = o.seen.lock().unwrap().first().cloned().expect("corps reçu");
    assert!(
        !recu.contains("mon+secret") && !recu.contains("mon%20secret"),
        "le secret est parti en clair : {recu}"
    );
    assert!(
        recu.contains("BROCOLIS"),
        "un jeton doit remplacer le secret : {recu}"
    );
    assert!(
        recu.contains("autre=bonjour"),
        "un champ sans marqueur doit rester intact : {recu}"
    );
}

/// Un formulaire SANS marqueur ne doit pas être touché du tout : c'est ce qui
/// borne le risque de la réécriture des corps de formulaire.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_formulaire_sans_marqueur_sort_identique() {
    let o = origin("ok".to_string(), "text/plain").await;
    let (proxy, _stop) = start_proxy().await;

    let corps = "a=1&b=deux+mots&c=%C3%A9t%C3%A9&d&e=x%3Dy";
    client(proxy)
        .post(format!("http://{}/post", o.addr))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(corps)
        .send()
        .await
        .expect("requête relayée");

    let recu = o.seen.lock().unwrap().first().cloned().expect("corps reçu");
    assert_eq!(recu, corps, "le formulaire doit sortir octet pour octet");
}

/// Le jeton réencodé doit survivre au décodage du serveur : `+`, `/` et `=`
/// du base64 seraient sinon réinterprétés et le jeton détruit.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn le_jeton_survit_au_decodage_du_formulaire() {
    let o = origin("ok".to_string(), "text/plain").await;
    let (proxy, _stop) = start_proxy().await;

    client(proxy)
        .post(format!("http://{}/post", o.addr))
        .header("content-type", "application/x-www-form-urlencoded")
        .body("t=%28%28secret+a+chiffrer+avec+un+peu+de+longueur%29%29")
        .send()
        .await
        .expect("requête relayée");

    let recu = o.seen.lock().unwrap().first().cloned().expect("corps reçu");
    // On rejoue le décodage que ferait le serveur.
    let valeur = recu.split_once('=').expect("champ").1;
    let decode = percent_decode(valeur);
    let key = SymKey::from_base64(KEY_B64).unwrap();
    let (clair, n) = brocolis::crypto::decrypt_tokens(&decode, &key);
    assert_eq!(n, 1, "le serveur doit voir un jeton exploitable : {decode}");
    assert_eq!(clair, "secret a chiffrer avec un peu de longueur");
}

/// Décodage urlencodé minimal, tel que le ferait le serveur destinataire.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let h = (b[i + 1] as char).to_digit(16).unwrap();
                let l = (b[i + 2] as char).to_digit(16).unwrap();
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap()
}

/// Un corps compressé doit arriver au client AVEC son `Content-Encoding`.
///
/// Il était retiré alors que le corps compressé était relayé tel quel : le
/// navigateur recevait des octets gzip présentés comme du texte, et affichait
/// du binaire. Le cas survient dès qu'un serveur compresse sans qu'on l'ait
/// demandé.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn un_corps_compresse_garde_son_encodage() {
    use tokio::io::AsyncWriteExt;
    // « hello » compressé en gzip.
    const GZ: &[u8] = &[
        0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xcb, 0x48, 0xcd, 0xc9, 0xc9,
        0x07, 0x00, 0x86, 0xa6, 0x10, 0x36, 0x05, 0x00, 0x00, 0x00,
    ];

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut b = [0u8; 8192];
                let _ = sock.read(&mut b).await;
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\
                     Content-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    GZ.len()
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(GZ).await;
            });
        }
    });

    let (proxy, _stop) = start_proxy().await;
    let resp = client(proxy)
        .get(format!("http://{origin_addr}/page.html"))
        .send()
        .await
        .expect("requête relayée");

    assert_eq!(
        resp.headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok()),
        Some("gzip"),
        "l'encodage doit être restitué, sinon le corps est illisible"
    );
    let corps = resp.bytes().await.unwrap();
    assert_eq!(&corps[..], GZ, "les octets compressés doivent être intacts");
}

/// Une réponse à HEAD doit conserver les en-têtes qui décrivent la ressource :
/// c'est tout l'intérêt de la méthode. Les réécrire revenait à annoncer une
/// taille nulle pour une ressource qui n'est pas vide.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn une_reponse_a_head_conserve_ses_entetes() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut b = [0u8; 8192];
                let _ = sock.read(&mut b).await;
                // Réponse à HEAD : en-têtes décrivant la ressource, corps vide.
                let head = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\
                            Content-Length: 12345\r\nConnection: close\r\n\r\n";
                let _ = sock.write_all(head.as_bytes()).await;
            });
        }
    });

    let (proxy, _stop) = start_proxy().await;
    let resp = client(proxy)
        .head(format!("http://{origin_addr}/gros.html"))
        .send()
        .await
        .expect("requête relayée");

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok()),
        Some("12345"),
        "HEAD doit rapporter la taille réelle de la ressource"
    );
}
