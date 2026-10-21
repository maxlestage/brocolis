//! Appairage complet en ligne de commande, contre un faux serveur de
//! rendez-vous tenu ici même.
//!
//! `pair` et `pair-group` n'étaient couverts par rien : ils ne fonctionnent
//! qu'à deux processus, ce qui les rendait invisibles aux tests unitaires. Le
//! serveur ci-dessous implémente le protocole documenté en tête de
//! `src/pairing.rs`, ce qui permet de faire réellement dialoguer deux
//! `brocolis` et de vérifier qu'ils aboutissent à LA MÊME clé.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_brocolis")
}

fn tmpdir(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("brocolis-pair-{}-{nom}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

type Pair = (String, String, String, String);

#[derive(Default)]
struct Etat {
    /// code -> [(peerId, pub, paquet v3 ou "", capacité v4 ou "")]
    pairs: HashMap<String, Vec<Pair>>,
    /// code -> [(destinataire, expéditeur, sonPub, blob)]
    wraps: Vec<(String, String, String, String, String)>,
}

/// Faux serveur de rendez-vous, conforme au protocole de `src/pairing.rs`.
fn rendezvous() -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let etat = Arc::new(Mutex::new(Etat::default()));

    std::thread::spawn(move || {
        for flux in l.incoming() {
            let Ok(flux) = flux else { continue };
            let etat = etat.clone();
            std::thread::spawn(move || servir(flux, etat));
        }
    });
    format!("http://{addr}")
}

fn servir(mut flux: TcpStream, etat: Arc<Mutex<Etat>>) {
    let mut lecteur = BufReader::new(flux.try_clone().unwrap());
    let mut ligne = String::new();
    if lecteur.read_line(&mut ligne).is_err() {
        return;
    }
    let mut morceaux = ligne.split_whitespace();
    let methode = morceaux.next().unwrap_or("").to_string();
    let cible = morceaux.next().unwrap_or("/").to_string();

    // En-têtes : on ne retient que la longueur du corps.
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        if lecteur.read_line(&mut h).is_err() || h.trim().is_empty() {
            break;
        }
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
    }
    let mut corps = vec![0u8; len];
    if len > 0 && lecteur.read_exact(&mut corps).is_err() {
        return;
    }
    let corps = String::from_utf8_lossy(&corps).into_owned();

    let (chemin, requete) = cible.split_once('?').unwrap_or((cible.as_str(), ""));
    let params: HashMap<&str, String> = requete
        .split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k, urldecode(v)))
        .collect();

    if std::env::var("PAIR_DEBUG").is_ok() {
        eprintln!("[srv] {methode} {chemin} q={requete} corps={corps}");
    }
    let json = match (methode.as_str(), chemin) {
        ("POST", "/pair") => {
            let code = champ(&corps, "code");
            let moi = champ(&corps, "peerId");
            let pubk = champ(&corps, "pub");
            let paquet = champ(&corps, "bundle");
            // Un code « V3… » joue un serveur d'avant le v4 : il ne relaie
            // pas la capacité.
            let caps = if code.starts_with("V3") {
                String::new()
            } else {
                champ(&corps, "caps")
            };
            let mut e = etat.lock().unwrap();
            let liste = e.pairs.entry(code.clone()).or_default();
            if !liste.iter().any(|(p, _, _, _)| *p == moi) {
                liste.push((moi.clone(), pubk, paquet, caps));
            }
            pairs_json(liste, &moi)
        }
        ("GET", "/pair") => {
            let code = params.get("code").cloned().unwrap_or_default();
            let moi = params.get("peerId").cloned().unwrap_or_default();
            let e = etat.lock().unwrap();
            let vide = Vec::new();
            pairs_json(e.pairs.get(&code).unwrap_or(&vide), &moi)
        }
        ("POST", "/wrap") => {
            let code = champ(&corps, "code");
            let de = champ(&corps, "fromPeerId");
            let de_pub = champ(&corps, "fromPub");
            let mut e = etat.lock().unwrap();
            // Les objets de `items` portent `toPeerId` et `blob` dans un
            // ordre non garanti : on les découpe d'abord, puis on lit chaque
            // champ par son nom. (Un parseur qui supposait l'ordre ratait
            // silencieusement tous les dépôts.)
            for item in items_bruts(&corps) {
                let vers = champ(&item, "toPeerId");
                let blob = champ(&item, "blob");
                if !vers.is_empty() && !blob.is_empty() {
                    e.wraps
                        .push((code.clone(), vers, de.clone(), de_pub.clone(), blob));
                }
            }
            "{\"ok\":true,\"count\":1}".to_string()
        }
        ("GET", "/wrap") => {
            let code = params.get("code").cloned().unwrap_or_default();
            let moi = params.get("peerId").cloned().unwrap_or_default();
            let e = etat.lock().unwrap();
            match e
                .wraps
                .iter()
                .find(|(c, vers, _, _, _)| *c == code && *vers == moi)
            {
                Some((_, _, de, de_pub, blob)) => format!(
                    "{{\"wrap\":{{\"from\":\"{de}\",\"fromPub\":\"{de_pub}\",\"blob\":\"{blob}\"}}}}"
                ),
                None => "{\"wrap\":null}".to_string(),
            }
        }
        _ => "{}".to_string(),
    };

    let reponse = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
        json.len()
    );
    let _ = flux.write_all(reponse.as_bytes());
}

/// Découpe le tableau `items` en objets, sans rien supposer de l'ordre des
/// champs.
fn items_bruts(corps: &str) -> Vec<String> {
    let Some(i) = corps.find("\"items\":[") else {
        return Vec::new();
    };
    let reste = &corps[i + "\"items\":[".len()..];
    let fin = reste.find(']').unwrap_or(reste.len());
    reste[..fin]
        .split("},")
        .map(|o| {
            o.trim_start_matches(['{', ','])
                .trim_end_matches('}')
                .to_string()
        })
        .filter(|o| !o.is_empty())
        .collect()
}

/// Les correspondants renvoyés excluent toujours le demandeur.
/// Le paquet v3 (et la capacité v4) n'est rendu que pour qui en a publié
/// un, comme le serveur.
fn pairs_json(liste: &[Pair], moi: &str) -> String {
    let items: Vec<String> = liste
        .iter()
        .filter(|(p, _, _, _)| p != moi)
        .map(|(p, k, b, c)| {
            let mut o = format!("{{\"peerId\":\"{p}\",\"pub\":\"{k}\"");
            if !b.is_empty() {
                o.push_str(&format!(",\"bundle\":\"{b}\""));
            }
            if !c.is_empty() {
                o.push_str(&format!(",\"caps\":\"{c}\""));
            }
            o + "}"
        })
        .collect();
    format!("{{\"peers\":[{}]}}", items.join(","))
}

/// Valeur d'un champ `"nom":"valeur"` dans un JSON plat.
fn champ(json: &str, nom: &str) -> String {
    let motif = format!("\"{nom}\":");
    match json.find(&motif) {
        Some(i) => entre_guillemets(&json[i + motif.len()..]),
        None => String::new(),
    }
}

fn entre_guillemets(s: &str) -> String {
    let s = match s.find('"') {
        Some(i) => &s[i + 1..],
        None => return String::new(),
    };
    match s.find('"') {
        Some(j) => s[..j].to_string(),
        None => String::new(),
    }
}

fn urldecode(s: &str) -> String {
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
                let h = (b[i + 1] as char).to_digit(16).unwrap_or(0);
                let l = (b[i + 2] as char).to_digit(16).unwrap_or(0);
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Clés du champ `keys` d'un `brocolis.toml`.
fn cles_du_config(p: &std::path::Path) -> Vec<String> {
    let s = std::fs::read_to_string(p).unwrap_or_default();
    s.lines()
        .filter(|l| l.contains('"'))
        .filter_map(|l| {
            let d = l.find('"')? + 1;
            let f = l[d..].find('"')? + d;
            let v = &l[d..f];
            // On ne retient que ce qui ressemble à une clé base64 de 32 octets.
            (v.len() == 44 && v.ends_with('=')).then(|| v.to_string())
        })
        .collect()
}

/// Deux `brocolis pair` sur le même code doivent aboutir à la MÊME clé, et
/// chacun l'écrire dans sa propre configuration.
#[test]
fn deux_correspondants_derivent_la_meme_cle() {
    let serveur = rendezvous();
    let a = tmpdir("alice");
    let b = tmpdir("bob");

    let lance = |d: &PathBuf| {
        Command::new(bin())
            .args(["pair", "--code", "CODE-COMMUN", "--server", &serveur])
            .arg("--config")
            .arg(d.join("brocolis.toml"))
            .arg("--identity")
            .arg(d.join("id.key"))
            .args(["--timeout", "30"])
            .current_dir(d)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };

    // Les deux doivent tourner en parallèle : chacun attend l'autre.
    let pa = lance(&a);
    std::thread::sleep(std::time::Duration::from_millis(400));
    let pb = lance(&b);

    let ra = pa.wait_with_output().unwrap();
    let rb = pb.wait_with_output().unwrap();
    assert!(
        ra.status.success(),
        "alice: {}",
        String::from_utf8_lossy(&ra.stderr)
    );
    assert!(
        rb.status.success(),
        "bob: {}",
        String::from_utf8_lossy(&rb.stderr)
    );

    let ka = cles_du_config(&a.join("brocolis.toml"));
    let kb = cles_du_config(&b.join("brocolis.toml"));
    assert_eq!(ka.len(), 1, "alice doit avoir exactement une clé : {ka:?}");
    assert_eq!(kb.len(), 1, "bob doit avoir exactement une clé : {kb:?}");
    assert_eq!(ka, kb, "les deux doivent dériver la MÊME clé partagée");

    // L'empreinte affichée doit être identique des deux côtés : c'est elle que
    // les correspondants comparent de vive voix.
    let emp = |o: &std::process::Output| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find(|l| l.contains("Empreinte"))
            .map(|l| l.trim().to_string())
            .unwrap_or_default()
    };
    assert_eq!(emp(&ra), emp(&rb), "l'empreinte doit correspondre");
    assert!(!emp(&ra).is_empty(), "une empreinte doit être affichée");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// La clé dérivée doit être réellement exploitable : ce que l'un chiffre,
/// l'autre doit pouvoir le lire.
#[test]
fn la_cle_appairee_permet_de_se_lire() {
    let serveur = rendezvous();
    let a = tmpdir("chiffre-a");
    let b = tmpdir("chiffre-b");

    let lance = |d: &PathBuf| {
        Command::new(bin())
            .args(["pair", "--code", "LECTURE", "--server", &serveur])
            .arg("--config")
            .arg(d.join("brocolis.toml"))
            .arg("--identity")
            .arg(d.join("id.key"))
            .args(["--timeout", "30"])
            .current_dir(d)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let mut pa = lance(&a);
    std::thread::sleep(std::time::Duration::from_millis(400));
    let mut pb = lance(&b);
    assert!(pa.wait().unwrap().success());
    assert!(pb.wait().unwrap().success());

    // Alice chiffre avec SA configuration…
    let chiffre = Command::new(bin())
        .args(["encrypt", "--text", "rdv ((secret partage)) ok", "--config"])
        .arg(a.join("brocolis.toml"))
        .output()
        .unwrap();
    assert!(chiffre.status.success());
    let chiffre = String::from_utf8(chiffre.stdout).unwrap();
    assert!(
        !chiffre.contains("secret partage"),
        "pas de fuite : {chiffre}"
    );

    // …et Bob doit pouvoir le lire avec la sienne.
    let clair = Command::new(bin())
        .args(["decrypt", "--text", chiffre.trim(), "--config"])
        .arg(b.join("brocolis.toml"))
        .output()
        .unwrap();
    assert!(clair.status.success());
    assert_eq!(
        String::from_utf8(clair.stdout).unwrap().trim(),
        "rdv secret partage ok",
        "bob doit lire ce qu'alice a chiffré"
    );

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// Deux `pair --name` : chacun devient le contact de l'autre avec son paquet
/// de clés v3, l'empreinte (qui couvre la clé ET les deux identités) est la
/// même des deux côtés, et ce qu'Alice chiffre pour Bob part en double
/// cliquet — Bob le lit, et sait que ça vient d'Alice.
#[test]
fn deux_contacts_appaires_parlent_en_double_cliquet() {
    duo_parle("V3", false);
}

/// Avec un serveur qui relaie la capacité v4 (§9.6) : triple cliquet, et une
/// empreinte v4 identique des deux côtés.
#[test]
fn deux_contacts_appaires_parlent_en_triple_cliquet() {
    duo_parle("V4", true);
}

fn duo_parle(code: &str, v4: bool) {
    let serveur = rendezvous();
    let a = tmpdir(&format!("{code}-alice"));
    let b = tmpdir(&format!("{code}-bob"));
    let (cliquet, tete) = if v4 {
        ("triple cliquet", "[[BROCOLIS:v4:")
    } else {
        ("double cliquet", "[[BROCOLIS:v3:")
    };

    let lance = |d: &PathBuf, nom: &str| {
        Command::new(bin())
            .args(["pair", "--code", code, "--name", nom, "--server", &serveur])
            .arg("--config")
            .arg(d.join("brocolis.toml"))
            .arg("--identity")
            .arg(d.join("id.key"))
            .args(["--timeout", "30"])
            .current_dir(d)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let pa = lance(&a, "Bob");
    std::thread::sleep(std::time::Duration::from_millis(400));
    let pb = lance(&b, "Alice");
    let ra = pa.wait_with_output().unwrap();
    let rb = pb.wait_with_output().unwrap();
    assert!(
        ra.status.success(),
        "{}",
        String::from_utf8_lossy(&ra.stderr)
    );
    assert!(
        rb.status.success(),
        "{}",
        String::from_utf8_lossy(&rb.stderr)
    );
    let emp = |o: &std::process::Output| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find(|l| l.contains("Empreinte"))
            .map(|l| l.trim().to_string())
            .unwrap_or_default()
    };
    assert_eq!(emp(&ra), emp(&rb), "la même empreinte des deux côtés");
    let config_a = std::fs::read_to_string(a.join("brocolis.toml")).unwrap();
    assert!(
        config_a.contains("name = \"Bob\"") && config_a.contains("bundle = "),
        "{config_a}"
    );
    assert!(a.join("v3").join("appareil.json").exists());

    // La liste des contacts dit ce que donnera le prochain envoi.
    let liste = Command::new(bin())
        .arg("contacts")
        .arg("--config")
        .arg(a.join("brocolis.toml"))
        .output()
        .unwrap();
    let liste = String::from_utf8(liste.stdout).unwrap();
    assert!(
        liste.contains(&format!("{cliquet} (post-quantique")),
        "{liste}"
    );
    assert_eq!(config_a.contains("caps = "), v4, "{config_a}");

    let chiffre = Command::new(bin())
        .args([
            "encrypt",
            "--text",
            "rdv ((au café)) ok",
            "--to",
            "Bob",
            "--config",
        ])
        .arg(a.join("brocolis.toml"))
        .output()
        .unwrap();
    assert!(chiffre.status.success());
    let chiffre = String::from_utf8(chiffre.stdout).unwrap();
    assert!(chiffre.contains(tete), "{chiffre}");
    assert!(!chiffre.contains("au café"));

    let lire = |d: &PathBuf| {
        Command::new(bin())
            .args(["decrypt", "--text", chiffre.trim(), "--config"])
            .arg(d.join("brocolis.toml"))
            .output()
            .unwrap()
    };
    let clair = lire(&b);
    assert!(clair.status.success());
    assert_eq!(
        String::from_utf8_lossy(&clair.stdout).trim(),
        "rdv au café ok"
    );
    assert!(
        String::from_utf8_lossy(&clair.stderr).contains(&format!("{cliquet}, de : Alice")),
        "{}",
        String::from_utf8_lossy(&clair.stderr)
    );
    // Relu : le clair est gardé (la clé de message, elle, est effacée).
    assert_eq!(
        String::from_utf8_lossy(&lire(&b).stdout).trim(),
        "rdv au café ok"
    );
    // Alice relit son propre envoi.
    assert_eq!(
        String::from_utf8_lossy(&lire(&a).stdout).trim(),
        "rdv au café ok"
    );

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// Un correspondant sans paquet v3 (un ancien client) : l'appairage reste en
/// ancien format, avec l'empreinte d'avant des deux côtés.
#[test]
fn un_ancien_client_reste_en_ancien_format() {
    let serveur = rendezvous();
    let a = tmpdir("v3-nouveau");
    let b = tmpdir("v3-ancien");
    let lance = |d: &PathBuf, nom: Option<&str>| {
        let mut c = Command::new(bin());
        c.args(["pair", "--code", "MIXTE", "--server", &serveur]);
        if let Some(n) = nom {
            c.args(["--name", n]);
        }
        c.arg("--config")
            .arg(d.join("brocolis.toml"))
            .arg("--identity")
            .arg(d.join("id.key"))
            .args(["--timeout", "30"])
            .current_dir(d)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let pa = lance(&a, Some("Ancien"));
    std::thread::sleep(std::time::Duration::from_millis(400));
    let pb = lance(&b, None);
    let ra = pa.wait_with_output().unwrap();
    let rb = pb.wait_with_output().unwrap();
    assert!(ra.status.success() && rb.status.success());
    let emp = |o: &std::process::Output| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find(|l| l.contains("Empreinte"))
            .map(|l| l.trim().to_string())
            .unwrap_or_default()
    };
    assert_eq!(emp(&ra), emp(&rb));
    let config_a = std::fs::read_to_string(a.join("brocolis.toml")).unwrap();
    assert!(config_a.contains("name = \"Ancien\""));
    assert!(!config_a.contains("bundle"), "{config_a}");
    assert!(String::from_utf8_lossy(&ra.stdout).contains("ancien format"));
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// Sans correspondant, la commande doit abandonner dans le délai annoncé
/// plutôt que d'attendre indéfiniment.
#[test]
fn pair_abandonne_au_bout_du_delai() {
    let serveur = rendezvous();
    let d = tmpdir("solitaire");

    let debut = std::time::Instant::now();
    let out = Command::new(bin())
        .args([
            "pair",
            "--code",
            "PERSONNE",
            "--server",
            &serveur,
            "--timeout",
            "3",
        ])
        .arg("--config")
        .arg(d.join("brocolis.toml"))
        .arg("--identity")
        .arg(d.join("id.key"))
        .current_dir(&d)
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap();
    let ecoule = debut.elapsed();

    assert!(!out.status.success(), "doit échouer faute de correspondant");
    assert!(
        ecoule < std::time::Duration::from_secs(25),
        "doit abandonner près du délai demandé, pas bien après ({ecoule:?})"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("correspondant"),
        "le message doit expliquer l'attente vaine : {err}"
    );
    // Rien ne doit avoir été ajouté à la configuration.
    assert!(
        cles_du_config(&d.join("brocolis.toml")).is_empty(),
        "aucune clé ne doit être écrite en cas d'échec"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// L'identité X25519 doit être RÉUTILISÉE d'un appairage à l'autre : la
/// régénérer changerait l'empreinte que les correspondants ont comparée.
#[test]
fn lidentite_est_reutilisee_entre_deux_appairages() {
    let d = tmpdir("identite");
    let serveur = rendezvous();
    let id = d.join("id.key");

    // Un premier appel, même voué à l'échec, crée l'identité.
    let _ = Command::new(bin())
        .args([
            "pair",
            "--code",
            "A",
            "--server",
            &serveur,
            "--timeout",
            "1",
        ])
        .arg("--config")
        .arg(d.join("brocolis.toml"))
        .arg("--identity")
        .arg(&id)
        .current_dir(&d)
        .output()
        .unwrap();
    let premiere = std::fs::read_to_string(&id).expect("identité créée");

    let _ = Command::new(bin())
        .args([
            "pair",
            "--code",
            "B",
            "--server",
            &serveur,
            "--timeout",
            "1",
        ])
        .arg("--config")
        .arg(d.join("brocolis.toml"))
        .arg("--identity")
        .arg(&id)
        .current_dir(&d)
        .output()
        .unwrap();
    let seconde = std::fs::read_to_string(&id).unwrap();

    assert_eq!(premiere, seconde, "l'identité ne doit pas être régénérée");
    let _ = std::fs::remove_dir_all(&d);
}

/// Appairage de GROUPE : le fondateur distribue la clé, le membre la récupère.
/// Les deux doivent finir avec la même clé de groupe.
#[test]
fn un_groupe_partage_la_meme_cle() {
    let serveur = rendezvous();
    let f = tmpdir("fondateur");
    let m = tmpdir("membre");

    let pf = Command::new(bin())
        .args([
            "pair-group",
            "--code",
            "GROUPE",
            "--found",
            "--server",
            &serveur,
        ])
        .arg("--config")
        .arg(f.join("brocolis.toml"))
        .arg("--identity")
        .arg(f.join("id.key"))
        .args(["--timeout", "30"])
        .current_dir(&f)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));
    let pm = Command::new(bin())
        .args(["pair-group", "--code", "GROUPE", "--server", &serveur])
        .arg("--config")
        .arg(m.join("brocolis.toml"))
        .arg("--identity")
        .arg(m.join("id.key"))
        .args(["--timeout", "30"])
        .current_dir(&m)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let rm = pm.wait_with_output().unwrap();
    let rf = pf.wait_with_output().unwrap();

    assert!(
        rm.status.success(),
        "le membre doit aboutir.\n--- membre ---\n{}\n--- fondateur ---\n{}\n{}",
        String::from_utf8_lossy(&rm.stderr),
        String::from_utf8_lossy(&rf.stdout),
        String::from_utf8_lossy(&rf.stderr)
    );

    let kf = cles_du_config(&f.join("brocolis.toml"));
    let km = cles_du_config(&m.join("brocolis.toml"));
    assert_eq!(kf.len(), 1, "le fondateur a la clé de groupe : {kf:?}");
    assert_eq!(km.len(), 1, "le membre a reçu la clé de groupe : {km:?}");
    assert_eq!(kf, km, "toute la clé de groupe doit être identique");

    let _ = std::fs::remove_dir_all(&f);
    let _ = std::fs::remove_dir_all(&m);
}
