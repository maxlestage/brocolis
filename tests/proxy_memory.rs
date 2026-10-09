//! Mesure la mémoire consommée par le proxy pour relayer un gros corps NON
//! textuel (un binaire, typiquement un téléchargement).
//!
//! Test exploratoire : il ne fige pas de seuil, il imprime la consommation.
//! Il sert à établir si le proxy diffuse le corps au fil de l'eau ou s'il le
//! bufferise intégralement en mémoire.

use std::net::SocketAddr;
use std::sync::Arc;

use brocolis::{build_app_state, config::Config};

/// Occupation mémoire résidente du processus, en octets.
fn rss_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    pages * 4096
}

/// Serveur d'origine minimal : répond `size` octets en application/octet-stream.
async fn origin(size: usize) -> SocketAddr {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let _ = sock.read(&mut buf).await;
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n\
                     Content-Length: {size}\r\nConnection: close\r\n\r\n"
                );
                if sock.write_all(head.as_bytes()).await.is_err() {
                    return;
                }
                let chunk = vec![0x42u8; 64 * 1024];
                let mut sent = 0;
                while sent < size {
                    let n = chunk.len().min(size - sent);
                    if sock.write_all(&chunk[..n]).await.is_err() {
                        return;
                    }
                    sent += n;
                }
                let _ = sock.flush().await;
            });
        }
    });
    addr
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn gros_corps_binaire_nest_pas_bufferise_en_memoire() {
    const SIZE: usize = 256 * 1024 * 1024; // 256 Mio

    let origin_addr = origin(SIZE).await;

    let dir = std::env::temp_dir().join(format!("brocolis-mem-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = Config {
        ca_cert: dir.join("ca.crt"),
        ca_key: dir.join("ca.key"),
        ..Config::default()
    };

    let (state, _keys) = build_app_state(cfg).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    drop(listener);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let st: Arc<_> = state.clone();
    tokio::spawn(async move {
        let _ = brocolis::run_until(st, proxy_addr, async {
            let _ = rx.await;
        })
        .await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let before = rss_bytes();

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(format!("http://{proxy_addr}")).unwrap())
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    let resp = client
        .get(format!("http://{origin_addr}/gros.bin"))
        .send()
        .await
        .expect("la requête doit aboutir");
    assert_eq!(resp.status(), 200);

    // Pic mesuré pendant que le corps transite.
    let peak = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let p2 = peak.clone();
    let watcher = tokio::spawn(async move {
        for _ in 0..2000 {
            let r = rss_bytes();
            p2.fetch_max(r, std::sync::atomic::Ordering::Relaxed);
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    });

    // On consomme le flux morceau par morceau SANS l'accumuler : sinon c'est le
    // client de test qui occuperait la mémoire et la mesure ne dirait plus rien
    // sur le proxy.
    let mut received = 0usize;
    {
        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            received += chunk.expect("morceau lisible").len();
        }
    }
    watcher.abort();
    assert_eq!(received, SIZE, "le corps doit arriver entier");

    let peak = peak
        .load(std::sync::atomic::Ordering::Relaxed)
        .max(rss_bytes());
    let growth = peak.saturating_sub(before);

    eprintln!("--- transfert de {} Mio ---", SIZE / 1024 / 1024);
    eprintln!("RSS avant : {} Mio", before / 1024 / 1024);
    eprintln!("RSS pic   : {} Mio", peak / 1024 / 1024);
    eprintln!("croissance: {} Mio", growth / 1024 / 1024);

    let _ = tx.send(());
    let _ = std::fs::remove_dir_all(&dir);

    // Le client consomme le flux sans l'accumuler : toute croissance notable
    // vient donc du proxy. Un relais au fil de l'eau doit rester très en
    // dessous de la taille transférée ; un buffer complet s'en approcherait.
    assert!(
        growth < (SIZE as u64) / 4,
        "le proxy semble bufferiser le corps en mémoire : croissance de {} Mio \
         pour un transfert de {} Mio (attendu : bien moins de {} Mio)",
        growth / 1024 / 1024,
        SIZE / 1024 / 1024,
        SIZE / 4 / 1024 / 1024
    );
}
