//! Tests de la CLI, exécutée comme un utilisateur l'exécute.
//!
//! Ils manquaient entièrement : rien ne vérifiait le contrat de sortie des
//! commandes (ce qui part sur stdout, les codes de retour) ni les droits des
//! fichiers de clés produits.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_brocolis")
}

/// Répertoire de travail isolé pour un test.
fn tmpdir(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("brocolis-cli-{}-{nom}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[cfg(unix)]
fn mode(p: &std::path::Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).unwrap().permissions().mode() & 0o777
}

/// `keygen` doit mettre la CLÉ SEULE sur stdout.
///
/// Les conseils d'usage étaient imprimés sur stdout avec elle : `cle=$(brocolis
/// keygen)` capturait donc trois lignes, et `brocolis keygen > cle.txt`
/// produisait un fichier inutilisable tel quel.
#[test]
fn keygen_ne_met_que_la_cle_sur_stdout() {
    let out = Command::new(bin()).arg("keygen").output().unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let lignes: Vec<&str> = stdout.lines().collect();
    assert_eq!(lignes.len(), 1, "stdout doit contenir la clé et rien d'autre : {stdout:?}");

    // La clé doit être directement exploitable.
    let cle = lignes[0];
    let brute = base64_decode(cle).expect("stdout doit être du base64 valide");
    assert_eq!(brute.len(), 32, "une clé AES-256 fait 32 octets");

    // Les conseils restent affichés, mais sur stderr.
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("Partagez cette clé"),
        "les conseils doivent rester visibles, sur stderr : {stderr:?}"
    );
}

/// Deux appels ne doivent jamais rendre la même clé.
#[test]
fn keygen_produit_une_cle_differente_a_chaque_appel() {
    let a = Command::new(bin()).arg("keygen").output().unwrap().stdout;
    let b = Command::new(bin()).arg("keygen").output().unwrap().stdout;
    assert_ne!(a, b);
}

/// La clé privée du CA ne doit jamais être lisible par les autres comptes.
///
/// Elle signe des certificats pour N'IMPORTE QUEL site que le navigateur de
/// l'utilisateur croira ensuite légitime : la laisser en 0644 permettait à tout
/// autre compte du poste de forger ces certificats. Elle était écrite en 0644,
/// alors que la clé d'appairage X25519, elle, était déjà correctement en 0600.
#[cfg(unix)]
#[test]
fn la_cle_privee_du_ca_nest_pas_lisible_par_les_autres() {
    let d = tmpdir("ca-perms");
    let cert = d.join("ca.crt");
    let key = d.join("ca.key");

    let st = Command::new(bin())
        .args(["ca", "--ca-cert"])
        .arg(&cert)
        .arg("--ca-key")
        .arg(&key)
        .current_dir(&d)
        .status()
        .unwrap();
    assert!(st.success(), "`brocolis ca` doit réussir");

    assert_eq!(mode(&key), 0o600, "la clé privée du CA doit être en 0600");
    // Le certificat, lui, est public : il doit rester lisible pour être
    // installé dans le magasin de confiance.
    assert_ne!(mode(&cert), 0o600, "le certificat racine doit rester lisible");

    let _ = std::fs::remove_dir_all(&d);
}

/// Même si le fichier préexiste avec des droits trop larges, il est resserré.
#[cfg(unix)]
#[test]
fn une_cle_preexistante_trop_ouverte_est_resserree() {
    use std::os::unix::fs::PermissionsExt;
    let d = tmpdir("ca-preexistant");
    let key = d.join("ca.key");
    std::fs::write(&key, "contenu bidon").unwrap();
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(mode(&key), 0o644);

    brocolis::ca::write_private_key(&key, "nouveau contenu").unwrap();

    assert_eq!(mode(&key), 0o600, "les droits doivent être resserrés");
    assert_eq!(std::fs::read_to_string(&key).unwrap(), "nouveau contenu");
    let _ = std::fs::remove_dir_all(&d);
}

/// Une erreur de configuration doit sortir en échec, pas en succès silencieux.
#[test]
fn une_cle_invalide_fait_echouer_la_commande() {
    let d = tmpdir("cle-invalide");
    let out = Command::new(bin())
        .args(["run", "--listen", "127.0.0.1:0", "--key", "pas-du-base64!!"])
        .current_dir(&d)
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap();

    assert!(!out.status.success(), "le code de retour doit signaler l'échec");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("clé"),
        "le message doit désigner la clé en cause : {err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Une sous-commande inconnue doit être refusée, pas ignorée.
#[test]
fn une_sous_commande_inconnue_est_refusee() {
    let out = Command::new(bin()).arg("nawak").output().unwrap();
    assert!(!out.status.success());
}

/// Décodage base64 minimal, pour éviter d'ajouter une dépendance de test.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let s = s.trim().trim_end_matches('=');
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::new();
    for c in s.bytes() {
        let v = T.iter().position(|&t| t == c)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Aller-retour complet en ligne de commande, sans navigateur ni proxy.
///
/// C'est ce qui permet de vérifier qu'une clé fonctionne : avant, il fallait
/// monter un proxy et un navigateur pour s'en assurer.
#[test]
fn encrypt_puis_decrypt_restitue_le_texte() {
    let d = tmpdir("aller-retour");
    let cle = String::from_utf8(
        Command::new(bin()).arg("keygen").output().unwrap().stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    let chiffre = Command::new(bin())
        .args(["encrypt", "--text", "rendez-vous ((code 4291)) a 18h", "--key", &cle])
        .current_dir(&d)
        .output()
        .unwrap();
    assert!(chiffre.status.success());
    let chiffre = String::from_utf8(chiffre.stdout).unwrap();

    assert!(
        !chiffre.contains("code 4291"),
        "le secret ne doit pas apparaître en clair : {chiffre}"
    );
    assert!(chiffre.contains("[[BROCOLIS:v1:"), "un jeton doit être produit");
    assert!(
        chiffre.contains("rendez-vous ") && chiffre.contains(" a 18h"),
        "le texte autour du marqueur est conservé : {chiffre}"
    );

    let clair = Command::new(bin())
        .args(["decrypt", "--text", chiffre.trim(), "--key", &cle])
        .current_dir(&d)
        .output()
        .unwrap();
    assert!(clair.status.success());
    assert_eq!(
        String::from_utf8(clair.stdout).unwrap().trim(),
        "rendez-vous code 4291 a 18h",
        "le clair doit être restitué"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Le texte peut arriver par l'entrée standard, pour s'enchaîner dans un tube.
#[test]
fn encrypt_lit_lentree_standard() {
    use std::io::Write;
    use std::process::Stdio;

    let d = tmpdir("stdin");
    let cle = String::from_utf8(Command::new(bin()).arg("keygen").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string();

    let mut fils = Command::new(bin())
        .args(["encrypt", "--key", &cle])
        .current_dir(&d)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    fils.stdin
        .as_mut()
        .unwrap()
        .write_all(b"debut ((secret)) fin")
        .unwrap();
    let out = fils.wait_with_output().unwrap();

    assert!(out.status.success());
    let sortie = String::from_utf8(out.stdout).unwrap();
    assert!(!sortie.contains("secret"), "pas de fuite : {sortie}");
    assert!(sortie.contains("[[BROCOLIS:"), "jeton attendu : {sortie}");
    let _ = std::fs::remove_dir_all(&d);
}

/// stdout ne doit porter QUE le texte : le compte de segments va sur stderr,
/// sinon la commande n'est pas utilisable dans un tube.
#[test]
fn encrypt_ne_pollue_pas_stdout_avec_le_compte() {
    let d = tmpdir("tube");
    let cle = String::from_utf8(Command::new(bin()).arg("keygen").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string();

    let out = Command::new(bin())
        .args(["encrypt", "--text", "a ((b)) c", "--key", &cle])
        .current_dir(&d)
        .output()
        .unwrap();

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "une seule ligne sur stdout : {stdout:?}");
    assert!(!stdout.contains("segment"), "le compte ne doit pas être sur stdout");
    assert!(
        String::from_utf8(out.stderr).unwrap().contains("segment"),
        "le compte doit rester visible, sur stderr"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Sans clé, la commande refuse plutôt que d'inventer une clé jetable :
/// chiffrer avec une clé que personne ne possède donnerait l'illusion d'avoir
/// protégé quelque chose.
#[test]
fn encrypt_sans_cle_refuse_au_lieu_dinventer() {
    let d = tmpdir("sans-cle");
    let out = Command::new(bin())
        .args(["encrypt", "--text", "a ((b)) c"])
        .current_dir(&d)
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap();

    assert!(!out.status.success(), "doit échouer");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("aucune clé"), "message explicite attendu : {err}");
    let _ = std::fs::remove_dir_all(&d);
}

/// Une clé qui n'est pas la bonne laisse le jeton intact plutôt que de rendre
/// n'importe quoi : c'est le comportement « message pour quelqu'un d'autre ».
#[test]
fn decrypt_avec_une_mauvaise_cle_laisse_le_jeton_intact() {
    let d = tmpdir("mauvaise-cle");
    let cle_a = String::from_utf8(Command::new(bin()).arg("keygen").output().unwrap().stdout)
        .unwrap().trim().to_string();
    let cle_b = String::from_utf8(Command::new(bin()).arg("keygen").output().unwrap().stdout)
        .unwrap().trim().to_string();

    let chiffre = String::from_utf8(
        Command::new(bin())
            .args(["encrypt", "--text", "x ((secret)) y", "--key", &cle_a])
            .current_dir(&d)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["decrypt", "--text", chiffre.trim(), "--key", &cle_b])
        .current_dir(&d)
        .output()
        .unwrap();

    let sortie = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "ce n'est pas une erreur, juste illisible");
    assert!(
        sortie.contains("[[BROCOLIS:"),
        "le jeton doit rester tel quel : {sortie}"
    );
    assert!(!sortie.contains("secret"), "et surtout ne rien révéler");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------------------
// `run` : le proxy lancé par la CLI chiffre-t-il réellement ?
// ---------------------------------------------------------------------------

/// Serveur d'origine minimal qui mémorise le corps reçu.
fn origine(recu: std::sync::Arc<std::sync::Mutex<Vec<String>>>) -> u16 {
    use std::io::{BufRead, BufReader, Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for flux in l.incoming() {
            let Ok(mut flux) = flux else { continue };
            let recu = recu.clone();
            std::thread::spawn(move || {
                let mut lect = BufReader::new(flux.try_clone().unwrap());
                let mut ligne = String::new();
                let _ = lect.read_line(&mut ligne);
                let mut len = 0usize;
                loop {
                    let mut h = String::new();
                    if lect.read_line(&mut h).is_err() || h.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut corps = vec![0u8; len];
                if len > 0 {
                    let _ = lect.read_exact(&mut corps);
                }
                recu.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&corps).into_owned());
                let _ = flux.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                );
            });
        }
    });
    port
}

fn port_libre() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

/// Le proxy démarré par `brocolis run` doit réellement chiffrer le trafic.
///
/// Les tests bout en bout existants passent par la bibliothèque ; celui-ci
/// vérifie le binaire tel que l'utilisateur le lance, avec sa configuration.
#[test]
fn run_demarre_un_proxy_qui_chiffre_vraiment() {
    let d = tmpdir("run-proxy");
    let recu = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let port_origine = origine(recu.clone());
    let port_proxy = port_libre();

    let cle = String::from_utf8(Command::new(bin()).arg("keygen").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string();

    let mut proxy = Command::new(bin())
        .args(["run", "--listen", &format!("127.0.0.1:{port_proxy}"), "--key", &cle])
        .current_dir(&d)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // Attendre que le proxy écoute.
    let mut pret = false;
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port_proxy)).is_ok() {
            pret = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(pret, "le proxy doit finir par écouter");

    // Requête AU TRAVERS du proxy, écrite à la main pour ne dépendre d'aucun
    // réglage d'environnement (une variable no_proxy contournerait le proxy et
    // ferait croire à tort que le chiffrement ne se fait pas).
    {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port_proxy)).unwrap();
        let corps = "rendez-vous ((code 4291)) a 18h";
        let req = format!(
            "POST http://127.0.0.1:{port_origine}/envoi HTTP/1.1\r\nHost: 127.0.0.1:{port_origine}\r\n\
             Content-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corps}",
            corps.len()
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut rep = String::new();
        let _ = s.read_to_string(&mut rep);
        assert!(rep.contains("200"), "le proxy doit relayer la réponse : {rep}");
    }

    let _ = proxy.kill();
    let _ = proxy.wait();
    let vu = recu.lock().unwrap().first().cloned().unwrap_or_default();

    assert!(
        !vu.contains("code 4291"),
        "le secret ne doit pas sortir en clair : {vu}"
    );
    assert!(vu.contains("[[BROCOLIS:v1:"), "un jeton doit remplacer le segment : {vu}");
    let _ = std::fs::remove_dir_all(&d);
}

/// `--encrypt-host` restreint la portée : un hôte hors périmètre n'est pas touché.
#[test]
fn run_respecte_la_portee_encrypt_host() {
    let d = tmpdir("run-portee");
    let recu = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let port_origine = origine(recu.clone());
    let port_proxy = port_libre();
    let cle = String::from_utf8(Command::new(bin()).arg("keygen").output().unwrap().stdout)
        .unwrap().trim().to_string();

    let mut proxy = Command::new(bin())
        .args([
            "run", "--listen", &format!("127.0.0.1:{port_proxy}"), "--key", &cle,
            // La portée ne couvre PAS 127.0.0.1.
            "--encrypt-host", "exemple.invalide",
        ])
        .current_dir(&d)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    let mut pret = false;
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port_proxy)).is_ok() {
            pret = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(pret, "le proxy doit finir par écouter");
    {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port_proxy)).unwrap();
        let corps = "hors ((perimetre)) donc intact";
        let req = format!(
            "POST http://127.0.0.1:{port_origine}/x HTTP/1.1\r\nHost: 127.0.0.1:{port_origine}\r\n\
             Content-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corps}",
            corps.len()
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut rep = String::new();
        let _ = s.read_to_string(&mut rep);
        assert!(rep.contains("200"), "le proxy doit relayer la réponse : {rep}");
    }
    let _ = proxy.kill();
    let _ = proxy.wait();

    let vu = recu.lock().unwrap().first().cloned().unwrap_or_default();
    assert_eq!(
        vu, "hors ((perimetre)) donc intact",
        "un hôte hors portée doit traverser sans modification"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------------------
// `ca`
// ---------------------------------------------------------------------------

/// Relancer `ca` doit RECHARGER le CA existant, pas en forger un nouveau :
/// le régénérer invaliderait le certificat déjà installé dans le navigateur.
#[test]
fn ca_relance_ne_regenere_pas_le_certificat() {
    let d = tmpdir("ca-stable");
    let cert = d.join("ca.crt");
    let key = d.join("ca.key");
    let lancer = || {
        Command::new(bin())
            .args(["ca", "--ca-cert"]).arg(&cert)
            .arg("--ca-key").arg(&key)
            .current_dir(&d)
            .output()
            .unwrap()
    };

    assert!(lancer().status.success());
    let cert1 = std::fs::read_to_string(&cert).unwrap();
    let key1 = std::fs::read_to_string(&key).unwrap();

    assert!(lancer().status.success());
    assert_eq!(cert1, std::fs::read_to_string(&cert).unwrap(), "le certificat doit être stable");
    assert_eq!(key1, std::fs::read_to_string(&key).unwrap(), "la clé doit être stable");
    let _ = std::fs::remove_dir_all(&d);
}

/// `--export` doit écrire exactement le certificat racine, prêt à installer.
#[test]
fn ca_export_ecrit_le_meme_certificat() {
    let d = tmpdir("ca-export");
    let cert = d.join("ca.crt");
    let exporte = d.join("pour-le-navigateur.crt");

    let out = Command::new(bin())
        .args(["ca", "--ca-cert"]).arg(&cert)
        .arg("--ca-key").arg(d.join("ca.key"))
        .arg("--export").arg(&exporte)
        .current_dir(&d)
        .output()
        .unwrap();
    assert!(out.status.success());

    let a = std::fs::read_to_string(&cert).unwrap();
    let b = std::fs::read_to_string(&exporte).unwrap();
    assert_eq!(a, b, "l'export doit être le certificat racine, à l'identique");
    assert!(a.contains("BEGIN CERTIFICATE"), "et être un PEM exploitable");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------------------
// `auto`
// ---------------------------------------------------------------------------

/// `auto` n'est pas exécutable en test : elle modifie le proxy système et les
/// magasins de confiance de la machine. On vérifie au moins que la commande
/// existe, que ses options sont celles annoncées, et qu'elle est documentée —
/// sa logique interne est couverte côté `system::linux`.
#[test]
fn auto_expose_les_options_attendues() {
    let out = Command::new(bin()).args(["auto", "--help"]).output().unwrap();
    assert!(out.status.success());
    let aide = String::from_utf8(out.stdout).unwrap();
    for option in ["--config", "--listen", "--key", "--ca-cert", "--ca-key"] {
        assert!(aide.contains(option), "`auto` doit accepter {option} : {aide}");
    }
}

/// Toutes les sous-commandes annoncées dans l'aide doivent répondre à `--help` :
/// une commande listée mais cassée serait invisible autrement.
#[test]
fn toutes_les_sous_commandes_repondent() {
    for cmd in ["run", "auto", "keygen", "encrypt", "decrypt", "pair", "pair-group", "ca"] {
        let out = Command::new(bin()).args([cmd, "--help"]).output().unwrap();
        assert!(out.status.success(), "`{cmd} --help` doit répondre");
        assert!(
            !String::from_utf8_lossy(&out.stdout).is_empty(),
            "`{cmd} --help` doit décrire la commande"
        );
    }
}

// ---------------------------------------------------------------------------
// `on` / `off` / `status` — l'interrupteur
// ---------------------------------------------------------------------------
//
// Ces tests n'engagent JAMAIS le système : ils exercent les garde-fous qui
// entourent l'activation (état mémorisé, double activation, état corrompu).
// Le cycle complet a été validé à la main sur une machine jetable ; le rejouer
// ici modifierait le proxy et le magasin de confiance de la machine de test.

/// Sans état mémorisé, l'interrupteur se déclare inactif.
#[test]
fn status_est_inactif_par_defaut() {
    let d = tmpdir("statut-inactif");
    let out = Command::new(bin())
        .args(["status", "--state"])
        .arg(d.join("etat.json"))
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "inactif",
        "sans état mémorisé, rien n'est actif"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `off` alors que rien n'est actif ne doit ni échouer ni rien créer.
#[test]
fn off_sans_etat_ne_fait_rien_et_reussit() {
    let d = tmpdir("off-a-vide");
    let etat = d.join("etat.json");
    let out = Command::new(bin())
        .args(["off", "--state"])
        .arg(&etat)
        .output()
        .unwrap();

    assert!(out.status.success(), "couper deux fois n'est pas une erreur");
    assert!(
        String::from_utf8(out.stdout).unwrap().contains("Déjà inactif"),
        "l'utilisateur doit comprendre qu'il n'y avait rien à faire"
    );
    assert!(!etat.exists(), "aucun fichier ne doit être créé au passage");
    let _ = std::fs::remove_dir_all(&d);
}

/// Un `on` alors qu'un état existe déjà ne doit PAS réengager.
///
/// C'est le garde-fou important : réengager écraserait l'état d'origine
/// mémorisé par le premier `on` — l'état d'avant-brocolis serait alors perdu,
/// et `off` restaurerait une configuration déjà détournée.
#[test]
fn on_refuse_de_reengager_et_preserve_letat_memorise() {
    let d = tmpdir("double-on");
    let etat = d.join("etat.json");
    // Un état mémorisé plausible, écrit tel quel : la commande doit s'arrêter
    // AVANT de toucher au système, donc sans même le relire.
    std::fs::write(&etat, "{\"Inactive\":null}").unwrap();
    let avant = std::fs::read_to_string(&etat).unwrap();

    let out = Command::new(bin())
        .args(["on", "--config"])
        .arg(d.join("inexistant.toml"))
        .arg("--state")
        .arg(&etat)
        .current_dir(&d)
        .output()
        .unwrap();

    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stdout).unwrap().contains("Déjà actif"),
        "un second `on` doit être annoncé comme sans effet"
    );
    assert_eq!(
        std::fs::read_to_string(&etat).unwrap(),
        avant,
        "l'état d'origine mémorisé ne doit surtout pas être écrasé"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `status` doit refléter la présence d'un état, sans rien modifier.
#[test]
fn status_signale_actif_et_ne_modifie_rien() {
    let d = tmpdir("statut-actif");
    let etat = d.join("etat.json");
    std::fs::write(&etat, "{\"Inactive\":null}").unwrap();

    let out = Command::new(bin())
        .args(["status", "--state"]).arg(&etat).output().unwrap();

    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains("actif"), "doit annoncer l'état actif : {s}");
    assert!(etat.exists(), "`status` ne doit rien effacer");
    let _ = std::fs::remove_dir_all(&d);
}

/// Un état illisible doit produire une erreur qui dit quoi faire, et surtout
/// NE PAS effacer le fichier : il peut contenir la seule trace de la
/// configuration d'origine.
#[test]
fn off_sur_un_etat_corrompu_explique_et_conserve_le_fichier() {
    let d = tmpdir("etat-corrompu");
    let etat = d.join("etat.json");
    std::fs::write(&etat, "ceci n'est pas du json").unwrap();

    let out = Command::new(bin())
        .args(["off", "--state"]).arg(&etat)
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap();

    assert!(!out.status.success(), "un état illisible est une erreur");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("illisible"), "le message doit être explicite : {err}");
    assert!(
        err.contains("Supprimez"),
        "et indiquer comment s'en sortir : {err}"
    );
    assert!(etat.exists(), "le fichier ne doit pas être effacé sur erreur");
    let _ = std::fs::remove_dir_all(&d);
}
