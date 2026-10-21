//! Le protocole v3 sur disque : l'appareil et l'état gardés à côté de
//! `brocolis.toml`, et une transaction à la fois (§8.1–8.3 de la sécurité de
//! la messagerie).
//!
//! ```text
//! <dossier de brocolis.toml>/v3/
//!     appareil.json   l'appareil (identité, SPK, PQPK, mémoire des premiers messages)
//!     etat.json       sessions et messages ouverts
//!     verrou          verrou exclusif d'une transaction
//! ```
//!
//! Le proxy, la ligne de commande et l'application peuvent écrire le même
//! état en même temps. Un message ne doit **jamais** être chiffré deux fois
//! avec la même clé de message : toute opération est donc une transaction
//! ([`V3Store::transaction`]) — verrou exclusif (entre fils comme entre
//! processus), lecture, opération, écriture de l'appareil **puis** de l'état
//! (fichier temporaire renommé : jamais à moitié écrit), et seulement alors
//! le jeton est rendu. Si une écriture échoue, l'opération échoue.
//!
//! Comme `brocolis.toml` et la clé d'appairage X25519, les fichiers ne sont
//! lisibles que par leur propriétaire (`0600`, dossier `0700` sous Unix) :
//! c'est ce qui est local à la plateforme (§8.11). Un fichier illisible est
//! remplacé par un état vide : les sessions se rouvrent.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use brocolis_proto::KeyBundle;

use crate::config::{same_name, Config};
use crate::crypto::SymKey;
use crate::v3::{Engine, KeyInfo, Protection};

/// L'heure, en secondes Unix (le protocole la reçoit de l'appelant).
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// L'appareil et l'état v3 rangés dans un dossier.
pub struct V3Store {
    dir: PathBuf,
    /// Les fils d'un même processus passent un par un : le verrou de fichier
    /// les exclurait aussi, mais en bloquant chacun un fil du proxy.
    local: Mutex<()>,
}

impl V3Store {
    /// L'état v3 d'un dossier.
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            local: Mutex::new(()),
        }
    }

    /// L'état v3 qui va avec un fichier de configuration : `v3/` à côté.
    pub fn for_config(config: &Path) -> Self {
        let parent = config
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        Self::at(parent.join("v3"))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Une transaction (§8.3) : `work` voit l'appareil et l'état, ce qu'il a
    /// changé est écrit avant de rendre son résultat.
    pub fn transaction<R>(&self, work: impl FnOnce(&mut Engine, u64) -> R) -> anyhow::Result<R> {
        let _local = self.local.lock().unwrap_or_else(|e| e.into_inner());
        create_private_dir(&self.dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.dir.join("verrou"))?;
        lock.lock()?;
        let now = now();
        let device = read_optional(&self.dir.join("appareil.json"))?;
        let state = read_optional(&self.dir.join("etat.json"))?;
        let mut engine = Engine::load(device.as_deref(), state.as_deref(), now);
        let out = work(&mut engine, now);
        let dirty = engine.take_dirty();
        // L'appareil d'abord : sa mémoire des premiers messages n'est jamais
        // en retard sur les sessions (un premier message rejoué reste refusé).
        if let Some(d) = dirty.device {
            write_private(&self.dir.join("appareil.json"), d.as_bytes())?;
        }
        if let Some(s) = dirty.state {
            write_private(&self.dir.join("etat.json"), s.as_bytes())?;
        }
        drop(lock);
        Ok(out)
    }

    /// Le paquet de clés publiques à publier à l'appairage (§8.4), base64.
    /// L'appareil est créé au premier besoin.
    pub fn bundle_b64(&self) -> anyhow::Result<String> {
        self.transaction(|e, _| e.bundle_b64())?
            .ok_or_else(|| anyhow::anyhow!("paquet de clés v3 indisponible"))
    }

    /// Mon identité v3 : IK X25519 ‖ IK Ed25519 (empreinte d'appairage).
    pub fn identity(&self) -> anyhow::Result<[u8; 64]> {
        self.transaction(|e, _| e.identity())
    }
}

fn read_optional(path: &Path) -> anyhow::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        // Illisible (non UTF-8…) : repartir d'un état vide, comme un fichier
        // abîmé. Un refus d'accès, lui, est une vraie erreur.
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    let mut b = std::fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b.create(dir)
}

/// Écrit dans un fichier temporaire, puis le renomme : le fichier n'est
/// jamais à moitié écrit.
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut o = OpenOptions::new();
    o.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let mut f: File = o.open(&tmp)?;
    f.write_all(data)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)
}

// ---------------------------------------------------------------------------
// Les clés de la configuration, telles que le moteur les voit
// ---------------------------------------------------------------------------

/// Une clé de la configuration, avec ce que le v3 en sait.
#[derive(Clone)]
pub struct Recipient {
    /// Le nom du contact ; `None` pour une clé sans nom.
    pub name: Option<String>,
    /// Où ranger les sessions à clé partagée (PSK) de cette clé : un
    /// identifiant dérivé de la clé, local à cet appareil.
    pub id: String,
    pub key: SymKey,
    raw: [u8; 32],
    /// Le paquet du correspondant reçu à l'appairage.
    pub bundle: Option<KeyBundle>,
    pub bundle_at: u64,
    /// Une clé de groupe (3 personnes ou plus) : jamais de v3 (§7.4).
    pub group: bool,
    /// Marquée « à deux, double cliquet » (PSK) par l'utilisateur.
    pub psk: bool,
}

impl Recipient {
    pub fn info(&self) -> KeyInfo<'_> {
        KeyInfo {
            id: &self.id,
            key: self.raw,
            bundle: self.bundle.as_ref(),
            bundle_at: self.bundle_at,
            group: self.group,
            psk: self.psk,
        }
    }
}

fn key_id(key: &SymKey) -> String {
    crate::agreement::group_fingerprint(&key.to_base64())
}

/// Toutes les clés de la configuration, contacts d'abord.
///
/// - Un contact est « à deux » sauf s'il est marqué `duo = false` (un
///   appairage de groupe) ou si sa clé sert aussi à un autre contact ou à une
///   clé sans nom : une clé connue de trois personnes reste en v1/v2.
/// - Une clé sans nom est traitée comme une clé de groupe : on ne sait pas
///   qui la détient (`pair-group`, clé collée…). Pour passer en v3, on en
///   fait un contact.
pub fn recipients(cfg: &Config) -> Vec<Recipient> {
    let mut out: Vec<Recipient> = Vec::new();
    let unnamed: Vec<String> = cfg
        .key
        .iter()
        .chain(cfg.keys.iter())
        .map(|k| k.trim().to_owned())
        .filter(|k| !k.is_empty())
        .collect();
    for c in &cfg.contacts {
        let Some(b64) = c.key() else { continue };
        let Ok(key) = SymKey::from_base64(b64) else {
            continue;
        };
        let shared = unnamed.iter().any(|k| k == b64)
            || cfg
                .contacts
                .iter()
                .any(|o| !same_name(&o.name, &c.name) && o.key() == Some(b64));
        out.push(Recipient {
            name: Some(c.name.clone()),
            id: key_id(&key),
            raw: key.bytes(),
            bundle: c
                .bundle
                .as_deref()
                .and_then(|b| KeyBundle::from_base64(b.trim()).ok()),
            bundle_at: c.bundle_at.unwrap_or(0),
            group: c.duo == Some(false) || shared,
            psk: c.psk,
            key,
        });
    }
    for b64 in unnamed {
        let Ok(key) = SymKey::from_base64(&b64) else {
            continue;
        };
        if out.iter().any(|r| r.key.to_base64() == key.to_base64()) {
            continue;
        }
        out.push(Recipient {
            name: None,
            id: key_id(&key),
            raw: key.bytes(),
            bundle: None,
            bundle_at: 0,
            group: true,
            psk: false,
            key,
        });
    }
    out
}

/// Le contact de ce nom parmi `all`.
pub fn find<'a>(all: &'a [Recipient], name: &str) -> Option<&'a Recipient> {
    all.iter()
        .find(|r| r.name.as_deref().is_some_and(|n| same_name(n, name)))
}

/// La protection de chaque contact, telle que le prochain envoi la donnera
/// (§8.8). Le v3 indisponible : tout est en ancien format.
pub fn protections(store: &V3Store, all: &[Recipient]) -> Vec<Protection> {
    store
        .transaction(|e, now| all.iter().map(|r| e.protection(&r.info(), now)).collect())
        .unwrap_or_else(|_| vec![Protection::Legacy; all.len()])
}

/// Ce qui scelle un texte en jeton ; `None` : échec fermé.
type Sealer<'a> = dyn FnMut(&str) -> Option<String> + 'a;

// ---------------------------------------------------------------------------
// Chiffrer et déchiffrer : le v3 quand il le peut, v1/v2 sinon (§8.5, §8.6)
// ---------------------------------------------------------------------------

/// Le trousseau complet : les clés de la configuration et l'état v3. C'est
/// ce que le proxy, la ligne de commande et l'application utilisent pour
/// chiffrer et déchiffrer.
pub struct V3 {
    pub store: V3Store,
    pub recipients: Vec<Recipient>,
}

/// Ce qu'une lecture a trouvé.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Read {
    pub text: String,
    /// Jetons ouverts (toutes versions).
    pub opened: usize,
    /// Jetons v3 déjà lus sur cet appareil dont le clair n'est plus gardé.
    pub replayed: usize,
    /// Les contacts qui ont écrit les jetons v3 ouverts, et `true` si un de
    /// ces jetons est un de mes envois.
    pub v3_from: Vec<String>,
    pub v3_mine: bool,
}

impl V3 {
    /// Le trousseau d'une configuration et de son fichier.
    pub fn for_config(cfg: &Config, config_path: &Path) -> Self {
        Self {
            store: V3Store::for_config(config_path),
            recipients: recipients(cfg),
        }
    }

    /// La clé à deux qui recevrait un envoi pour ces clés, s'il n'y en a
    /// qu'une : le v3 ne vise jamais qu'**une** clé (§8.5).
    fn single(&self, targets: &[SymKey]) -> Option<&Recipient> {
        let [only] = targets else { return None };
        let raw = only.bytes();
        self.recipients
            .iter()
            .find(|r| r.raw == raw)
            .filter(|r| !r.group)
    }

    /// Ce que donnerait un envoi pour ces clés (§8.8).
    pub fn protection(&self, targets: &[SymKey]) -> Protection {
        match self.single(targets) {
            Some(r) => self
                .store
                .transaction(|e, now| e.protection(&r.info(), now))
                .unwrap_or(Protection::Legacy),
            None => Protection::Legacy,
        }
    }

    /// Fait tourner `op` avec un scelleur : v3 pour la clé à deux, v1/v2
    /// sinon. Le v3 indisponible (état illisible, écriture impossible) : tout
    /// repart en v1/v2, et aucun jeton v3 n'est rendu sans son état écrit.
    fn with_sealer<R>(&self, targets: &[SymKey], op: &mut dyn FnMut(&mut Sealer<'_>) -> R) -> R {
        let legacy = |text: &str| crate::crypto::seal_token(text.as_bytes(), targets);
        if let Some(r) = self.single(targets) {
            let info = r.info();
            let done = self.store.transaction(|e, now| {
                op(&mut |text: &str| e.seal(&info, text, now).or_else(|| legacy(text)))
            });
            match done {
                Ok(out) => return out,
                Err(err) => tracing::warn!("v3 indisponible ({err}) : envoi en v1"),
            }
        }
        op(&mut |text: &str| legacy(text))
    }

    /// [`crate::crypto::encrypt_message`], en v3 quand c'est possible.
    pub fn encrypt_message(
        &self,
        input: &str,
        targets: &[SymKey],
    ) -> Result<(String, crate::crypto::Sealed), crate::crypto::CryptoError> {
        if targets.is_empty() {
            return Err(crate::crypto::CryptoError::NoKey);
        }
        self.with_sealer(targets, &mut |seal| {
            crate::crypto::encrypt_message_with(input, seal)
        })
    }

    /// [`crate::crypto::encrypt_marked_multi`], en v3 quand c'est possible.
    pub fn encrypt_marked(&self, input: &str, targets: &[SymKey]) -> (String, usize) {
        self.with_sealer(targets, &mut |seal| {
            crate::crypto::encrypt_marked_with(input, seal)
        })
    }

    /// [`crate::crypto::encrypt_marked_form`], en v3 quand c'est possible.
    pub fn encrypt_marked_form(&self, body: &str, targets: &[SymKey]) -> (String, usize) {
        self.with_sealer(targets, &mut |seal| {
            crate::crypto::encrypt_marked_form_with(body, seal)
        })
    }

    /// Déchiffre tous les jetons : v1/v2 avec `keys`, v3 avec l'état (le
    /// clair gardé, une session, un premier message d'un contact ou en PSK).
    pub fn read(&self, input: &str, keys: &[SymKey], escape: crate::crypto::PlainEscape) -> Read {
        if !input.contains(brocolis_proto::jeton_v3::HEAD) {
            let (text, opened) = crate::crypto::decrypt_tokens_escaped(input, keys, escape);
            return Read {
                text,
                opened,
                ..Read::default()
            };
        }
        let infos: Vec<KeyInfo<'_>> = self.recipients.iter().map(Recipient::info).collect();
        let mut replayed = 0;
        let mut from: Vec<String> = Vec::new();
        let mut mine = false;
        let done = self.store.transaction(|e, now| {
            crate::crypto::decrypt_tokens_with(input, keys, escape, &mut |token| match e
                .open(token, &infos, now)
            {
                crate::v3::Opened::Text(t) => {
                    if e.is_mine(token) {
                        mine = true;
                    } else if let Some(name) = e
                        .sender(token, &infos)
                        .and_then(|i| self.recipients[i].name.clone())
                    {
                        if !from.contains(&name) {
                            from.push(name);
                        }
                    }
                    Some(t)
                }
                crate::v3::Opened::Replay => {
                    replayed += 1;
                    None
                }
                crate::v3::Opened::Failed(_) => None,
            })
        });
        let (text, opened) = done.unwrap_or_else(|err| {
            tracing::warn!("v3 indisponible ({err}) : jetons v3 laissés chiffrés");
            crate::crypto::decrypt_tokens_escaped(input, keys, escape)
        });
        Read {
            text,
            opened,
            replayed,
            v3_from: from,
            v3_mine: mine,
        }
    }

    /// Déchiffre (texte brut) : [`V3::read`] sans échappement.
    pub fn decrypt(&self, input: &str, keys: &[SymKey]) -> Read {
        self.read(input, keys, crate::crypto::PlainEscape::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("brocolis-v3store-{}-{nom}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn la_transaction_ecrit_l_appareil_puis_le_relit() {
        let dir = tmp("relit");
        let store = V3Store::at(&dir);
        let a = store.identity().unwrap();
        assert!(dir.join("appareil.json").exists(), "créé au premier besoin");
        assert_eq!(store.identity().unwrap(), a, "le même appareil, relu");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("appareil.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "lisible par son seul propriétaire");
        }
        let b = KeyBundle::from_base64(&store.bundle_b64().unwrap()).unwrap();
        assert!(b.pq.is_some(), "paquet hybride ML-KEM-768");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_etat_abime_repart_de_zero() {
        let dir = tmp("abime");
        let store = V3Store::at(&dir);
        let a = store.identity().unwrap();
        std::fs::write(dir.join("etat.json"), "{pas du json").unwrap();
        assert_eq!(store.identity().unwrap(), a, "l'appareil reste");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Deux trousseaux appairés, comme le proxy les construit.
    fn duo(nom: &str) -> (V3, V3, SymKey) {
        let (da, db) = (tmp(&format!("{nom}-a")), tmp(&format!("{nom}-b")));
        let (sa, sb) = (V3Store::at(&da), V3Store::at(&db));
        let (pa, pb) = (sa.bundle_b64().unwrap(), sb.bundle_b64().unwrap());
        let key = SymKey::generate();
        let k = key.to_base64();
        let conf = |nom: &str, paquet: &str| -> Config {
            let t = crate::contacts::add_paired("", nom, &k, paquet, now()).unwrap();
            toml::from_str(&t).unwrap()
        };
        let a = V3 {
            store: sa,
            recipients: recipients(&conf("Bob", &pb)),
        };
        let b = V3 {
            store: sb,
            recipients: recipients(&conf("Alice", &pa)),
        };
        (a, b, key)
    }

    #[test]
    fn le_proxy_chiffre_un_formulaire_en_v3_et_echappe_le_clair() {
        use crate::crypto::PlainEscape;
        let (a, b, key) = duo("proxy");
        let targets = [key.clone()];
        assert_eq!(a.protection(&targets), Protection::PostQuantum);
        let (body, n) = a.encrypt_marked_form("t=%28%28%3Cb%3Ehi%3C%2Fb%3E%29%29&x=1", &targets);
        assert_eq!(n, 1);
        assert!(body.contains("BROCOLIS%3Av3%3A"), "{body}");
        assert!(body.ends_with("&x=1"));
        let (text, n) = a.encrypt_marked("<p>((secret))</p>", &targets);
        assert_eq!(n, 1);
        let page = format!("<div>{text}</div>");
        let read = b.read(&page, std::slice::from_ref(&key), PlainEscape::Html);
        assert_eq!(read.text, "<div><p>secret</p></div>");
        assert_eq!(read.v3_from, ["Alice"]);
        // La page rechargée : le clair gardé, sans rien déchiffrer.
        assert_eq!(
            b.read(&page, std::slice::from_ref(&key), PlainEscape::Html)
                .opened,
            1
        );
        // Mon propre envoi, relu dans la page.
        let mine = a.read(&page, &[key], PlainEscape::Html);
        assert!(mine.v3_mine && mine.text == "<div><p>secret</p></div>");
        // Un clair hostile est échappé, comme en v1.
        let (t, _) = a.encrypt_marked("((<script>x</script>))", &targets);
        let r = b.read(&t, &[], PlainEscape::Html);
        assert_eq!(r.text, "&lt;script&gt;x&lt;/script&gt;");
        for v in [a, b] {
            let _ = std::fs::remove_dir_all(v.store.dir());
        }
    }

    #[test]
    fn plusieurs_destinataires_ou_v3_indisponible_partent_en_v1() {
        let (a, b, key) = duo("repli");
        let other = SymKey::generate();
        let (t, _) = a.encrypt_marked("((x))", &[key.clone(), other]);
        assert!(t.starts_with("[[BROCOLIS:v2:"), "plusieurs clés : v2");
        // État impossible à écrire : le dossier v3 est un fichier.
        let dir = a.store.dir().to_path_buf();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::write(&dir, "pas un dossier").unwrap();
        let (t, n) = a.encrypt_marked("((y))", std::slice::from_ref(&key));
        assert_eq!(n, 1);
        assert!(t.starts_with("[[BROCOLIS:v1:"), "{t}");
        assert_eq!(crate::crypto::decrypt_tokens(&t, &key).0, "y");
        let _ = std::fs::remove_file(&dir);
        let _ = std::fs::remove_dir_all(b.store.dir());
    }

    #[test]
    fn deux_contacts_qui_partagent_une_cle_sont_un_groupe() {
        let k = SymKey::generate().to_base64();
        let seul = SymKey::generate().to_base64();
        let toml = format!(
            "key = \"{}\"\n\
             [[contacts]]\nname = \"A\"\nkey = \"{k}\"\n\
             [[contacts]]\nname = \"B\"\nkey = \"{k}\"\n\
             [[contacts]]\nname = \"C\"\nkey = \"{seul}\"\n\
             [[contacts]]\nname = \"D\"\nkey = \"{}\"\nduo = false\n",
            SymKey::generate().to_base64(),
            SymKey::generate().to_base64(),
        );
        let cfg: Config = toml::from_str(&toml).unwrap();
        let all = recipients(&cfg);
        assert_eq!(all.len(), 5, "quatre contacts, une clé sans nom");
        assert!(find(&all, "a").unwrap().group);
        assert!(find(&all, "B").unwrap().group);
        assert!(!find(&all, "C").unwrap().group, "à deux");
        assert!(find(&all, "D").unwrap().group, "appairage de groupe");
        assert!(all[4].name.is_none() && all[4].group, "clé sans nom");
    }
}
