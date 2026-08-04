//! Autorité de certification interne.
//!
//! Brocolis génère (au premier lancement) un certificat racine auto-signé,
//! puis forge à la volée un certificat feuille par hôte, signé par ce CA.
//! Pour que l'interception HTTPS soit transparente, le client doit faire
//! confiance à ce certificat racine. Sur macOS, `brocolis auto` s'en charge
//! automatiquement (trousseau Système → Safari/Chrome ; politique entreprise
//! → Firefox) et restaure l'état initial à l'arrêt.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, Once};

use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyUsagePurpose,
    ExtendedKeyUsagePurpose, KeyPair,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;

#[derive(Debug, thiserror::Error)]
pub enum CaError {
    #[error("erreur rcgen: {0}")]
    Rcgen(#[from] rcgen::Error),
    #[error("erreur d'entrée/sortie: {0}")]
    Io(#[from] std::io::Error),
    #[error("erreur rustls: {0}")]
    Rustls(#[from] rustls::Error),
    #[error("PEM invalide: {0}")]
    Pem(String),
    #[error("nom d'hôte invalide pour un certificat: {0}")]
    BadHost(String),
}

/// Un nom d'hôte est-il utilisable tel quel dans un certificat ?
///
/// Le SAN dNSName est encodé en `IA5String`, qui n'accepte QUE de l'ASCII : un
/// hôte contenant un octet non ASCII faisait paniquer l'encodeur ASN.1 en
/// profondeur (yasna), depuis une simple requête `CONNECT`. Les vrais clients
/// envoient déjà les noms internationalisés en punycode ; on refuse donc
/// proprement le reste au lieu de laisser paniquer.
fn is_encodable_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host.is_ascii()
        // Pas de caractère de contrôle ni d'espace : ils n'ont rien à faire
        // dans un nom d'hôte et brouilleraient le certificat.
        && !host
            .chars()
            .any(|c| c.is_ascii_control() || c == ' ' || c == '\t')
}

/// Autorité de certification racine + cache de configurations TLS par hôte.
pub struct Ca {
    /// Certificat racine (rechargé en mémoire pour signer les feuilles).
    root: Certificate,
    /// PEM du certificat racine, à exporter pour le navigateur.
    root_cert_pem: String,
    /// Configurations TLS feuilles déjà générées, indexées par nom d'hôte.
    cache: Mutex<HashMap<String, Arc<ServerConfig>>>,
}

impl Ca {
    /// Charge le CA depuis `cert_path`/`key_path`, ou le génère puis le
    /// persiste si les fichiers n'existent pas.
    ///
    /// rcgen 0.12 ne sait pas reconstruire un signataire depuis un PEM ; on
    /// reconstruit donc des paramètres CA *déterministes* (mêmes DN/extensions)
    /// associés à la clé privée rechargée. Le certificat racine envoyé au
    /// navigateur reste celui stocké sur disque.
    pub fn load_or_generate(cert_path: &Path, key_path: &Path) -> Result<Self, CaError> {
        if cert_path.exists() && key_path.exists() {
            let cert_pem = std::fs::read_to_string(cert_path)?;
            let key_pem = std::fs::read_to_string(key_path)?;
            let key = KeyPair::from_pem(&key_pem)?;
            let mut params = Self::root_params();
            params.key_pair = Some(key);
            let root = Certificate::from_params(params)?;
            tracing::info!("CA chargé depuis {}", cert_path.display());
            Ok(Self {
                root,
                root_cert_pem: cert_pem,
                cache: Mutex::new(HashMap::new()),
            })
        } else {
            let root = Certificate::from_params(Self::root_params())?;
            let root_cert_pem = root.serialize_pem()?;
            let root_key_pem = root.serialize_private_key_pem();
            std::fs::write(cert_path, &root_cert_pem)?;
            std::fs::write(key_path, &root_key_pem)?;
            tracing::warn!(
                "Nouveau CA généré: {} / {} — installez {} dans le magasin de confiance du navigateur",
                cert_path.display(),
                key_path.display(),
                cert_path.display()
            );
            Ok(Self {
                root,
                root_cert_pem,
                cache: Mutex::new(HashMap::new()),
            })
        }
    }

    /// Paramètres déterministes du certificat racine (DN/extensions fixes).
    fn root_params() -> CertificateParams {
        let mut params = CertificateParams::new(Vec::<String>::new());
        params
            .distinguished_name
            .push(DnType::CommonName, "Brocolis Root CA");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "Brocolis");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        params
    }

    /// PEM du certificat racine (à fournir au navigateur).
    pub fn root_cert_pem(&self) -> &str {
        &self.root_cert_pem
    }

    /// Renvoie une configuration TLS serveur pour `host`, en réutilisant le
    /// cache si le certificat feuille a déjà été forgé.
    pub fn server_config_for(&self, host: &str) -> Result<Arc<ServerConfig>, CaError> {
        // Validé AVANT le cache : un hôte impossible ne doit ni paniquer plus
        // bas, ni occuper une entrée de cache.
        if !is_encodable_host(host) {
            return Err(CaError::BadHost(host.chars().take(60).collect()));
        }
        if let Some(cfg) = self.cache.lock().expect("mutex CA empoisonné").get(host) {
            return Ok(cfg.clone());
        }
        let cfg = Arc::new(self.make_leaf_config(host)?);
        self.cache
            .lock()
            .expect("mutex CA empoisonné")
            .insert(host.to_string(), cfg.clone());
        Ok(cfg)
    }

    fn make_leaf_config(&self, host: &str) -> Result<ServerConfig, CaError> {
        ensure_crypto_provider();
        let mut params = CertificateParams::new(vec![host.to_string()]);
        params.distinguished_name.push(DnType::CommonName, host);
        params.is_ca = IsCa::ExplicitNoCa;
        params.use_authority_key_identifier_extension = true;
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

        let leaf = Certificate::from_params(params)?;
        let leaf_cert_pem = leaf.serialize_pem_with_signer(&self.root)?;
        let leaf_key_pem = leaf.serialize_private_key_pem();

        let cert_chain = pem_to_cert_chain(&leaf_cert_pem)?;
        let key = pem_to_private_key(&leaf_key_pem)?;

        let mut cfg = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)?;
        cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(cfg)
    }
}

/// rustls 0.23 exige un fournisseur cryptographique installé au niveau du
/// processus. Plusieurs fournisseurs cohabitent dans l'arbre de dépendances
/// (reqwest, tokio-rustls), donc on l'installe explicitement une seule fois.
pub fn ensure_crypto_provider() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

fn pem_to_cert_chain(pem: &str) -> Result<Vec<CertificateDer<'static>>, CaError> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CaError::Pem(e.to_string()))?;
    if certs.is_empty() {
        return Err(CaError::Pem("aucun certificat dans le PEM".into()));
    }
    Ok(certs)
}

fn pem_to_private_key(pem: &str) -> Result<PrivateKeyDer<'static>, CaError> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| CaError::Pem(e.to_string()))?
        .ok_or_else(|| CaError::Pem("clé privée absente du PEM".into()))
}

#[cfg(test)]
mod tests_hote_hostile {
    use super::*;

    /// Un nom d'hôte non ASCII faisait paniquer l'encodeur ASN.1 (IA5String)
    /// depuis une simple requête CONNECT. Il doit maintenant produire une
    /// erreur propre.
    #[test]
    fn un_hote_hostile_ne_fait_plus_paniquer() {
        let dir = std::env::temp_dir().join(format!("brocolis-ca-hostile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ca = Ca::load_or_generate(&dir.join("ca.crt"), &dir.join("ca.key")).unwrap();

        for hote in [
            "é".repeat(10),                 // non ASCII -> paniquait
            "aé.example.com".to_string(),   // non ASCII mêlé
            "\u{0}\u{1}".to_string(),       // caractères de contrôle
            "a b\tc".to_string(),           // espaces
            String::new(),                  // vide
            "a".repeat(300),                // trop long
        ] {
            let r = ca.server_config_for(&hote);
            assert!(r.is_err(), "hôte {hote:?} devrait être refusé proprement");
        }

        // Les hôtes légitimes continuent de fonctionner, punycode compris.
        for hote in ["example.com", "xn--caf-dma.example.com", "127.0.0.1"] {
            assert!(
                ca.server_config_for(hote).is_ok(),
                "hôte légitime refusé : {hote}"
            );
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_and_signs_leaf() {
        let dir = std::env::temp_dir().join(format!("brocolis-ca-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cert = dir.join("ca.crt");
        let key = dir.join("ca.key");
        let _ = std::fs::remove_file(&cert);
        let _ = std::fs::remove_file(&key);

        let ca = Ca::load_or_generate(&cert, &key).unwrap();
        assert!(ca.root_cert_pem().contains("BEGIN CERTIFICATE"));
        assert!(cert.exists() && key.exists());

        // Première génération + cache.
        let c1 = ca.server_config_for("twitter.com").unwrap();
        let c2 = ca.server_config_for("twitter.com").unwrap();
        assert!(Arc::ptr_eq(&c1, &c2));

        // Rechargement depuis le disque.
        let ca2 = Ca::load_or_generate(&cert, &key).unwrap();
        ca2.server_config_for("example.com").unwrap();

        std::fs::remove_dir_all(&dir).ok();
    }
}
