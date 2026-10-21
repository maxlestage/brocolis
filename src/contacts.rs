//! Contacts : un nom, et la clé partagée avec cette personne.
//!
//! Dans `brocolis.toml`, chaque contact est une table :
//!
//! ```toml
//! [[contacts]]
//! name = "Alice"
//! key = "<clé partagée avec Alice>"
//! ```
//!
//! Un contact, une clé : elle chiffre ce qu'on lui envoie et ouvre ce qu'il
//! nous envoie. Une clé partagée est symétrique : ce qu'on chiffre pour
//! Alice, on peut aussi le relire soi-même.
//!
//! Les fonctions d'édition travaillent sur le texte TOML et en préservent
//! commentaires et mise en forme, comme [`crate::pairing::append_key_to_toml`].

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, Value};

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

/// Ajoute un contact. Un contact du même nom (casse indifférente) est
/// refusé : remplacer sa clé se fait explicitement, avec [`set_key`].
pub fn add(existing: &str, name: &str, key: &str) -> Result<String, String> {
    let name = check_name(name)?;
    let key = check_key(key)?;
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    if position(arr, &name).is_some() {
        return Err(format!("un contact « {name} » existe déjà"));
    }
    let mut t = Table::new();
    t.insert("name", toml_edit::value(name));
    t.insert("key", toml_edit::value(key));
    arr.push(t);
    Ok(doc.to_string())
}

/// Ajoute un contact appairé à deux avec son paquet de clés v3 (base64),
/// arrivé à `at` (secondes Unix) : ce qu'on lui envoie part en double
/// cliquet — en triple cliquet avec sa capacité v4 `caps` (§9.6).
pub fn add_paired(
    existing: &str,
    name: &str,
    key: &str,
    bundle: &str,
    caps: Option<&str>,
    at: u64,
) -> Result<String, String> {
    brocolis_proto::KeyBundle::from_base64(bundle.trim())
        .map_err(|e| format!("paquet de clés v3 invalide : {e}"))?;
    let out = add(existing, name, key)?;
    let mut doc = parse(&out)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).expect("contact ajouté");
    let t = arr.get_mut(i).expect("indice trouvé");
    t.insert("bundle", toml_edit::value(bundle.trim()));
    t.insert(
        "bundle_at",
        toml_edit::value(i64::try_from(at).unwrap_or(i64::MAX)),
    );
    t.insert("duo", toml_edit::value(true));
    if let Some(c) = caps.map(str::trim).filter(|c| !c.is_empty()) {
        t.insert("caps", toml_edit::value(c));
    }
    Ok(doc.to_string())
}

/// Ré-appaire un contact existant : nouvelle clé, nouveau paquet (et
/// capacité v4). Si le paquet porte **une autre identité** que celui qu'il
/// avait, l'ancienne est gardée (`previous_identity`) et le contact est
/// marqué `identity_changed` : l'envoi vers lui est bloqué jusqu'à
/// [`confirm_identity`] (§9.8). Renvoie le TOML et `true` dans ce cas.
pub fn repair_paired(
    existing: &str,
    name: &str,
    key: &str,
    bundle: &str,
    caps: Option<&str>,
    at: u64,
) -> Result<(String, bool), String> {
    let key = check_key(key)?;
    let new = brocolis_proto::KeyBundle::from_base64(bundle.trim())
        .map_err(|e| format!("paquet de clés v3 invalide : {e}"))?;
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).ok_or_else(|| format!("contact « {name} » inconnu"))?;
    let t = arr.get_mut(i).expect("indice trouvé");
    let old = t
        .get("bundle")
        .and_then(|v| v.as_str())
        .and_then(|b| brocolis_proto::KeyBundle::from_base64(b.trim()).ok());
    let (old_id, new_id) = (
        old.as_ref().map(crate::v3::identity_of),
        crate::v3::identity_of(&new),
    );
    let already = t
        .get("identity_changed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let changed = old_id.is_some_and(|o| o != new_id);
    for field in ["bundle", "bundle_at", "caps", "duo", "psk"] {
        t.remove(field);
    }
    t.insert("key", toml_edit::value(key));
    t.insert("bundle", toml_edit::value(bundle.trim()));
    t.insert(
        "bundle_at",
        toml_edit::value(i64::try_from(at).unwrap_or(i64::MAX)),
    );
    t.insert("duo", toml_edit::value(true));
    if let Some(c) = caps.map(str::trim).filter(|c| !c.is_empty()) {
        t.insert("caps", toml_edit::value(c));
    }
    if changed {
        // Une alerte encore en attente garde l'identité d'avant la première.
        if !already {
            use base64::Engine as _;
            let prev =
                base64::engine::general_purpose::STANDARD.encode(old_id.expect("identité connue"));
            t.insert("previous_identity", toml_edit::value(prev));
        }
        t.insert("identity_changed", toml_edit::value(true));
    }
    Ok((doc.to_string(), changed || already))
}

/// « J'ai comparé l'empreinte » : lève l'alerte de changement d'identité
/// d'un contact (§9.8) ; l'envoi vers lui reprend.
pub fn confirm_identity(existing: &str, name: &str) -> Result<String, String> {
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).ok_or_else(|| format!("contact « {name} » inconnu"))?;
    let t = arr.get_mut(i).expect("indice trouvé");
    t.remove("identity_changed");
    t.remove("previous_identity");
    Ok(doc.to_string())
}

/// Marque un contact « à deux, double cliquet » (PSK), ou retire la marque :
/// sans paquet échangé, le v3 part alors de la clé partagée. À ne faire que
/// si l'autre est à jour (un ancien client ne lirait rien).
pub fn set_psk(existing: &str, name: &str, on: bool) -> Result<String, String> {
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).ok_or_else(|| format!("contact « {name} » inconnu"))?;
    let t = arr.get_mut(i).expect("indice trouvé");
    if on {
        t.insert("psk", toml_edit::value(true));
    } else {
        t.remove("psk");
    }
    Ok(doc.to_string())
}

/// Remplace la clé d'un contact. Ce qu'il avait chiffré avec l'ancienne
/// devient illisible : c'est le but quand elle a fuité.
pub fn set_key(existing: &str, name: &str, key: &str) -> Result<String, String> {
    let key = check_key(key)?;
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).ok_or_else(|| format!("contact « {name} » inconnu"))?;
    let t = arr.get_mut(i).expect("indice trouvé");
    t.insert("key", toml_edit::value(key));
    // Une clé neuve n'est plus celle de l'appairage : son paquet ne lui va
    // plus (l'empreinte comparée portait sur l'ancienne).
    for field in [
        "bundle",
        "bundle_at",
        "caps",
        "duo",
        "psk",
        "identity_changed",
        "previous_identity",
    ] {
        t.remove(field);
    }
    Ok(doc.to_string())
}

/// Retire un contact, et le retire des destinataires du proxy
/// (`encrypt_to`) : sinon le proxy refuserait de démarrer.
pub fn remove(existing: &str, name: &str) -> Result<String, String> {
    let mut doc = parse(existing)?;
    let arr = contacts(&mut doc)?;
    let i = position(arr, name).ok_or_else(|| format!("contact « {name} » inconnu"))?;
    arr.remove(i);
    if let Some(to) = doc.get_mut("encrypt_to").and_then(|v| v.as_array_mut()) {
        to.retain(|v| !v.as_str().is_some_and(|n| same_name(n, name)));
        if to.is_empty() {
            doc.remove("encrypt_to");
        }
    }
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
        let mut arr = toml_edit::Array::new();
        for n in names {
            arr.push(check_name(n)?);
        }
        doc["encrypt_to"] = Item::Value(Value::Array(arr));
    }
    Ok(doc.to_string())
}

/// Les contacts dont la clé ouvre au moins un jeton du texte : « de qui
/// vient ce message ». Une clé partagée par plusieurs contacts (clé de
/// groupe) les nomme tous.
pub fn readers(input: &str, cfg: &Config) -> Vec<String> {
    cfg.contacts
        .iter()
        .filter(|c| {
            c.key()
                .and_then(|k| SymKey::from_base64(k).ok())
                .is_some_and(|k| decrypt_tokens_multi(input, &[k]).1 > 0)
        })
        .map(|c| c.name.clone())
        .collect()
}

/// Ajoute un groupe à cliquet : `id` (16 octets base64) est tiré par celui
/// qui crée le groupe (`None`) et recopié par les autres membres. Chaque
/// membre doit être un contact. Renvoie le TOML et l'identifiant.
pub fn add_group(
    existing: &str,
    name: &str,
    id: Option<&str>,
    members: &[String],
) -> Result<(String, String), String> {
    use base64::Engine as _;
    let name = check_name(name)?;
    let cfg: Config = toml::from_str(existing).map_err(|e| format!("TOML invalide : {e}"))?;
    if cfg.group(&name).is_some() {
        return Err(format!("un groupe « {name} » existe déjà"));
    }
    if members.is_empty() {
        return Err("un groupe a au moins un autre membre".into());
    }
    for m in members {
        if cfg.contact(m).is_none() {
            return Err(format!("contact « {} » inconnu", m.trim()));
        }
    }
    let id = match id {
        Some(i) => {
            let raw = base64::engine::general_purpose::STANDARD
                .decode(i.trim())
                .map_err(|_| "identifiant de groupe illisible".to_string())?;
            if raw.len() != 16 {
                return Err("identifiant de groupe : 16 octets".into());
            }
            i.trim().to_owned()
        }
        None => {
            let mut raw = [0u8; 16];
            rand::Rng::fill_bytes(&mut rand::rng(), &mut raw);
            base64::engine::general_purpose::STANDARD.encode(raw)
        }
    };
    let mut doc = parse(existing)?;
    let arr = doc
        .entry("groups")
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or_else(|| "`groups` doit être une liste de tables [[groups]]".to_string())?;
    let mut t = Table::new();
    t.insert("name", toml_edit::value(name));
    t.insert("id", toml_edit::value(id.clone()));
    let mut list = toml_edit::Array::new();
    for m in members {
        list.push(m.trim());
    }
    t.insert("members", toml_edit::value(list));
    arr.push(t);
    Ok((doc.to_string(), id))
}

/// Change les membres d'un groupe. Retirer quelqu'un fait tirer une chaîne
/// neuve au prochain envoi : il ne lit plus la suite.
pub fn set_group_members(existing: &str, name: &str, members: &[String]) -> Result<String, String> {
    let cfg: Config = toml::from_str(existing).map_err(|e| format!("TOML invalide : {e}"))?;
    for m in members {
        if cfg.contact(m).is_none() {
            return Err(format!("contact « {} » inconnu", m.trim()));
        }
    }
    let mut doc = parse(existing)?;
    let arr = doc
        .get_mut("groups")
        .and_then(|g| g.as_array_of_tables_mut())
        .ok_or_else(|| format!("groupe « {name} » inconnu"))?;
    let t = arr
        .iter_mut()
        .find(|t| {
            t.get("name")
                .and_then(|v| v.as_str())
                .is_some_and(|n| same_name(n, name))
        })
        .ok_or_else(|| format!("groupe « {name} » inconnu"))?;
    let mut list = toml_edit::Array::new();
    for m in members {
        list.push(m.trim());
    }
    t.insert("members", toml_edit::value(list));
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::encrypt_message;

    fn cfg(s: &str) -> Config {
        toml::from_str(s).unwrap()
    }

    #[test]
    fn creer_changer_de_cle_renommer_retirer() {
        let (k1, k2) = (
            SymKey::generate().to_base64(),
            SymKey::generate().to_base64(),
        );
        let t = "# mes réglages\nlisten = \"127.0.0.1:8080\"\nkey = \"x\"\n";
        let t = add(t, "Alice", &k1).unwrap();
        assert!(t.contains("# mes réglages"), "commentaires préservés");
        assert_eq!(cfg(&t).contact("alice").unwrap().key(), Some(k1.as_str()));
        assert!(
            add(&t, "alice ", &k2).is_err(),
            "un contact du même nom existe déjà"
        );

        let t = set_key(&t, "ALICE", &k2).unwrap();
        let c = cfg(&t);
        assert_eq!(c.contacts.len(), 1);
        assert_eq!(c.contact("Alice").unwrap().key(), Some(k2.as_str()));

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
        assert!(add("", "  ", &k).is_err(), "nom vide");
        assert!(
            add("", &"x".repeat(NAME_MAX + 1), &k).is_err(),
            "nom trop long"
        );
        assert!(add("", "Bob", "pas une clé").is_err());
        let t = add("", "Bob", &k).unwrap();
        assert!(set_key(&t, "Bob", "QUJD").is_err(), "clé trop courte");
        assert!(set_key(&t, "Zoé", &k).is_err(), "contact inconnu");
        let t = add(&t, "Carole", &SymKey::generate().to_base64()).unwrap();
        assert!(
            rename(&t, "Bob", "carole").is_err(),
            "deux contacts du même nom"
        );
        assert!(remove(&t, "Zoé").is_err());
    }

    #[test]
    fn de_qui_vient_le_message() {
        let (alice, bob) = (SymKey::generate(), SymKey::generate());
        let t = add("", "Alice", &alice.to_base64()).unwrap();
        let t = add(&t, "Bob", &bob.to_base64()).unwrap();
        let c = cfg(&t);
        let (de_bob, _) = encrypt_message("salut", &[bob]).unwrap();
        assert_eq!(readers(&de_bob, &c), ["Bob"]);
        assert!(readers("rien de chiffré", &c).is_empty());
    }
}
