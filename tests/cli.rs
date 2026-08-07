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
