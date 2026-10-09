//! Le trousseau de l'application, lu et écrit dans `brocolis.toml`.
//!
//! Le fichier reste la seule source de vérité : chaque modification y est
//! écrite aussitôt (en préservant commentaires et mise en forme), puis relue.
//! La ligne de commande et l'application voient donc toujours les mêmes clés.

use std::path::Path;

use brocolis::{Config, KeyManager, SymKey};

/// Une clé du trousseau, telle qu'affichée.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub b64: String,
    /// La clé principale chiffre ; les autres servent seulement à lire.
    pub primary: bool,
    /// Empreinte courte, à comparer de vive voix (même calcul que l'app iOS).
    pub fingerprint: String,
}

/// Les clés du fichier, la principale d'abord.
pub fn load(config: &Path) -> anyhow::Result<Vec<Entry>> {
    let cfg = Config::load(config)?;
    let primary = cfg.key.as_deref().map(str::trim).filter(|k| !k.is_empty());
    Ok(cfg
        .all_key_b64()
        .into_iter()
        .enumerate()
        .map(|(i, b64)| Entry {
            primary: i == 0 && primary == Some(b64.as_str()),
            fingerprint: brocolis::agreement::group_fingerprint(&b64),
            b64,
        })
        .collect())
}

fn edit(config: &Path, f: impl FnOnce(&str) -> Result<String, String>) -> anyhow::Result<()> {
    let existing = std::fs::read_to_string(config).unwrap_or_default();
    let updated = f(&existing).map_err(|e| anyhow::anyhow!(e))?;
    std::fs::write(config, updated)?;
    Ok(())
}

/// Vérifie qu'une chaîne est une clé utilisable (32 octets en base64).
pub fn validate(b64: &str) -> anyhow::Result<String> {
    let b64 = b64.trim();
    SymKey::from_base64(b64).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(b64.to_owned())
}

/// Ajoute une clé. Sans clé principale, elle le devient : sinon rien ne
/// chiffrerait.
pub fn add(config: &Path, b64: &str) -> anyhow::Result<()> {
    let b64 = validate(b64)?;
    let has_primary = load(config)?.iter().any(|e| e.primary);
    edit(config, |s| {
        if has_primary {
            brocolis::pairing::append_key_to_toml(s, &b64)
        } else {
            brocolis::pairing::set_primary_key_in_toml(s, &b64)
        }
    })
}

/// Génère une clé neuve et l'ajoute. Renvoie la clé, pour qu'on puisse la
/// partager.
pub fn generate(config: &Path) -> anyhow::Result<String> {
    let b64 = SymKey::generate().to_base64();
    add(config, &b64)?;
    Ok(b64)
}

pub fn remove(config: &Path, b64: &str) -> anyhow::Result<()> {
    edit(config, |s| brocolis::pairing::remove_key_from_toml(s, b64))
}

pub fn make_primary(config: &Path, b64: &str) -> anyhow::Result<()> {
    edit(config, |s| {
        brocolis::pairing::set_primary_key_in_toml(s, b64)
    })
}

/// Les clés prêtes à l'emploi, avec la même règle que le proxy : chiffrer avec
/// la principale (ou toutes si `encrypt_multi`), déchiffrer avec toutes.
pub fn manager(config: &Path) -> anyhow::Result<Option<KeyManager>> {
    let cfg = Config::load(config)?;
    let keys = cfg
        .all_key_b64()
        .iter()
        .map(|k| SymKey::from_base64(k).map_err(|e| anyhow::anyhow!("{e}")))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok((!keys.is_empty()).then(|| KeyManager::from_keys(keys, cfg.encrypt_multi)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::Files;

    fn fresh(name: &str) -> Files {
        let dir = std::env::temp_dir().join(format!("brocolis-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let files = Files::at(dir);
        files.ensure().unwrap();
        files
    }

    #[test]
    fn ajouter_promouvoir_retirer() {
        let f = fresh("trousseau");
        let initiale = load(&f.config).unwrap();
        assert_eq!(initiale.len(), 1);
        assert!(initiale[0].primary);

        let nouvelle = generate(&f.config).unwrap();
        let apres = load(&f.config).unwrap();
        assert_eq!(apres.len(), 2);
        assert!(!apres.iter().find(|e| e.b64 == nouvelle).unwrap().primary);

        make_primary(&f.config, &nouvelle).unwrap();
        let promue = load(&f.config).unwrap();
        assert_eq!(promue[0].b64, nouvelle, "la principale vient en premier");
        assert!(promue[0].primary);
        assert_eq!(promue.len(), 2, "l'ancienne principale reste, pour lire");

        remove(&f.config, &nouvelle).unwrap();
        let reste = load(&f.config).unwrap();
        assert_eq!(reste.len(), 1);
        assert!(!reste[0].primary, "plus de principale : rien ne chiffre");

        // Ajouter une clé quand il n'y a plus de principale la rend principale.
        let autre = SymKey::generate().to_base64();
        add(&f.config, &autre).unwrap();
        assert!(
            load(&f.config)
                .unwrap()
                .iter()
                .any(|e| e.b64 == autre && e.primary)
        );
        let _ = std::fs::remove_dir_all(&f.dir);
    }

    #[test]
    fn une_cle_invalide_est_refusee_sans_toucher_au_fichier() {
        let f = fresh("invalide");
        let avant = std::fs::read_to_string(&f.config).unwrap();
        assert!(add(&f.config, "pas une clé").is_err());
        assert!(
            add(&f.config, "QUJD").is_err(),
            "base64 valide mais trop court"
        );
        assert_eq!(avant, std::fs::read_to_string(&f.config).unwrap());
        let _ = std::fs::remove_dir_all(&f.dir);
    }

    #[test]
    fn le_gestionnaire_chiffre_et_dechiffre() {
        let f = fresh("gestionnaire");
        let km = manager(&f.config).unwrap().unwrap();
        let (chiffre, n) =
            brocolis::crypto::encrypt_marked_multi("rdv ((18h))", &km.encryption_keys());
        assert_eq!(n, 1);
        let (clair, m) = brocolis::crypto::decrypt_tokens_multi(&chiffre, &km.all_keys());
        assert_eq!((clair.as_str(), m), ("rdv 18h", 1));
        let _ = std::fs::remove_dir_all(&f.dir);
    }
}
