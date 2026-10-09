//! Le trousseau de l'application, lu et écrit dans `brocolis.toml` : vos
//! contacts (un nom, une clé) et vos clés sans nom.
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
    /// La clé principale (clés sans nom) : elle chiffre par défaut.
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

/// Un contact et sa clé.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    pub name: String,
    pub key: Entry,
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
            key: Entry::new(c.key.trim(), false),
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

/// Fait d'une clé sans nom un contact : on saura désormais de qui viennent
/// les messages qu'elle ouvre.
pub fn assign(config: &Path, b64: &str, name: &str) -> anyhow::Result<()> {
    edit(config, |s| {
        let s = contacts::add(s, name, b64)?;
        brocolis::pairing::remove_key_from_toml(&s, b64)
    })
}

// --- Contacts --------------------------------------------------------------

/// Nouveau contact, avec la clé qu'il nous a donnée.
pub fn contact_add(config: &Path, name: &str, b64: &str) -> anyhow::Result<()> {
    let b64 = validate(b64)?;
    edit(config, |s| contacts::add(s, name, &b64))
}

/// Nouveau contact, avec une clé créée ici. Renvoie la clé, à lui transmettre.
pub fn contact_generate(config: &Path, name: &str) -> anyhow::Result<String> {
    let b64 = SymKey::generate().to_base64();
    edit(config, |s| contacts::add(s, name, &b64))?;
    Ok(b64)
}

/// Remplace la clé d'un contact : celle qu'il a donnée, ou (`None`) une clé
/// neuve, renvoyée pour la lui transmettre.
pub fn contact_set_key(config: &Path, name: &str, b64: Option<&str>) -> anyhow::Result<String> {
    let b64 = match b64 {
        Some(k) => validate(k)?,
        None => SymKey::generate().to_base64(),
    };
    edit(config, |s| contacts::set_key(s, name, &b64))?;
    Ok(b64)
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

/// Les clés pour chiffrer un message : la clé principale si `me`, et celle de
/// chaque contact nommé. Sans doublon.
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
        b64s.push(&person.key.b64);
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
    fn un_contact_une_cle() {
        let f = fresh("contacts");
        let a1 = contact_generate(&f.config, "Alice").unwrap();
        assert!(
            contact_generate(&f.config, "alice").is_err(),
            "Alice existe déjà"
        );
        assert!(contact_add(&f.config, "ALICE", &SymKey::generate().to_base64()).is_err());
        assert_eq!(
            load(&f.config).unwrap().person("alice").unwrap().key.b64,
            a1
        );

        // Changer de clé : celle reçue, ou une neuve.
        let recue = SymKey::generate().to_base64();
        contact_set_key(&f.config, "Alice", Some(&recue)).unwrap();
        assert_eq!(
            load(&f.config).unwrap().person("Alice").unwrap().key.b64,
            recue
        );
        let neuve = contact_set_key(&f.config, "Alice", None).unwrap();
        assert_ne!(neuve, recue);
        let book = load(&f.config).unwrap();
        assert_eq!(book.contacts.len(), 1);
        assert_eq!(book.person("Alice").unwrap().key.b64, neuve);

        // Une clé sans nom devient un contact, et quitte vos clés.
        let anonyme = generate(&f.config).unwrap();
        assign(&f.config, &anonyme, "Bob").unwrap();
        let book = load(&f.config).unwrap();
        assert!(!book.own.iter().any(|e| e.b64 == anonyme));
        assert_eq!(book.person("Bob").unwrap().key.b64, anonyme);

        set_encrypt_to(&f.config, &["Bob".into()]).unwrap();
        assert!(load(&f.config).unwrap().is_target("bob"));
        contact_rename(&f.config, "Bob", "Robert").unwrap();
        assert!(load(&f.config).unwrap().is_target("Robert"));
        contact_delete(&f.config, "Robert").unwrap();
        let book = load(&f.config).unwrap();
        assert!(book.person("Robert").is_none() && book.encrypt_to.is_empty());
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
