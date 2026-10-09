//! Textes de l'application — français, anglais, espagnol, comme le site.
//!
//! Une `struct` par langue, du même type : une traduction oubliée est une
//! erreur de compilation, pas un libellé manquant découvert à l'écran.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Fr,
    En,
    Es,
}

pub const LANGS: [Lang; 3] = [Lang::Fr, Lang::En, Lang::Es];

impl Lang {
    pub fn label(self) -> &'static str {
        match self {
            Lang::Fr => "FR",
            Lang::En => "EN",
            Lang::Es => "ES",
        }
    }

    /// Langue du système (`fr-FR`, `en_GB.UTF-8`…), français par défaut.
    pub fn detect() -> Self {
        sys_locale::get_locale()
            .as_deref()
            .and_then(Self::from_code)
            .unwrap_or_default()
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.get(..2)?.to_ascii_lowercase().as_str() {
            "fr" => Some(Lang::Fr),
            "en" => Some(Lang::En),
            "es" => Some(Lang::Es),
            _ => None,
        }
    }

    pub fn t(self) -> &'static Strings {
        match self {
            Lang::Fr => &FR,
            Lang::En => &EN,
            Lang::Es => &ES,
        }
    }
}

pub struct Strings {
    pub tagline: &'static str,
    pub tab_protection: &'static str,
    pub tab_keys: &'static str,
    pub tab_message: &'static str,

    pub on_title: &'static str,
    pub off_title: &'static str,
    pub on_desc: &'static str,
    pub off_desc: &'static str,
    pub activate: &'static str,
    pub deactivate: &'static str,
    pub waiting_auth: &'static str,
    pub row_proxy: &'static str,
    pub row_system: &'static str,
    /// `{addr}` est remplacé par l'adresse d'écoute.
    pub proxy_app: &'static str,
    pub proxy_service: &'static str,
    pub proxy_off: &'static str,
    pub system_on: &'static str,
    pub system_off: &'static str,
    pub auth_note: &'static str,
    pub no_cli: &'static str,
    pub canceled: &'static str,
    pub failed: &'static str,
    pub quitting: &'static str,
    pub quit_failed: &'static str,
    pub quit_anyway: &'static str,
    pub stay: &'static str,

    pub keys_title: &'static str,
    pub keys_intro: &'static str,
    pub keys_empty: &'static str,
    pub primary: &'static str,
    pub copy: &'static str,
    pub copied: &'static str,
    pub make_primary: &'static str,
    pub remove: &'static str,
    pub new_key: &'static str,
    pub new_key_done: &'static str,
    pub add_title: &'static str,
    pub add_hint: &'static str,
    pub add_btn: &'static str,
    pub add_done: &'static str,
    pub invalid_key: &'static str,
    pub pair_title: &'static str,
    pub pair_intro: &'static str,
    pub pair_hint: &'static str,
    pub pair_btn: &'static str,
    pub pair_wait: &'static str,
    pub pair_done: &'static str,
    /// `{path}` est remplacé par le chemin de la configuration du service.
    pub service_note: &'static str,

    pub msg_title: &'static str,
    pub msg_intro: &'static str,
    pub msg_hint: &'static str,
    pub encrypt: &'static str,
    pub decrypt: &'static str,
    pub result: &'static str,
    /// `{n}` est remplacé par le nombre de segments.
    pub encrypted_n: &'static str,
    pub decrypted_n: &'static str,
    pub nothing_marked: &'static str,
    pub nothing_readable: &'static str,
    pub no_keys: &'static str,

    pub footer: &'static str,
}

pub static FR: Strings = Strings {
    tagline: "Chiffrement sélectif de votre trafic web",
    tab_protection: "Protection",
    tab_keys: "Clés",
    tab_message: "Message",

    on_title: "Protection active",
    off_title: "Protection coupée",
    on_desc: "Ce que vous marquez ((comme ceci)) est chiffré avant de quitter votre ordinateur.",
    off_desc: "Votre trafic passe sans être modifié. Activez la protection pour chiffrer ce que vous marquez.",
    activate: "Activer la protection",
    deactivate: "Couper la protection",
    waiting_auth: "En attente de votre autorisation…",
    row_proxy: "Proxy",
    row_system: "Système",
    proxy_app: "en marche dans l'application ({addr})",
    proxy_service: "service système en marche ({addr})",
    proxy_off: "arrêté",
    system_on: "dirigé vers le proxy",
    system_off: "configuration d'origine",
    auth_note: "Activer et couper demandent le mot de passe administrateur : c'est le système qui l'exige pour faire confiance au certificat de Brocolis et régler le proxy.",
    no_cli: "La ligne de commande « brocolis » est introuvable à côté de l'application. Réinstallez l'archive complète.",
    canceled: "Autorisation refusée : rien n'a changé.",
    failed: "Échec : ",
    quitting: "Coupure de la protection avant de quitter…",
    quit_failed: "La protection n'a pas pu être coupée. En quittant maintenant, le système resterait dirigé vers un proxy arrêté et la navigation ne fonctionnerait plus.",
    quit_anyway: "Quitter quand même",
    stay: "Rester",

    keys_title: "Votre trousseau",
    keys_intro: "La clé principale chiffre ce que vous marquez. Les autres servent à lire ce qu'on vous envoie.",
    keys_empty: "Aucune clé : créez-en une, ou ajoutez celle qu'on vous a confiée.",
    primary: "principale",
    copy: "Copier",
    copied: "Copiée dans le presse-papiers.",
    make_primary: "Rendre principale",
    remove: "Retirer",
    new_key: "Nouvelle clé",
    new_key_done: "Clé créée et copiée : partagez-la par un canal sûr.",
    add_title: "Ajouter une clé reçue",
    add_hint: "Collez la clé (base64)",
    add_btn: "Ajouter",
    add_done: "Clé ajoutée au trousseau.",
    invalid_key: "Clé invalide : ",
    pair_title: "Appairer avec un code",
    pair_intro: "Convenez d'un code de vive voix, puis saisissez-le chacun de votre côté : vous obtenez la même clé sans rien recopier.",
    pair_hint: "Code convenu",
    pair_btn: "Appairer",
    pair_wait: "En attente de votre correspondant…",
    pair_done: "Appairé. Comparez cette empreinte de vive voix avant tout secret :",
    service_note: "Le service système utilise son propre trousseau ({path}). Ces clés-ci servent à l'application.",

    msg_title: "Chiffrer un message",
    msg_intro: "Marquez ((ce qui est secret)), puis chiffrez. Collez un message reçu pour le déchiffrer.",
    msg_hint: "Écrivez ou collez votre texte ici…",
    encrypt: "Chiffrer",
    decrypt: "Déchiffrer",
    result: "Résultat",
    encrypted_n: "{n} segment(s) chiffré(s).",
    decrypted_n: "{n} jeton(s) déchiffré(s).",
    nothing_marked: "Aucun segment ((marqué)) : rien à chiffrer.",
    nothing_readable: "Aucun jeton lisible avec vos clés.",
    no_keys: "Aucune clé dans le trousseau : voyez l'onglet Clés.",

    footer: "Conçu et développé par Maxime Nathan Lestage",
};

pub static EN: Strings = Strings {
    tagline: "Selective encryption of your web traffic",
    tab_protection: "Protection",
    tab_keys: "Keys",
    tab_message: "Message",

    on_title: "Protection on",
    off_title: "Protection off",
    on_desc: "What you mark ((like this)) is encrypted before it leaves your computer.",
    off_desc: "Your traffic goes through unchanged. Turn protection on to encrypt what you mark.",
    activate: "Turn protection on",
    deactivate: "Turn protection off",
    waiting_auth: "Waiting for your authorization…",
    row_proxy: "Proxy",
    row_system: "System",
    proxy_app: "running in the app ({addr})",
    proxy_service: "system service running ({addr})",
    proxy_off: "stopped",
    system_on: "routed through the proxy",
    system_off: "original settings",
    auth_note: "Turning protection on and off asks for the administrator password: the system requires it to trust the Brocolis certificate and set the proxy.",
    no_cli: "The \"brocolis\" command-line tool was not found next to the app. Reinstall the full archive.",
    canceled: "Authorization declined: nothing changed.",
    failed: "Failed: ",
    quitting: "Turning protection off before quitting…",
    quit_failed: "Protection could not be turned off. Quitting now would leave the system routed through a stopped proxy, and browsing would stop working.",
    quit_anyway: "Quit anyway",
    stay: "Stay",

    keys_title: "Your keyring",
    keys_intro: "The primary key encrypts what you mark. The others are used to read what people send you.",
    keys_empty: "No keys: create one, or add the one you were given.",
    primary: "primary",
    copy: "Copy",
    copied: "Copied to the clipboard.",
    make_primary: "Make primary",
    remove: "Remove",
    new_key: "New key",
    new_key_done: "Key created and copied: share it over a secure channel.",
    add_title: "Add a key you received",
    add_hint: "Paste the key (base64)",
    add_btn: "Add",
    add_done: "Key added to the keyring.",
    invalid_key: "Invalid key: ",
    pair_title: "Pair with a code",
    pair_intro: "Agree on a code in person, then each of you types it: you both get the same key without copying anything.",
    pair_hint: "Agreed code",
    pair_btn: "Pair",
    pair_wait: "Waiting for your contact…",
    pair_done: "Paired. Compare this fingerprint in person before sharing any secret:",
    service_note: "The system service uses its own keyring ({path}). These keys are the app's.",

    msg_title: "Encrypt a message",
    msg_intro: "Mark ((what is secret)), then encrypt. Paste a message you received to decrypt it.",
    msg_hint: "Type or paste your text here…",
    encrypt: "Encrypt",
    decrypt: "Decrypt",
    result: "Result",
    encrypted_n: "{n} segment(s) encrypted.",
    decrypted_n: "{n} token(s) decrypted.",
    nothing_marked: "No ((marked)) segment: nothing to encrypt.",
    nothing_readable: "No token your keys can read.",
    no_keys: "No key in the keyring: see the Keys tab.",

    footer: "Designed and developed by Maxime Nathan Lestage",
};

pub static ES: Strings = Strings {
    tagline: "Cifrado selectivo de tu tráfico web",
    tab_protection: "Protección",
    tab_keys: "Claves",
    tab_message: "Mensaje",

    on_title: "Protección activa",
    off_title: "Protección desactivada",
    on_desc: "Lo que marcas ((así)) se cifra antes de salir de tu ordenador.",
    off_desc: "Tu tráfico pasa sin modificarse. Activa la protección para cifrar lo que marques.",
    activate: "Activar la protección",
    deactivate: "Desactivar la protección",
    waiting_auth: "Esperando tu autorización…",
    row_proxy: "Proxy",
    row_system: "Sistema",
    proxy_app: "en marcha en la aplicación ({addr})",
    proxy_service: "servicio del sistema en marcha ({addr})",
    proxy_off: "detenido",
    system_on: "dirigido al proxy",
    system_off: "configuración original",
    auth_note: "Activar y desactivar piden la contraseña de administrador: el sistema la exige para confiar en el certificado de Brocolis y configurar el proxy.",
    no_cli: "No se encontró la herramienta de línea de comandos «brocolis» junto a la aplicación. Reinstala el archivo completo.",
    canceled: "Autorización rechazada: nada ha cambiado.",
    failed: "Error: ",
    quitting: "Desactivando la protección antes de salir…",
    quit_failed: "No se pudo desactivar la protección. Si sales ahora, el sistema seguiría dirigido a un proxy detenido y la navegación dejaría de funcionar.",
    quit_anyway: "Salir de todos modos",
    stay: "Quedarse",

    keys_title: "Tu llavero",
    keys_intro: "La clave principal cifra lo que marcas. Las demás sirven para leer lo que te envían.",
    keys_empty: "Ninguna clave: crea una o añade la que te dieron.",
    primary: "principal",
    copy: "Copiar",
    copied: "Copiada al portapapeles.",
    make_primary: "Hacer principal",
    remove: "Quitar",
    new_key: "Nueva clave",
    new_key_done: "Clave creada y copiada: compártela por un canal seguro.",
    add_title: "Añadir una clave recibida",
    add_hint: "Pega la clave (base64)",
    add_btn: "Añadir",
    add_done: "Clave añadida al llavero.",
    invalid_key: "Clave no válida: ",
    pair_title: "Emparejar con un código",
    pair_intro: "Acordad un código en persona y escribidlo cada uno por su lado: obtenéis la misma clave sin copiar nada.",
    pair_hint: "Código acordado",
    pair_btn: "Emparejar",
    pair_wait: "Esperando a tu contacto…",
    pair_done: "Emparejados. Comparad esta huella en persona antes de compartir ningún secreto:",
    service_note: "El servicio del sistema usa su propio llavero ({path}). Estas claves son las de la aplicación.",

    msg_title: "Cifrar un mensaje",
    msg_intro: "Marca ((lo que es secreto)) y cifra. Pega un mensaje recibido para descifrarlo.",
    msg_hint: "Escribe o pega tu texto aquí…",
    encrypt: "Cifrar",
    decrypt: "Descifrar",
    result: "Resultado",
    encrypted_n: "{n} segmento(s) cifrado(s).",
    decrypted_n: "{n} token(s) descifrado(s).",
    nothing_marked: "Ningún segmento ((marcado)): nada que cifrar.",
    nothing_readable: "Ningún token legible con tus claves.",
    no_keys: "Ninguna clave en el llavero: mira la pestaña Claves.",

    footer: "Diseñado y desarrollado por Maxime Nathan Lestage",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn langue_du_systeme() {
        assert_eq!(Lang::from_code("fr-FR"), Some(Lang::Fr));
        assert_eq!(Lang::from_code("en_GB.UTF-8"), Some(Lang::En));
        assert_eq!(Lang::from_code("es-419"), Some(Lang::Es));
        assert_eq!(Lang::from_code("de-DE"), None);
        assert_eq!(Lang::from_code("x"), None);
    }

    #[test]
    fn les_gabarits_portent_leurs_marqueurs() {
        for l in LANGS {
            let t = l.t();
            assert!(t.proxy_app.contains("{addr}") && t.proxy_service.contains("{addr}"));
            assert!(t.encrypted_n.contains("{n}") && t.decrypted_n.contains("{n}"));
            assert!(t.service_note.contains("{path}"));
        }
    }
}
