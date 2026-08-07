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
