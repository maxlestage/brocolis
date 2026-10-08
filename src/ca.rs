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
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
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

/// Nombre maximal de certificats feuilles gardés en cache.
///
/// Chaque entrée pèse une dizaine de kilo-octets ; 1024 hôtes plafonnent donc
/// le cache aux alentours d'une dizaine de mégaoctets, tout en couvrant
/// largement les domaines d'une session de navigation normale.
const MAX_CACHED_HOSTS: usize = 1024;

/// Écrit un fichier de clé privée avec des droits restreints DÈS SA CRÉATION.
///
/// Un `std::fs::write` suivi d'un `set_permissions` laisse une fenêtre pendant
/// laquelle le fichier existe en 0644 : un autre utilisateur du poste peut le
/// lire à cet instant précis. On ouvre donc directement avec le bon mode, et on
/// resserre malgré tout ensuite au cas où le fichier préexistait avec des
/// droits trop larges (`mode` ne s'applique qu'à la création).
pub fn write_private_key(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(contents.as_bytes())?;
    f.flush()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
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
    /// Signataire des certificats feuilles : le DN, les usages et la méthode
    /// d'identifiant de clé du certificat racine, associés à sa clé privée.
    issuer: Issuer<'static, KeyPair>,
    /// PEM du certificat racine, à exporter pour le navigateur.
    root_cert_pem: String,
    /// Configurations TLS feuilles déjà générées, indexées par nom d'hôte.
    cache: Mutex<HashMap<String, Arc<ServerConfig>>>,
}

impl Ca {
    /// Charge le CA depuis `cert_path`/`key_path`, ou le génère puis le
    /// persiste si les fichiers n'existent pas.
    ///
    /// Au rechargement, le signataire est reconstruit À PARTIR DU CERTIFICAT
    /// STOCKÉ, et non de paramètres réécrits à l'identique. La différence n'est
    /// pas cosmétique : le champ « issuer » des feuilles doit correspondre au
    /// « subject » du certificat racine effectivement installé dans le
    /// navigateur. Avec des paramètres reconstruits, toute évolution ultérieure
    /// de `root_params` ferait signer les feuilles par une autorité que le
    /// navigateur ne reconnaîtrait pas — et l'interception casserait en
    /// silence, sur tous les postes où le CA avait été installé plus tôt.
    pub fn load_or_generate(cert_path: &Path, key_path: &Path) -> Result<Self, CaError> {
        if cert_path.exists() && key_path.exists() {
            let cert_pem = std::fs::read_to_string(cert_path)?;
            let key_pem = std::fs::read_to_string(key_path)?;
            let key = KeyPair::from_pem(&key_pem)?;
            let issuer = Issuer::from_ca_cert_pem(&cert_pem, key)?;
            tracing::info!("CA chargé depuis {}", cert_path.display());
            Ok(Self {
                issuer,
                root_cert_pem: cert_pem,
                cache: Mutex::new(HashMap::new()),
            })
        } else {
            let key = KeyPair::generate()?;
            let params = Self::root_params()?;
            let root_cert_pem = params.self_signed(&key)?.pem();
            let root_key_pem = key.serialize_pem();
            std::fs::write(cert_path, &root_cert_pem)?;
            // La clé du CA signe des certificats pour N'IMPORTE QUEL site que
            // le navigateur de l'utilisateur croira ensuite légitime : elle ne
            // doit jamais être lisible par un autre compte du poste.
            write_private_key(key_path, &root_key_pem)?;
            tracing::warn!(
                "Nouveau CA généré: {} / {} — installez {} dans le magasin de confiance du navigateur",
                cert_path.display(),
                key_path.display(),
                cert_path.display()
            );
            Ok(Self {
                issuer: Issuer::new(params, key),
                root_cert_pem,
                cache: Mutex::new(HashMap::new()),
            })
        }
    }

    /// Paramètres déterministes du certificat racine (DN/extensions fixes).
    fn root_params() -> Result<CertificateParams, CaError> {
        let mut params = CertificateParams::new(Vec::<String>::new())?;
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
        Ok(params)
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
        {
            let mut cache = self.cache.lock().expect("mutex CA empoisonné");
            // Le cache ne se vidait jamais : chaque hôte visité y laissait un
            // certificat et sa clé (~11 Ko mesurés). Une navigation ordinaire
            // touche des centaines de domaines tiers, et 4000 hôtes distincts
            // faisaient déjà +45 Mo jamais rendus.
            //
            // Ce cache n'est qu'une optimisation : un certificat jeté sera
            // simplement reforgé. On le vide donc en bloc au seuil, ce qui
            // borne la mémoire sans dépendre d'une structure LRU.
            if cache.len() >= MAX_CACHED_HOSTS {
                tracing::debug!(
                    "cache CA plein ({MAX_CACHED_HOSTS} hôtes) : vidé, les certificats seront reforgés"
                );
                cache.clear();
            }
            cache.insert(host.to_string(), cfg.clone());
        }
        Ok(cfg)
    }

    fn make_leaf_config(&self, host: &str) -> Result<ServerConfig, CaError> {
        ensure_crypto_provider();
        let mut params = CertificateParams::new(vec![host.to_string()])?;
        params.distinguished_name.push(DnType::CommonName, host);
        params.is_ca = IsCa::ExplicitNoCa;
        params.use_authority_key_identifier_extension = true;
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

        let leaf_key = KeyPair::generate()?;
        let leaf = params.signed_by(&leaf_key, &self.issuer)?;

        let cert_chain = pem_to_cert_chain(&leaf.pem())?;
        let key = pem_to_private_key(&leaf_key.serialize_pem())?;

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
            "é".repeat(10),               // non ASCII -> paniquait
            "aé.example.com".to_string(), // non ASCII mêlé
            "\u{0}\u{1}".to_string(),     // caractères de contrôle
            "a b\tc".to_string(),         // espaces
            String::new(),                // vide
            "a".repeat(300),              // trop long
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

    /// Le cache de certificats doit rester borné : sans plafond, chaque hôte
    /// visité y laissait un certificat et sa clé, sans jamais rien libérer.
    #[test]
    fn le_cache_de_certificats_reste_borne() {
        let dir = std::env::temp_dir().join(format!("brocolis-ca-cache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ca = Ca::load_or_generate(&dir.join("ca.crt"), &dir.join("ca.key")).unwrap();

        for i in 0..(MAX_CACHED_HOSTS + 50) {
            ca.server_config_for(&format!("h{i}.example.com")).unwrap();
        }
        let taille = ca.cache.lock().unwrap().len();
        assert!(
            taille <= MAX_CACHED_HOSTS,
            "cache non borné : {taille} entrées pour un plafond de {MAX_CACHED_HOSTS}"
        );
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

    /// Fait circuler les octets TLS en attente d'un bout à l'autre.
    fn transmettre(
        de: &mut rustls::Connection,
        vers: &mut rustls::Connection,
    ) -> Result<(), rustls::Error> {
        let mut tampon = Vec::new();
        while de.wants_write() {
            de.write_tls(&mut tampon).expect("écriture en mémoire");
        }
        let mut reste = &tampon[..];
        while !reste.is_empty() {
            vers.read_tls(&mut reste).expect("lecture en mémoire");
            vers.process_new_packets()?;
        }
        Ok(())
    }

    /// Négocie TLS entre un serveur utilisant `config` et un client qui ne fait
    /// confiance qu'à `racines_pem`. Renvoie l'erreur du client, s'il refuse.
    fn poignee_de_main(
        config: Arc<ServerConfig>,
        racines_pem: &str,
        hote: &'static str,
    ) -> Result<(), rustls::Error> {
        let mut racines = rustls::RootCertStore::empty();
        for certificat in pem_to_cert_chain(racines_pem).expect("PEM de racine lisible") {
            racines.add(certificat).expect("racine acceptable");
        }
        let client_config = Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(racines)
                .with_no_client_auth(),
        );

        let mut serveur: rustls::Connection = rustls::ServerConnection::new(config)
            .expect("configuration serveur valide")
            .into();
        let mut client: rustls::Connection =
            rustls::ClientConnection::new(client_config, hote.try_into().expect("nom d'hôte"))
                .expect("configuration client valide")
                .into();

        // Deux allers-retours suffisent largement à TLS 1.3 ; la borne évite
        // simplement une boucle infinie si la négociation se bloquait.
        for _ in 0..8 {
            if !client.is_handshaking() && !serveur.is_handshaking() {
                return Ok(());
            }
            transmettre(&mut client, &mut serveur)?;
            transmettre(&mut serveur, &mut client)?;
        }
        Ok(())
    }

    /// Un certificat feuille doit être accepté par un client qui ne fait
    /// confiance qu'au certificat racine STOCKÉ SUR DISQUE — même quand ce
    /// certificat ne ressemble pas à ce que `root_params` produirait
    /// aujourd'hui.
    ///
    /// C'est exactement le scénario d'un poste où le CA a été installé par une
    /// version antérieure : le fichier sur disque fait foi, pas le code. Si le
    /// signataire était reconstruit depuis `root_params`, les feuilles
    /// porteraient un émetteur inconnu du navigateur et l'interception
    /// casserait en silence. Ce test refait la poignée de main TLS complète,
    /// comme le ferait un navigateur.
    #[test]
    fn une_feuille_chaine_au_ca_stocke_meme_si_ses_parametres_ont_change() {
        use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose};

        ensure_crypto_provider();
        let dir = std::env::temp_dir().join(format!("brocolis-ca-chaine-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cert = dir.join("ca.crt");
        let key = dir.join("ca.key");

        // CA « d'une version antérieure » : même rôle, mais un DN que
        // `root_params` ne produit pas.
        let mut anciens = CertificateParams::new(Vec::<String>::new()).unwrap();
        anciens
            .distinguished_name
            .push(DnType::CommonName, "Brocolis Root CA (ancienne version)");
        anciens
            .distinguished_name
            .push(DnType::OrganizationName, "Brocolis d'avant");
        anciens.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        anciens.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let ancienne_cle = KeyPair::generate().unwrap();
        let racine_pem = anciens.self_signed(&ancienne_cle).unwrap().pem();
        std::fs::write(&cert, &racine_pem).unwrap();
        std::fs::write(&key, ancienne_cle.serialize_pem()).unwrap();

        // Rechargement par le code d'aujourd'hui, puis forge d'une feuille.
        let ca = Ca::load_or_generate(&cert, &key).unwrap();
        assert_eq!(
            ca.root_cert_pem(),
            racine_pem,
            "le CA exporté doit rester celui du disque"
        );
        let feuille = ca.server_config_for("exemple.test").unwrap();

        poignee_de_main(feuille.clone(), &racine_pem, "exemple.test")
            .expect("la feuille doit chaîner jusqu'au certificat racine stocké");

        // Contre-épreuve : face à une autre autorité, la même poignée de main
        // doit échouer. Sans elle, un test qui accepte tout passerait aussi.
        let autre_dir = dir.join("autre");
        std::fs::create_dir_all(&autre_dir).unwrap();
        let autre_racine =
            Ca::load_or_generate(&autre_dir.join("ca.crt"), &autre_dir.join("ca.key"))
                .unwrap()
                .root_cert_pem()
                .to_string();
        let refus = poignee_de_main(feuille, &autre_racine, "exemple.test");
        assert!(
            matches!(refus, Err(rustls::Error::InvalidCertificate(_))),
            "une autorité inconnue aurait dû être refusée, obtenu : {refus:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
