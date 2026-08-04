//! Tests jetables de vérification de bugs (à supprimer).

use std::net::SocketAddr;
use std::sync::Arc;

use brocolis::{build_app_state, config::Config};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn rss_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    pages * 4096
}

/// Démarre un proxy brocolis sur un port libre, renvoie son adresse.
async fn start_proxy(tag: &str) -> SocketAddr {
    let dir = std::env::temp_dir().join(format!("brocolis-bug-{}-{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut cfg = Config::default();
    cfg.ca_cert = dir.join("ca.crt");
    cfg.ca_key = dir.join("ca.key");
    let (state, _keys) = build_app_state(cfg).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    tokio::spawn(async move {
        let _ = brocolis::run_until(state, addr, std::future::pending::<()>()).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    addr
}

fn client_via(proxy: SocketAddr) -> reqwest::Client {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{proxy}")).unwrap())
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap()
}

/// Serveur d'origine générique : `respond` écrit la réponse brute.
async fn raw_origin<F, Fut>(respond: F) -> SocketAddr
where
    F: Fn(tokio::net::TcpStream) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let respond = Arc::new(respond);
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let respond = respond.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let _ = sock.read(&mut buf).await;
                respond(sock).await;
            });
        }
    });
    addr
}

// ---------------------------------------------------------------------------
// BUG A : réponse textuelle en chunked (sans Content-Length) -> bufferisée
//         intégralement, MAX_REWRITE_BYTES (8 Mio) n'est jamais appliqué.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bug_a_chunked_textuel_bufferise_sans_plafond() {
    const SIZE: usize = 256 * 1024 * 1024; // 256 Mio de text/html en chunked

    let origin = raw_origin(|mut sock| async move {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
                    Transfer-Encoding: chunked\r\n\r\n";
        if sock.write_all(head.as_bytes()).await.is_err() {
            return;
        }
        let payload = vec![b'a'; 64 * 1024];
        let mut sent = 0usize;
        while sent < SIZE {
            let n = payload.len().min(SIZE - sent);
            let chunk_hdr = format!("{n:x}\r\n");
            if sock.write_all(chunk_hdr.as_bytes()).await.is_err() {
                return;
            }
            if sock.write_all(&payload[..n]).await.is_err() {
                return;
            }
            if sock.write_all(b"\r\n").await.is_err() {
                return;
            }
            sent += n;
        }
        let _ = sock.write_all(b"0\r\n\r\n").await;
        let _ = sock.flush().await;
    })
    .await;

    let proxy = start_proxy("chunked").await;
    let client = client_via(proxy);

    let before = rss_bytes();
    let peak = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let p2 = peak.clone();
    let watcher = tokio::spawn(async move {
        loop {
            p2.fetch_max(rss_bytes(), std::sync::atomic::Ordering::Relaxed);
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    });

    let resp = client
        .get(format!("http://{origin}/page.html"))
        .send()
        .await
        .expect("requête aboutie");
    assert_eq!(resp.status(), 200);
    let mut received = 0usize;
    {
        use futures_util::StreamExt;
        let mut s = resp.bytes_stream();
        while let Some(c) = s.next().await {
            received += c.expect("morceau").len();
        }
    }
    watcher.abort();
    let peak = peak.load(std::sync::atomic::Ordering::Relaxed).max(rss_bytes());
    let growth = peak.saturating_sub(before);
    eprintln!(
        "BUG A: reçu {} Mio ; RSS avant {} Mio, pic {} Mio, croissance {} Mio",
        received / 1024 / 1024,
        before / 1024 / 1024,
        peak / 1024 / 1024,
        growth / 1024 / 1024
    );
    assert!(
        growth < (SIZE as u64) / 4,
        "MAX_REWRITE_BYTES contourné : croissance {} Mio pour {} Mio transférés",
        growth / 1024 / 1024,
        SIZE / 1024 / 1024
    );
}

// ---------------------------------------------------------------------------
// BUG B : coupure amont en plein corps -> 200 + corps tronqué SANS erreur.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bug_b_troncature_silencieuse() {
    let origin = raw_origin(|mut sock| async move {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n\
                    Content-Length: 1000000\r\n\r\n";
        let _ = sock.write_all(head.as_bytes()).await;
        let _ = sock.write_all(&vec![0x42u8; 1000]).await;
        let _ = sock.flush().await;
        // Coupe brutalement : 1000 octets envoyés sur 1000000 annoncés.
        drop(sock);
    })
    .await;

    let proxy = start_proxy("trunc").await;
    let client = client_via(proxy);

    let resp = client
        .get(format!("http://{origin}/gros.bin"))
        .send()
        .await
        .expect("requête aboutie");
    let status = resp.status();
    let cl = resp
        .headers()
        .get("content-length")
        .map(|v| v.to_str().unwrap().to_string());
    let body = resp.bytes().await;
    eprintln!(
        "BUG B: status={status} content-length={cl:?} body={:?}",
        body.as_ref().map(|b| b.len())
    );
    let body = body.expect("le client n'a vu AUCUNE erreur");
    assert!(
        !(status == 200 && body.len() == 1000),
        "troncature silencieuse : 200 OK avec 1000 octets sur 1000000 annoncés"
    );
}

// ---------------------------------------------------------------------------
// BUG C : Content-Encoding retiré de la réponse alors que le corps compressé
//         est relayé tel quel -> le client reçoit du gzip étiqueté text/html.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bug_c_content_encoding_retire_mais_corps_compresse() {
    // Corps « gzip » factice : seuls les octets comptent pour la démonstration.
    const GZ: &[u8] = &[
        0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xcb, 0x48, 0xcd, 0xc9, 0xc9,
        0x07, 0x00, 0x86, 0xa6, 0x10, 0x36, 0x05, 0x00, 0x00, 0x00,
    ];
    let origin = raw_origin(|mut sock| async move {
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Encoding: gzip\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n",
            GZ.len()
        );
        let _ = sock.write_all(head.as_bytes()).await;
        let _ = sock.write_all(GZ).await;
        let _ = sock.flush().await;
    })
    .await;

    let proxy = start_proxy("gzip").await;
    let client = client_via(proxy);

    let resp = client
        .get(format!("http://{origin}/p.html"))
        .send()
        .await
        .expect("requête aboutie");
    let ce = resp.headers().get("content-encoding").cloned();
    let ct = resp.headers().get("content-type").cloned();
    let body = resp.bytes().await.unwrap();
    eprintln!("BUG C: content-type={ct:?} content-encoding={ce:?} body={body:?}");
    assert!(
        !(ce.is_none() && body.as_ref() == GZ),
        "corps gzip relayé tel quel mais en-tête Content-Encoding supprimé"
    );
}

// ---------------------------------------------------------------------------
// BUG D : HEAD -> Content-Length écrasé à 0.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bug_d_head_content_length() {
    let origin = raw_origin(|mut sock| async move {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 123456\r\n\r\n";
        let _ = sock.write_all(head.as_bytes()).await;
        let _ = sock.flush().await;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    })
    .await;

    let proxy = start_proxy("head").await;
    let client = client_via(proxy);

    let resp = client
        .head(format!("http://{origin}/gros.html"))
        .send()
        .await
        .expect("requête aboutie");
    let cl = resp
        .headers()
        .get("content-length")
        .map(|v| v.to_str().unwrap().to_string());
    eprintln!("BUG D: status={} content-length={cl:?}", resp.status());
    assert_eq!(
        cl.as_deref(),
        Some("123456"),
        "HEAD : Content-Length amont perdu/écrasé"
    );
}
