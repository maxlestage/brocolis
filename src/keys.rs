//! Trousseau de clés symétriques en mémoire.
//!
//! Aligné sur le `BrocolisKeyStore` de la messagerie iOS : on garde PLUSIEURS
//! clés à la fois. La clé « primaire » (index 0) sert au chiffrement par
//! défaut ; au déchiffrement on essaie TOUTES les clés. On peut ainsi parler à
//! plusieurs correspondants sans jamais changer de clé active.
//!
//! La rotation remplace la clé primaire : le contenu chiffré avec l'ancienne
//! clé devient illisible pour soi, ce qui est le comportement attendu en cas de
//! compromission. Les autres clés du trousseau sont conservées.

use std::sync::RwLock;

use crate::crypto::{CryptoError, SymKey};

/// Gestionnaire de clés partagé entre les tâches du proxy.
pub struct KeyManager {
    /// Trousseau, jamais vide. `keys[0]` est la clé primaire (chiffrement par
    /// défaut) ; toutes les clés servent au déchiffrement.
    keys: RwLock<Vec<SymKey>>,
    /// Si vrai, le chiffrement vise TOUTES les clés (jeton `v2`
    /// multi-destinataire) au lieu de la seule clé primaire.
    encrypt_multi: bool,
}

impl KeyManager {
    pub fn new(key: SymKey) -> Self {
        Self {
            keys: RwLock::new(vec![key]),
            encrypt_multi: false,
        }
    }

    /// Construit un trousseau à partir de plusieurs clés. La première est la clé
    /// primaire. `encrypt_multi` active le chiffrement multi-destinataire (`v2`).
    /// Un trousseau vide est remplacé par une clé éphémère (jamais vide).
    pub fn from_keys(keys: Vec<SymKey>, encrypt_multi: bool) -> Self {
        let keys = if keys.is_empty() {
            vec![SymKey::generate()]
        } else {
            keys
        };
        Self {
            keys: RwLock::new(keys),
            encrypt_multi,
        }
    }

    /// Charge une clé unique depuis une chaîne base64 (32 octets).
    pub fn from_base64(s: &str) -> Result<Self, CryptoError> {
        Ok(Self::new(SymKey::from_base64(s)?))
    }

    /// Génère un trousseau à une clé éphémère aléatoire.
    pub fn ephemeral() -> Self {
        Self::new(SymKey::generate())
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Vec<SymKey>> {
        self.keys.read().expect("RwLock empoisonné")
    }

    /// Renvoie une copie de la clé primaire (compat : « la clé active »).
    pub fn current(&self) -> SymKey {
        self.read()[0].clone()
    }

    /// Toutes les clés du trousseau (déchiffrement : on les essaie toutes).
    pub fn all_keys(&self) -> Vec<SymKey> {
        self.read().clone()
    }

    /// Clés visées au CHIFFREMENT : tout le trousseau si `encrypt_multi` (jeton
    /// `v2`), sinon la seule clé primaire (jeton `v1`, compatible proxy/iOS).
    pub fn encryption_keys(&self) -> Vec<SymKey> {
        let keys = self.read();
        if self.encrypt_multi {
            keys.clone()
        } else {
            vec![keys[0].clone()]
        }
    }

    /// Nombre de clés du trousseau.
    pub fn len(&self) -> usize {
        self.read().len()
    }

    /// Toujours `false` : le trousseau est non vide par construction.
    pub fn is_empty(&self) -> bool {
        self.read().is_empty()
    }

    /// Ajoute une clé (nouveau correspondant) au trousseau, depuis du base64.
    /// Les doublons exacts sont ignorés.
    pub fn add_from_base64(&self, s: &str) -> Result<(), CryptoError> {
        let key = SymKey::from_base64(s)?;
        let b64 = key.to_base64();
        let mut keys = self.keys.write().expect("RwLock empoisonné");
        if !keys.iter().any(|k| k.to_base64() == b64) {
            keys.push(key);
        }
        Ok(())
    }

    /// Régénère la clé PRIMAIRE (rotation en cas de compromission) et renvoie sa
    /// représentation base64 partageable. Les autres clés sont conservées.
    pub fn rotate(&self) -> String {
        let fresh = SymKey::generate();
        let b64 = fresh.to_base64();
        self.keys.write().expect("RwLock empoisonné")[0] = fresh;
        b64
    }

    /// Remplace la clé PRIMAIRE par une clé partagée reçue (base64), en
    /// conservant les autres clés du trousseau.
    pub fn set_from_base64(&self, s: &str) -> Result<(), CryptoError> {
        let key = SymKey::from_base64(s)?;
        self.keys.write().expect("RwLock empoisonné")[0] = key;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_changes_active_key() {
        let km = KeyManager::ephemeral();
        let before = km.current().to_base64();
        let returned = km.rotate();
        let after = km.current().to_base64();
        assert_ne!(before, after);
        assert_eq!(returned, after);
    }

    #[test]
    fn set_from_base64_applies_shared_key() {
        let km = KeyManager::ephemeral();
        let shared = SymKey::generate().to_base64();
        km.set_from_base64(&shared).unwrap();
        assert_eq!(km.current().to_base64(), shared);
    }

    #[test]
    fn set_from_base64_rejects_bad_key() {
        let km = KeyManager::ephemeral();
        assert!(km.set_from_base64("not-base64!!").is_err());
    }

    #[test]
    fn from_keys_keeps_all_for_decryption() {
        let k1 = SymKey::generate();
        let k2 = SymKey::generate();
        let km = KeyManager::from_keys(vec![k1.clone(), k2.clone()], false);
        assert_eq!(km.len(), 2);
        // La primaire est la première.
        assert_eq!(km.current().to_base64(), k1.to_base64());
        // Toutes les clés sont disponibles au déchiffrement.
        let all: Vec<String> = km.all_keys().iter().map(|k| k.to_base64()).collect();
        assert!(all.contains(&k1.to_base64()) && all.contains(&k2.to_base64()));
    }

    #[test]
    fn add_from_base64_grows_ring_without_duplicates() {
        let km = KeyManager::ephemeral();
        let extra = SymKey::generate().to_base64();
        km.add_from_base64(&extra).unwrap();
        assert_eq!(km.len(), 2);
        // Doublon ignoré.
        km.add_from_base64(&extra).unwrap();
        assert_eq!(km.len(), 2);
    }

    #[test]
    fn encryption_keys_default_is_primary_only() {
        let km = KeyManager::from_keys(vec![SymKey::generate(), SymKey::generate()], false);
        assert_eq!(km.encryption_keys().len(), 1);
        assert_eq!(
            km.encryption_keys()[0].to_base64(),
            km.current().to_base64()
        );
    }

    #[test]
    fn encryption_keys_multi_targets_all() {
        let km = KeyManager::from_keys(vec![SymKey::generate(), SymKey::generate()], true);
        assert_eq!(km.encryption_keys().len(), 2);
    }

    #[test]
    fn from_keys_never_empty() {
        let km = KeyManager::from_keys(vec![], false);
        assert_eq!(km.len(), 1);
        assert!(!km.is_empty());
    }
}
