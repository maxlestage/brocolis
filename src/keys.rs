//! Gestion des clés symétriques en mémoire.
//!
//! Brocolis garde la clé active en mémoire (jamais sur disque sauf si
//! l'utilisateur la place explicitement dans la configuration). La rotation
//! remplace la clé active : le contenu chiffré avec l'ancienne clé devient
//! illisible, ce qui est le comportement attendu en cas de compromission.

use std::sync::RwLock;

use crate::crypto::{CryptoError, SymKey};

/// Gestionnaire de clés partagé entre les tâches du proxy.
pub struct KeyManager {
    active: RwLock<SymKey>,
}

impl KeyManager {
    pub fn new(key: SymKey) -> Self {
        Self {
            active: RwLock::new(key),
        }
    }

    /// Charge la clé depuis une chaîne base64 (32 octets).
    pub fn from_base64(s: &str) -> Result<Self, CryptoError> {
        Ok(Self::new(SymKey::from_base64(s)?))
    }

    /// Génère une clé éphémère aléatoire.
    pub fn ephemeral() -> Self {
        Self::new(SymKey::generate())
    }

    /// Renvoie une copie de la clé active courante.
    pub fn current(&self) -> SymKey {
        self.active.read().expect("RwLock empoisonné").clone()
    }

    /// Régénère la clé active (rotation en cas de compromission) et renvoie sa
    /// représentation base64 partageable.
    pub fn rotate(&self) -> String {
        let fresh = SymKey::generate();
        let b64 = fresh.to_base64();
        *self.active.write().expect("RwLock empoisonné") = fresh;
        b64
    }

    /// Remplace la clé active par une clé partagée (reçue d'un autre
    /// utilisateur), au format base64.
    pub fn set_from_base64(&self, s: &str) -> Result<(), CryptoError> {
        let key = SymKey::from_base64(s)?;
        *self.active.write().expect("RwLock empoisonné") = key;
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
}
