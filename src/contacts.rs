//! Contacts : un nom, et les clés partagées avec cette personne.
//!
//! Dans `brocolis.toml`, chaque contact est une table :
//!
//! ```toml
//! [[contacts]]
//! name = "Alice"
//! keys = ["<clé active>", "<ancienne clé, encore lisible>"]
//! ```
//!
//! La première clé chiffre ce qu'on envoie à ce contact ; toutes servent à
//! lire ce qu'il nous envoie. Une clé partagée est symétrique : ce qu'on
//! chiffre pour Alice, on peut aussi le relire soi-même.
//!
//! Les fonctions d'édition travaillent sur le texte TOML et en préservent
//! commentaires et mise en forme, comme [`crate::pairing::append_key_to_toml`].

use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, Value};

use crate::config::{same_name, Config};
use crate::crypto::{decrypt_tokens_multi, SymKey};

/// Longueur maximale d'un nom de contact.
pub const NAME_MAX: usize = 64;

fn parse(existing: &str) -> Result<DocumentMut, String> {
    existing
        .parse::<DocumentMut>()
        .map_err(|e| format!("TOML invalide : {e}"))
}

fn check_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("le nom du contact est vide".into());
    }
    if name.chars().count() > NAME_MAX {
        return Err(format!("nom trop long ({NAME_MAX} caractères au plus)"));
    }
    Ok(name.to_owned())
}

fn check_key(key: &str) -> Result<String, String> {
    let key = key.trim();
    SymKey::from_base64(key).map_err(|e| e.to_string())?;
    Ok(key.to_owned())
}

fn contacts(doc: &mut DocumentMut) -> Result<&mut ArrayOfTables, String> {
    doc.entry("contacts")
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or_else(|| "`contacts` doit être une liste de tables [[contacts]]".into())
}

fn position(arr: &ArrayOfTables, name: &str) -> Option<usize> {
    arr.iter().position(|t| {
        t.get("name")
            .and_then(|v| v.as_str())
            .is_some_and(|n| same_name(n, name))
    })
}

fn keys_of(table: &mut Table) -> Result<&mut Array, String> {
    table
        .entry("keys")
        .or_insert(Item::Value(Value::Array(Array::new())))
        .as_array_mut()
        .ok_or_else(|| "`keys` d'un contact doit être une liste".into())
}

/// Ajoute une clé à un contact, en le créant s'il n'existe pas (la clé est
/// alors sa clé active). Pour un contact existant, la clé rejoint les
/// suivantes ; [`set_active_key`] la fait passer en tête.
pub fn add_key(existing: &str, name: &str, key: &str) -> Result<String, String> {
    let name = check_name(name)?;
    let key = check_key(key)?;
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    match position(arr, &name) {
        Some(i) => {
            let keys = keys_of(arr.get_mut(i).expect("indice trouvé"))?;
            if !keys.iter().any(|v| v.as_str() == Some(key.as_str())) {
                keys.push(key);
            }
        }
        None => {
            let mut t = Table::new();
            t.insert("name", toml_edit::value(name));
            let mut keys = Array::new();
            keys.push(key);
            t.insert("keys", Item::Value(Value::Array(keys)));
            arr.push(t);
        }
    }
    Ok(doc.to_string())
}

/// Fait de `key` la clé active du contact (celle qui chiffre).
pub fn set_active_key(existing: &str, name: &str, key: &str) -> Result<String, String> {
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).ok_or_else(|| format!("contact « {name} » inconnu"))?;
    let keys = keys_of(arr.get_mut(i).expect("indice trouvé"))?;
    let at = keys
        .iter()
        .position(|v| v.as_str() == Some(key))
        .ok_or("cette clé n'appartient pas au contact")?;
    let v = keys.remove(at);
    keys.insert(0, v.as_str().unwrap_or_default());
    Ok(doc.to_string())
}

/// Retire une clé d'un contact. Le contact reste, même sans clé — mais il
/// n'est alors plus destinataire du proxy (`encrypt_to`) : sans clé, le proxy
/// refuserait de démarrer.
pub fn remove_key(existing: &str, name: &str, key: &str) -> Result<String, String> {
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).ok_or_else(|| format!("contact « {name} » inconnu"))?;
    let keys = keys_of(arr.get_mut(i).expect("indice trouvé"))?;
    keys.retain(|v| v.as_str() != Some(key));
    if keys.is_empty() {
        unlist(&mut doc, name);
    }
    Ok(doc.to_string())
}

/// Retire un nom des destinataires du proxy.
fn unlist(doc: &mut DocumentMut, name: &str) {
    if let Some(to) = doc.get_mut("encrypt_to").and_then(|v| v.as_array_mut()) {
        to.retain(|v| !v.as_str().is_some_and(|n| same_name(n, name)));
        if to.is_empty() {
            doc.remove("encrypt_to");
        }
    }
}

/// Retire un contact et toutes ses clés, et le retire des destinataires du
/// proxy (`encrypt_to`) : sinon le proxy refuserait de démarrer.
pub fn remove(existing: &str, name: &str) -> Result<String, String> {
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).ok_or_else(|| format!("contact « {name} » inconnu"))?;
    arr.remove(i);
    unlist(&mut doc, name);
    Ok(doc.to_string())
}

/// Renomme un contact (et le nom repris dans `encrypt_to`).
pub fn rename(existing: &str, old: &str, new: &str) -> Result<String, String> {
    let new = check_name(new)?;
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, old).ok_or_else(|| format!("contact « {old} » inconnu"))?;
    if position(arr, &new).is_some_and(|j| j != i) {
        return Err(format!("un contact « {new} » existe déjà"));
    }
    arr.get_mut(i)
        .expect("indice trouvé")
        .insert("name", toml_edit::value(new.clone()));
    if let Some(to) = doc.get_mut("encrypt_to").and_then(|v| v.as_array_mut()) {
        for v in to.iter_mut() {
            if v.as_str().is_some_and(|n| same_name(n, old)) {
                *v = Value::from(new.clone());
            }
        }
    }
    Ok(doc.to_string())
}

/// Fixe les destinataires du proxy (`encrypt_to`). Une liste vide retire le
/// champ : retour à la règle par défaut (la clé principale).
pub fn set_encrypt_to(existing: &str, names: &[String]) -> Result<String, String> {
    let mut doc = parse(existing)?;
    if names.is_empty() {
        doc.remove("encrypt_to");
    } else {
        let mut arr = Array::new();
        for n in names {
            arr.push(check_name(n)?);
        }
        doc["encrypt_to"] = Item::Value(Value::Array(arr));
    }
    Ok(doc.to_string())
}

/// Les contacts dont une clé ouvre au moins un jeton du texte : « de qui
/// vient ce message ». Une clé partagée avec plusieurs contacts (clé de
/// groupe) les nomme tous.
pub fn readers(input: &str, cfg: &Config) -> Vec<String> {
    cfg.contacts
        .iter()
        .filter(|c| {
            let keys: Vec<SymKey> = c
                .keys
                .iter()
                .filter_map(|k| SymKey::from_base64(k.trim()).ok())
                .collect();
            !keys.is_empty() && decrypt_tokens_multi(input, &keys).1 > 0
        })
        .map(|c| c.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::encrypt_message;

    fn cfg(s: &str) -> Config {
        toml::from_str(s).unwrap()
    }

    #[test]
    fn creer_completer_activer_retirer() {
        let (k1, k2) = (
            SymKey::generate().to_base64(),
            SymKey::generate().to_base64(),
        );
        let t = "# mes réglages\nlisten = \"127.0.0.1:8080\"\nkey = \"x\"\n";
        let t = add_key(t, "Alice", &k1).unwrap();
        let t = add_key(&t, "alice ", &k2).unwrap(); // même contact
        assert!(t.contains("# mes réglages"), "commentaires préservés");
        let c = cfg(&t);
        assert_eq!(c.contacts.len(), 1);
        assert_eq!(c.contacts[0].keys, [k1.clone(), k2.clone()]);
        assert_eq!(c.contacts[0].active_key(), Some(k1.as_str()));

        let t = set_active_key(&t, "ALICE", &k2).unwrap();
        assert_eq!(
            cfg(&t).contact("alice").unwrap().active_key(),
            Some(k2.as_str())
        );

        let t = remove_key(&t, "Alice", &k2).unwrap();
        assert_eq!(
            cfg(&t).contact("Alice").unwrap().keys,
            std::slice::from_ref(&k1)
        );

        // Sa dernière clé retirée, un contact cesse d'être destinataire.
        let sans = set_encrypt_to(&t, &["Alice".into()]).unwrap();
        let sans = remove_key(&sans, "Alice", &k1).unwrap();
        let c = cfg(&sans);
        assert!(c.contact("Alice").unwrap().keys.is_empty());
        assert!(c.encrypt_to.is_empty() && c.encryption_targets().unwrap().is_none());

        let t = set_encrypt_to(&t, &["Alice".into()]).unwrap();
        let t = rename(&t, "Alice", "Alice Martin").unwrap();
        let c = cfg(&t);
        assert_eq!(
            c.encrypt_to,
            ["Alice Martin"],
            "le destinataire suit le renommage"
        );
        assert!(c.encryption_targets().unwrap().is_some());

        let t = remove(&t, "alice martin").unwrap();
        let c = cfg(&t);
        assert!(c.contacts.is_empty());
        assert!(
            c.encrypt_to.is_empty(),
            "un contact retiré n'est plus destinataire"
        );
        assert_eq!(c.key.as_deref(), Some("x"), "le reste est intact");
    }

    #[test]
    fn refus_clairs() {
        let k = SymKey::generate().to_base64();
        assert!(add_key("", "  ", &k).is_err(), "nom vide");
        assert!(
            add_key("", &"x".repeat(NAME_MAX + 1), &k).is_err(),
            "nom trop long"
        );
        assert!(add_key("", "Bob", "pas une clé").is_err());
        let t = add_key("", "Bob", &k).unwrap();
        let t = add_key(&t, "Carole", &SymKey::generate().to_base64()).unwrap();
        assert!(
            rename(&t, "Bob", "carole").is_err(),
            "deux contacts du même nom"
        );
        assert!(remove(&t, "Zoé").is_err());
    }

    #[test]
    fn de_qui_vient_le_message() {
        let (alice, bob) = (SymKey::generate(), SymKey::generate());
        let t = add_key("", "Alice", &alice.to_base64()).unwrap();
        let t = add_key(&t, "Bob", &bob.to_base64()).unwrap();
        let c = cfg(&t);
        let (de_bob, _) = encrypt_message("salut", &[bob]).unwrap();
        assert_eq!(readers(&de_bob, &c), ["Bob"]);
        assert!(readers("rien de chiffré", &c).is_empty());
    }
}
