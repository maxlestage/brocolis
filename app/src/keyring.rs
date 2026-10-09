//! Le trousseau de l'application, lu et écrit dans `brocolis.toml` : vos clés
//! sans nom, et vos contacts avec leurs clés.
//!
//! Le fichier reste la seule source de vérité : chaque modification y est
//! écrite aussitôt (en préservant commentaires et mise en forme), puis relue.
//! La ligne de commande et l'application voient donc toujours les mêmes clés.

use std::path::Path;

use brocolis::{Config, SymKey, contacts};

/// Une clé, telle qu'affichée.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub b64: String,
    /// Clé sans nom : la principale chiffre par défaut. Clé d'un contact : la
    /// clé active, celle qui chiffre pour lui. Les autres servent à lire.
    pub primary: bool,
    /// Empreinte courte, à comparer de vive voix (même calcul que l'app iOS).
    pub fingerprint: String,
}

impl Entry {
    fn new(b64: &str, primary: bool) -> Self {
        Self {
            b64: b64.to_owned(),
            primary,
            fingerprint: brocolis::agreement::group_fingerprint(b64),
        }
    }
}

/// Un contact et ses clés, la clé active d'abord.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    pub name: String,
    pub keys: Vec<Entry>,
}

/// Tout ce que le fichier contient de clés.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Book {
    /// Vos clés sans nom (`key` puis `keys`), la principale d'abord.
    pub own: Vec<Entry>,
    pub contacts: Vec<Person>,
    /// Pour qui le proxy chiffre (`encrypt_to`). Vide : la clé principale.
    pub encrypt_to: Vec<String>,
}

impl Book {
    /// La clé principale, s'il y en a une.
    pub fn primary(&self) -> Option<&Entry> {
        self.own.first().filter(|e| e.primary)
    }

    pub fn person(&self, name: &str) -> Option<&Person> {
        self.contacts
            .iter()
            .find(|p| brocolis::config::same_name(&p.name, name))
    }

    /// Le proxy chiffre-t-il pour ce contact ?
    pub fn is_target(&self, name: &str) -> bool {
        self.encrypt_to
            .iter()
            .any(|n| brocolis::config::same_name(n, name))
    }
}

pub fn load(config: &Path) -> anyhow::Result<Book> {
    let cfg = Config::load(config)?;
    let primary = cfg.key.as_deref().map(str::trim).filter(|k| !k.is_empty());
    let mut own: Vec<Entry> = Vec::new();
    for k in cfg.key.iter().chain(cfg.keys.iter()) {
        let k = k.trim();
        if !k.is_empty() && !own.iter().any(|e| e.b64 == k) {
            own.push(Entry::new(k, own.is_empty() && primary == Some(k)));
        }
    }
    let contacts = cfg
        .contacts
        .iter()
        .map(|c| Person {
            name: c.name.clone(),
            keys: c
                .keys
                .iter()
                .map(|k| k.trim())
                .filter(|k| !k.is_empty())
                .enumerate()
                .map(|(i, k)| Entry::new(k, i == 0))
                .collect(),
        })
        .collect();
    Ok(Book {
        own,
        contacts,
        encrypt_to: cfg.encrypt_to.clone(),
    })
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

// --- Vos clés sans nom ---------------------------------------------------

/// Ajoute une clé. Sans clé principale, elle le devient : sinon rien ne
/// chiffrerait.
pub fn add(config: &Path, b64: &str) -> anyhow::Result<()> {
    let b64 = validate(b64)?;
    let has_primary = load(config)?.primary().is_some();
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

/// Range une clé sans nom sous un contact (créé s'il n'existe pas) : on saura
/// désormais de qui viennent les messages qu'elle ouvre.
pub fn assign(config: &Path, b64: &str, name: &str) -> anyhow::Result<()> {
    edit(config, |s| {
        let s = contacts::add_key(s, name, b64)?;
        brocolis::pairing::remove_key_from_toml(&s, b64)
    })
}

// --- Contacts --------------------------------------------------------------

/// Ajoute une clé reçue à un contact, en le créant au besoin.
pub fn contact_add(config: &Path, name: &str, b64: &str) -> anyhow::Result<()> {
    let b64 = validate(b64)?;
    edit(config, |s| contacts::add_key(s, name, &b64))
}

/// Crée une clé neuve pour un contact (créé au besoin) ; c'est sa clé active
/// s'il n'en avait pas. Renvoie la clé, à lui transmettre.
pub fn contact_generate(config: &Path, name: &str) -> anyhow::Result<String> {
    let b64 = SymKey::generate().to_base64();
    let had_keys = load(config)?
        .person(name)
        .is_some_and(|p| !p.keys.is_empty());
    edit(config, |s| {
        let s = contacts::add_key(s, name, &b64)?;
        // Une clé créée pour quelqu'un est faite pour lui écrire : elle
        // devient active.
        if had_keys {
            contacts::set_active_key(&s, name, &b64)
        } else {
            Ok(s)
        }
    })?;
    Ok(b64)
}

pub fn contact_use(config: &Path, name: &str, b64: &str) -> anyhow::Result<()> {
    edit(config, |s| contacts::set_active_key(s, name, b64))
}

pub fn contact_remove_key(config: &Path, name: &str, b64: &str) -> anyhow::Result<()> {
    edit(config, |s| contacts::remove_key(s, name, b64))
}

pub fn contact_rename(config: &Path, old: &str, new: &str) -> anyhow::Result<()> {
    edit(config, |s| contacts::rename(s, old, new))
}

pub fn contact_delete(config: &Path, name: &str) -> anyhow::Result<()> {
    edit(config, |s| contacts::remove(s, name))
}

/// Pour qui le proxy chiffre. Vide : la clé principale.
pub fn set_encrypt_to(config: &Path, names: &[String]) -> anyhow::Result<()> {
    edit(config, |s| contacts::set_encrypt_to(s, names))
}

// --- Chiffrer, déchiffrer ------------------------------------------------

/// Les clés pour chiffrer un message : la clé principale si `me`, et la clé
/// active de chaque contact nommé. Sans doublon.
pub fn recipients(config: &Path, me: bool, names: &[String]) -> anyhow::Result<Vec<SymKey>> {
    let book = load(config)?;
    let mut b64s: Vec<&str> = Vec::new();
    if me && let Some(p) = book.primary() {
        b64s.push(&p.b64);
    }
    for n in names {
        let person = book
            .person(n)
            .ok_or_else(|| anyhow::anyhow!("contact « {n} » inconnu"))?;
        let active = person
            .keys
            .first()
            .ok_or_else(|| anyhow::anyhow!("« {n} » n'a aucune clé"))?;
        b64s.push(&active.b64);
    }
    let mut keys: Vec<SymKey> = Vec::new();
    for b in b64s {
        let k = SymKey::from_base64(b).map_err(|e| anyhow::anyhow!("{e}"))?;
        if !keys.iter().any(|x| x.to_base64() == k.to_base64()) {
            keys.push(k);
        }
    }
    Ok(keys)
}

/// Déchiffre avec TOUTES les clés (les vôtres et celles des contacts), et dit
/// de qui vient le message. `None` : aucune clé du tout.
pub fn read(config: &Path, input: &str) -> anyhow::Result<Option<(String, usize, Vec<String>)>> {
    let cfg = Config::load(config)?;
    let keys = cfg
        .all_key_b64()
        .iter()
        .map(|k| SymKey::from_base64(k).map_err(|e| anyhow::anyhow!("{e}")))
        .collect::<anyhow::Result<Vec<_>>>()?;
    if keys.is_empty() {
        return Ok(None);
    }
    let (out, n) = brocolis::crypto::decrypt_tokens_multi(input, &keys);
    Ok(Some((out, n, contacts::readers(input, &cfg))))
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
        let initiale = load(&f.config).unwrap().own;
        assert_eq!(initiale.len(), 1);
        assert!(initiale[0].primary);

        let nouvelle = generate(&f.config).unwrap();
        let apres = load(&f.config).unwrap().own;
        assert_eq!(apres.len(), 2);
        assert!(!apres.iter().find(|e| e.b64 == nouvelle).unwrap().primary);

        make_primary(&f.config, &nouvelle).unwrap();
        let promue = load(&f.config).unwrap().own;
        assert_eq!(promue[0].b64, nouvelle, "la principale vient en premier");
        assert!(promue[0].primary);
        assert_eq!(promue.len(), 2, "l'ancienne principale reste, pour lire");

        remove(&f.config, &nouvelle).unwrap();
        let reste = load(&f.config).unwrap().own;
        assert_eq!(reste.len(), 1);
        assert!(!reste[0].primary, "plus de principale : rien ne chiffre");

        // Ajouter une clé quand il n'y a plus de principale la rend principale.
        let autre = SymKey::generate().to_base64();
        add(&f.config, &autre).unwrap();
        assert!(
            load(&f.config)
                .unwrap()
                .own
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
        assert!(contact_add(&f.config, "Alice", "QUJD").is_err());
        assert_eq!(avant, std::fs::read_to_string(&f.config).unwrap());
        let _ = std::fs::remove_dir_all(&f.dir);
    }

    #[test]
    fn contacts_a_plusieurs_cles() {
        let f = fresh("contacts");
        let a1 = contact_generate(&f.config, "Alice").unwrap();
        let a2 = SymKey::generate().to_base64();
        contact_add(&f.config, "alice", &a2).unwrap();
        let book = load(&f.config).unwrap();
        let alice = book.person("ALICE").unwrap();
        assert_eq!(alice.keys.len(), 2);
        assert!(alice.keys[0].primary && alice.keys[0].b64 == a1);

        // Une clé GÉNÉRÉE pour un contact existant devient active.
        let a3 = contact_generate(&f.config, "Alice").unwrap();
        assert_eq!(load(&f.config).unwrap().contacts[0].keys[0].b64, a3);
        contact_use(&f.config, "Alice", &a2).unwrap();
        assert_eq!(load(&f.config).unwrap().contacts[0].keys[0].b64, a2);

        // Une clé sans nom rangée sous un contact quitte vos clés.
        let anonyme = generate(&f.config).unwrap();
        assign(&f.config, &anonyme, "Bob").unwrap();
        let book = load(&f.config).unwrap();
        assert!(!book.own.iter().any(|e| e.b64 == anonyme));
        assert_eq!(book.person("Bob").unwrap().keys[0].b64, anonyme);

        set_encrypt_to(&f.config, &["Bob".into()]).unwrap();
        assert!(load(&f.config).unwrap().is_target("bob"));
        contact_rename(&f.config, "Bob", "Robert").unwrap();
        assert!(load(&f.config).unwrap().is_target("Robert"));
        contact_delete(&f.config, "Robert").unwrap();
        let book = load(&f.config).unwrap();
        assert!(book.person("Robert").is_none() && book.encrypt_to.is_empty());
        contact_remove_key(&f.config, "Alice", &a2).unwrap();
        assert_eq!(load(&f.config).unwrap().contacts[0].keys.len(), 2);
        let _ = std::fs::remove_dir_all(&f.dir);
    }

    #[test]
    fn chiffrer_pour_choisir_qui_lit() {
        let f = fresh("destinataires");
        let alice = contact_generate(&f.config, "Alice").unwrap();
        let keys = recipients(&f.config, false, &["Alice".into()]).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].to_base64(), alice);
        let both = recipients(&f.config, true, &["Alice".into()]).unwrap();
        assert_eq!(both.len(), 2, "moi et Alice");
        assert!(recipients(&f.config, false, &["Zoé".into()]).is_err());

        let (chiffre, _) = brocolis::crypto::encrypt_message("salut", &keys).unwrap();
        let (clair, n, de) = read(&f.config, &chiffre).unwrap().unwrap();
        assert_eq!((clair.as_str(), n), ("salut", 1));
        assert_eq!(de, ["Alice"]);
        let _ = std::fs::remove_dir_all(&f.dir);
    }
}
