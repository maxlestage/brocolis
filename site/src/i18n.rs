//! Internationalisation — français, anglais, espagnol.
//!
//! Chaque langue est une valeur `const` du MÊME type `Strings`. C'est le point
//! important : une traduction oubliée n'est pas une clé absente découverte au
//! rendu, c'est une erreur de compilation (« missing field »). L'ancienne
//! version TypeScript s'appuyait sur `satisfies Record<Lang, …>`, qui ne
//! vérifiait que la forme, pas l'exhaustivité clé par clé.
//!
//! Beaucoup de chaînes contiennent du balisage (`<code>`, `<strong>`) : elles
//! sont injectées via `html::raw`, exactement comme l'ancien
//! `dangerouslySetInnerHTML`. Ce sont des littéraux du binaire, jamais des
//! entrées utilisateur.

use crate::theme::Theme;

/// Une carte « fonctionnalité » : titre + corps.
pub struct Card {
    pub title: &'static str,
    pub body: &'static str,
}

/// Une carte « plateforme » : pictogramme, titre, description, appel à l'action.
pub struct PlatformCard {
    pub icon: &'static str,
    pub title: &'static str,
    pub desc: &'static str,
    pub cta: &'static str,
}

/// Les libellés de la maquette de l'application (section « Application ») :
/// les mêmes mots que dans la vraie application, pour qu'on s'y retrouve.
pub struct Mock {
    pub on_title: &'static str,
    pub off_title: &'static str,
    pub activate: &'static str,
    pub deactivate: &'static str,
    pub browser_to: &'static str,
    pub me: &'static str,
    pub badge: &'static str,
    pub contacts_title: &'static str,
    pub copy: &'static str,
    pub change_key: &'static str,
    pub rename: &'static str,
    pub delete: &'static str,
    pub new_contact: &'static str,
    pub msg_title: &'static str,
    pub message: &'static str,
    pub to: &'static str,
    pub encrypt: &'static str,
    pub decrypt: &'static str,
    pub result_note: &'static str,
}

/// Une entrée du menu : identifiant de la section visée + libellé traduit.
pub struct NavItem {
    pub id: &'static str,
    pub label: &'static str,
}

/// Toutes les chaînes d'une langue.
pub struct Strings {
    pub meta_title: &'static str,
    pub meta_desc: &'static str,
    pub meta_og_title: &'static str,
    pub meta_og_desc: &'static str,

    pub nav_platforms: &'static str,
    pub nav_features: &'static str,
    pub nav_quickstart: &'static str,
    pub nav_app: &'static str,
    pub nav_privacy: &'static str,
    pub nav_menu: &'static str,
    pub nav_download: &'static str,
    pub nav_skip: &'static str,

    pub hero_alt: &'static str,
    pub hero_tagline: &'static str,
    pub hero_lead: &'static str,
    pub hero_cta: &'static str,
    pub hero_cta_sub: &'static str,

    pub switch_lang: &'static str,
    pub switch_theme: &'static str,
    pub theme_green: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,

    pub platforms_h: &'static str,
    pub platforms_intro: &'static str,
    pub platforms_macos_t: &'static str,
    pub platforms_macos_d: &'static str,
    pub platforms_macos_cta: &'static str,
    pub platforms_linux_t: &'static str,
    pub platforms_linux_d: &'static str,
    pub platforms_linux_cta: &'static str,
    pub platforms_windows_t: &'static str,
    pub platforms_windows_d: &'static str,
    pub platforms_windows_cta: &'static str,

    pub dl_back: &'static str,
    pub dl_h: &'static str,
    pub dl_intro: &'static str,
    pub dl_version: &'static str,
    pub dl_macos_t: &'static str,
    pub dl_macos_d: &'static str,
    pub dl_macos_btn: &'static str,
    pub dl_macos_note: &'static str,
    pub dl_linux_t: &'static str,
    pub dl_linux_d: &'static str,
    pub dl_linux_deb: &'static str,
    pub dl_linux_rpm: &'static str,
    pub dl_linux_note: &'static str,
    pub dl_windows_t: &'static str,
    pub dl_windows_d: &'static str,
    pub dl_windows_btn: &'static str,
    pub dl_windows_note: &'static str,
    pub dl_all: &'static str,

    pub s1_h: &'static str,
    /// Description de la maquette de l'interrupteur, pour les lecteurs d'écran.
    pub switch_sr: &'static str,
    pub s1_li: [&'static str; 6],
    pub s1_note: &'static str,

    pub s2_h: &'static str,
    pub s2_cards: [Card; 4],

    pub s3_h: &'static str,
    pub s3_li: [&'static str; 4],

    pub app_h: &'static str,
    pub app_intro: &'static str,
    pub app_tabs: [&'static str; 3],
    /// Ce que fait chaque onglet, affiché à côté de la maquette.
    pub app_captions: [&'static str; 3],
    pub mock: Mock,

    pub s5_h: &'static str,
    pub s5_intro: &'static str,
    pub s5_diagram: &'static str,

    pub s6_h: &'static str,
    pub s6_li: [&'static str; 5],

    pub footer: &'static str,

    // Habillage animé (refonte) : surtitre et invitation à défiler du héros,
    // manifeste, bandeau défilant, appel final du pied de page.
    pub hero_eyebrow: &'static str,
    pub hero_scroll: &'static str,
    pub manifesto_label: &'static str,
    pub manifesto: &'static str,
    pub marquee: [&'static str; 6],
    pub footer_h: &'static str,
    pub footer_top: &'static str,

    // Démonstration animée du chiffrement sélectif.
    pub demo_kicker: &'static str,
    pub demo_h: &'static str,
    pub demo_sr: &'static str,
    pub demo_rows: [&'static str; 4],
    pub demo_before: &'static str,
    pub demo_secret: &'static str,
    pub demo_after: &'static str,
    pub demo_locked: &'static str,
    pub demo_unlocked: &'static str,
}

impl Strings {
    /// Les cinq sections atteignables depuis la barre de navigation.
    pub fn nav_items(&self) -> [NavItem; 5] {
        [
            NavItem {
                id: "platforms",
                label: self.nav_platforms,
            },
            NavItem {
                id: "features",
                label: self.nav_features,
            },
            NavItem {
                id: "quickstart",
                label: self.nav_quickstart,
            },
            NavItem {
                id: "app",
                label: self.nav_app,
            },
            NavItem {
                id: "privacy",
                label: self.nav_privacy,
            },
        ]
    }

    /// Les trois cartes de la section « Plateformes supportées ».
    pub fn platform_cards(&self) -> [PlatformCard; 3] {
        [
            PlatformCard {
                icon: "🍎",
                title: self.platforms_macos_t,
                desc: self.platforms_macos_d,
                cta: self.platforms_macos_cta,
            },
            PlatformCard {
                icon: "🐧",
                title: self.platforms_linux_t,
                desc: self.platforms_linux_d,
                cta: self.platforms_linux_cta,
            },
            PlatformCard {
                icon: "🪟",
                title: self.platforms_windows_t,
                desc: self.platforms_windows_d,
                cta: self.platforms_windows_cta,
            },
        ]
    }

    /// Libellé d'un thème dans la langue courante.
    pub fn theme_label(&self, theme: Theme) -> &'static str {
        match theme {
            Theme::Green => self.theme_green,
            Theme::Light => self.theme_light,
            Theme::Dark => self.theme_dark,
        }
    }
}

/// Les langues proposées, dans l'ordre d'affichage du sélecteur.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    #[default]
    Fr,
    En,
    Es,
}

pub const LANGS: [Lang; 3] = [Lang::Fr, Lang::En, Lang::Es];

impl Lang {
    /// Code ISO 639-1, utilisé pour `<html lang>` et le stockage local.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
            Lang::Es => "es",
        }
    }

    /// Libellé du sélecteur.
    pub fn label(self) -> &'static str {
        match self {
            Lang::Fr => "FR",
            Lang::En => "EN",
            Lang::Es => "ES",
        }
    }

    /// Reconnaît un code de langue. Tolère `fr-FR` comme `FR`, parce que
    /// `navigator.language` renvoie les deux formes selon le navigateur.
    pub fn from_code(raw: &str) -> Option<Self> {
        let base = raw.split(['-', '_']).next().unwrap_or(raw).to_lowercase();
        match base.as_str() {
            "fr" => Some(Lang::Fr),
            "en" => Some(Lang::En),
            "es" => Some(Lang::Es),
            _ => None,
        }
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::Fr => &FR,
            Lang::En => &EN,
            Lang::Es => &ES,
        }
    }
}

// ---------------------------------------------------------------------------
// Français
// ---------------------------------------------------------------------------

pub static FR: Strings = Strings {
    meta_title: "Brocolis 🥦 — Chiffrez ce que vous marquez, pour qui vous voulez",
    meta_desc: "Brocolis est une application pour macOS, Linux et Windows qui chiffre ce que vous marquez ((comme ceci)) sur Twitter, les forums, les messageries — pour les contacts que vous choisissez, en double cliquet post-quantique avec vos contacts appairés, sans rien changer côté serveur.",
    meta_og_title: "Brocolis 🥦 — Chiffrement sélectif de votre trafic web",
    meta_og_desc: "Un clic, zéro configuration. AES-256-GCM sur ce que vous marquez, pour les contacts que vous choisissez. macOS, Linux, Windows.",

    nav_platforms: "Plateformes",
    nav_features: "Fonctionnalités",
    nav_quickstart: "Démarrage rapide",
    nav_app: "Application",
    nav_privacy: "Sécurité",
    nav_menu: "Menu",
    nav_download: "Télécharger",
    nav_skip: "Aller au contenu",

    hero_alt: "Mascotte Brocolis : un brocoli en chapeau et lunettes de soleil devant un ordinateur portable",
    hero_tagline: "L'application qui <strong>chiffre ce que vous marquez</strong>, pour qui vous voulez.",
    hero_lead: "Marquez <code class=\"inv\">((votre secret))</code> dans vos messages : Brocolis le chiffre en AES-256-GCM avant qu'il ne quitte votre machine, pour les contacts que vous choisissez — un contact, une clé. Twitter, forums, messageries ne voient que du chiffré, sans rien changer côté serveur.",
    hero_cta: "Télécharger",
    hero_cta_sub: "macOS · Linux · Windows",

    switch_lang: "Langue",
    switch_theme: "Thème",
    theme_green: "Vert",
    theme_light: "Clair",
    theme_dark: "Sombre",

    platforms_h: "Plateformes supportées",
    platforms_intro: "Une application, trois systèmes. Chaque téléchargement contient l'<strong>application Brocolis</strong> : la protection d'un clic, vos contacts, vos messages. Elle règle le système pour vous, au bon endroit sur chaque OS :",
    platforms_macos_t: "macOS",
    platforms_macos_d: "<code>Brocolis.app</code> universelle, Intel et Apple Silicon. Au clic, votre mot de passe (ou Touch ID) autorise : certificat dans le trousseau Système (Safari, Chrome, Edge), politique Firefox, proxy web et HTTPS.",
    platforms_macos_cta: "Télécharger pour macOS",
    platforms_linux_t: "Linux",
    platforms_linux_d: "Paquets <code>.deb</code> et <code>.rpm</code> : l'application rejoint le menu. Au clic, <code>pkexec</code> autorise : magasin de confiance de la distribution (Debian/Ubuntu, Fedora/RHEL, Arch), politique Firefox, proxy système.",
    platforms_linux_cta: "Télécharger pour Linux",
    platforms_windows_t: "Windows",
    platforms_windows_d: "<code>brocolis-app.exe</code>, x86_64. Au clic, le contrôle de compte d'utilisateur autorise : magasin <em>Autorités de certification racines de confiance</em>, politique Firefox, proxy WinHTTP.",
    platforms_windows_cta: "Télécharger pour Windows",

    dl_back: "Accueil",
    dl_h: "Télécharger Brocolis",
    dl_intro: "Choisissez votre système. Ouvrez ensuite l'<strong>application Brocolis</strong> : créez vos contacts, puis activez la protection d'un clic.",
    dl_version: "Ce sont les <strong>dernières builds</strong>, recompilées automatiquement à chaque mise à jour. Pour une version précise ou l'historique, voir toutes les releases.",
    dl_macos_t: "macOS",
    dl_macos_d: "Application <code>Brocolis.app</code>, universelle Intel + Apple Silicon (<code>.tar.gz</code>).",
    dl_macos_btn: "Télécharger (.tar.gz)",
    dl_macos_note: "Décompressez et glissez <code>Brocolis.app</code> dans Applications. Premier lancement : clic droit → Ouvrir (application non signée).",
    dl_linux_t: "Linux",
    dl_linux_d: "Paquets x86_64 : <code>.deb</code> pour Debian/Ubuntu, <code>.rpm</code> pour Fedora/RHEL.",
    dl_linux_deb: "Télécharger .deb",
    dl_linux_rpm: "Télécharger .rpm",
    dl_linux_note: "Installez le paquet, puis ouvrez <strong>Brocolis</strong> depuis le menu des applications.",
    dl_windows_t: "Windows",
    dl_windows_d: "Application <code>brocolis-app.exe</code> pour Windows x86_64 (<code>.zip</code>).",
    dl_windows_btn: "Télécharger (.zip)",
    dl_windows_note: "Décompressez, puis lancez <code>brocolis-app.exe</code>. Gardez les fichiers du dossier ensemble : l'application s'en sert pour activer la protection.",
    dl_all: "Voir toutes les versions sur GitHub →",

    s1_h: "Un clic, zéro configuration",
    switch_sr: "Maquette de l'application : un grand interrupteur, qui passe de « Protection coupée » à « Protection active ».",
    s1_li: [
        "Le système vous demande votre autorisation <strong>au clic</strong> — mot de passe ou Touch ID sur macOS, <code>pkexec</code> sur Linux, contrôle de compte sur Windows. L'application ne garde jamais ces droits.",
        "Le certificat de Brocolis rejoint le <strong>magasin de confiance natif</strong> : trousseau Système (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust</code> (Linux), <code>certutil</code> (Windows).",
        "<strong>Firefox</strong> lui fait confiance aussi, toutes variantes détectées, par sa politique entreprise.",
        "Le proxy système est dirigé vers Brocolis, qui tourne dans l'application.",
        "Le chiffrement AES-256-GCM démarre, pour les contacts que vous avez choisis.",
        "Recliquez : <strong>tout revient exactement comme avant</strong> — certificat retiré, proxy d'origine remis. Quitter l'application coupe d'abord la protection.",
    ],
    s1_note: "Activer et couper demandent les droits administrateur : c'est le système qui l'exige pour faire confiance au certificat et régler le proxy.",

    s2_h: "Fonctionnalités",
    s2_cards: [
        Card {
            title: "Chiffrement sélectif",
            body: "Seuls les segments <code>((entre doubles parenthèses))</code> sont chiffrés, en AES-256-GCM, sous la forme d'un jeton <code>[[BROCOLIS:v3:…]]</code> pour un contact appairé (<code>v1</code>/<code>v2</code> sinon). Le service en ligne ne stocke que du chiffré ; le reste de la page fonctionne comme avant.",
        },
        Card {
            title: "Un contact, une clé",
            body: "Chaque contact a sa clé, créée dans l'application ou obtenue par un code convenu de vive voix. Choisissez pour qui le navigateur chiffre : un contact, ou plusieurs à la fois — un seul jeton, que chacun ouvre avec la sienne. Appairé par code, un contact passe en <strong>double cliquet</strong> : le protocole de la messagerie Brocolis, sur iPhone et Android.",
        },
        Card {
            title: "Messages entiers",
            body: "Dans l'onglet Message, sans parenthèses, c'est <strong>tout le message</strong> qui est chiffré, à coller où vous voulez. Collez un message reçu : il se déchiffre, et l'application dit de quel contact il vient.",
        },
        Card {
            title: "Tous les navigateurs",
            body: "Safari, Chrome, Edge, Chromium et toutes les variantes de Firefox font confiance au certificat. L'interception HTTPS est transparente : certificats forgés à la volée, TLS terminé avec <em>rustls</em>.",
        },
    ],

    s3_h: "Démarrage rapide",
    s3_li: [
        "<strong>Téléchargez</strong> l'application pour votre système et installez-la : <code>Brocolis.app</code> dans Applications, le paquet <code>.deb</code> ou <code>.rpm</code>, ou <code>brocolis-app.exe</code>.",
        "Onglet <strong>Contacts</strong> : créez un contact. Sa clé est créée et copiée — transmettez-la-lui par un canal sûr — ou appairez-vous avec un code convenu de vive voix.",
        "Onglet <strong>Protection</strong> : choisissez pour qui le navigateur chiffre, puis activez la protection d'un clic.",
        "Écrivez en marquant <code>((ce qui est secret))</code> : votre contact lit le clair, tous les autres du chiffré. Recliquez pour tout remettre comme avant.",
    ],

    app_h: "Trois onglets, c'est tout",
    app_intro: "Pas de terminal, pas de fichier à éditer : tout se fait dans l'application. Faites le tour :",
    app_tabs: ["Protection", "Contacts", "Message"],
    app_captions: [
        "Un interrupteur. Activé, ce que vous marquez est chiffré avant de quitter votre ordinateur, pour les contacts choisis ici. Coupé, votre système revient exactement à son état d'origine.",
        "Un contact, une clé. Copiez-la pour la lui transmettre, changez-en si elle a fuité, renommez ou supprimez le contact. L'appairage par code crée le contact sans rien recopier, en double cliquet ; chaque contact affiche sa protection.",
        "Écrivez, choisissez les destinataires, chiffrez : les segments entre parenthèses, ou tout le message. Collez un message reçu pour le lire — l'application dit de qui il vient.",
    ],
    mock: Mock {
        on_title: "Protection active",
        off_title: "Protection coupée",
        activate: "Activer la protection",
        deactivate: "Couper la protection",
        browser_to: "Le navigateur chiffre pour",
        me: "Moi",
        badge: "navigateur",
        contacts_title: "Vos contacts",
        copy: "Copier",
        change_key: "Changer de clé",
        rename: "Renommer",
        delete: "Supprimer",
        new_contact: "+  Nouveau contact",
        msg_title: "Chiffrer un message",
        message: "On se voit demain à 18h",
        to: "Pour :",
        encrypt: "Chiffrer",
        decrypt: "Déchiffrer",
        result_note: "Message entier chiffré pour Alice.",
    },

    s5_h: "Comment ça marche",
    s5_intro: "Brocolis s'intercale entre votre navigateur et le service en ligne. À l'envoi, il chiffre les segments marqués pour les contacts choisis ; à la réception, il déchiffre les jetons que vos clés ouvrent.",
    s5_diagram: "Navigateur ─►  Brocolis (sur votre machine)  ─►  Service en ligne\n                             │                           │\n   « salut ((secret)) » ───► « salut [[BROCOLIS:v3:…]] »  (chiffré pour Alice)\n   « salut secret »     ◄─── « salut [[BROCOLIS:v3:…]] »  (déchiffré pour vous)",

    s6_h: "Vie privée et sécurité",
    s6_li: [
        "Chiffrement <strong>de bout en bout</strong> entre vous et vos contacts : le service en ligne ne voit jamais le clair.",
        "<strong>Double cliquet</strong> avec un contact appairé : une clé neuve par message, effacée aussitôt — voler une clé plus tard n'ouvre pas les messages passés —, établie en hybride X25519 + ML-KEM-768 contre l'ordinateur quantique de demain. Un contact, une clé : changer celle de l'un n'affecte pas les autres.",
        "Aucune télémétrie, aucun compte : tout tourne sur votre machine. L'appairage par code n'échange que des clés publiques ; la clé partagée, elle, ne circule jamais.",
        "Les droits administrateur sont demandés au clic, <strong>jamais conservés</strong> ; couper la protection retire le certificat et remet le proxy d'origine.",
        "Code source consultable pour audit (logiciel sous licence propriétaire).",
    ],

    footer: "Brocolis — chiffrement sélectif, écrit en Rust. Conçu et développé par Maxime Nathan Lestage.",
    hero_eyebrow: "Application · AES-256-GCM · Rust",
    hero_scroll: "Défiler",
    manifesto_label: "Manifeste",
    manifesto: "Vos mots restent les vôtres. Brocolis chiffre ce que vous marquez avant que ça ne quitte votre machine, pour ceux que vous choisissez, et laisse tout le reste intact. Aucun serveur à convaincre, rien à changer à vos habitudes.",
    marquee: [
        "Chiffrement sélectif",
        "Un contact, une clé",
        "Double cliquet",
        "Un clic",
        "Écrit en Rust",
        "macOS · Linux · Windows",
    ],
    footer_h: "Prêt à chiffrer <em>ce qui compte</em> ?",
    footer_top: "Haut de page",
    demo_kicker: "Démonstration",
    demo_h: "Un message, quatre lecteurs",
    demo_sr: "Démonstration : vous écrivez « Rendez-vous ((demain à 18h)) au café. » Le serveur ne stocke que le jeton chiffré à la place du segment marqué ; votre contact, qui a la clé, lit « demain à 18h » en clair ; tous les autres ne voient que le jeton.",
    demo_rows: [
        "Vous écrivez",
        "Le serveur stocke",
        "Votre contact lit",
        "Les autres voient",
    ],
    demo_before: "Rendez-vous ",
    demo_secret: "demain à 18h",
    demo_after: " au café.",
    demo_locked: "chiffré",
    demo_unlocked: "déchiffré",
};

// ---------------------------------------------------------------------------
// English
// ---------------------------------------------------------------------------

pub static EN: Strings = Strings {
    meta_title: "Brocolis 🥦 — Encrypt what you mark, for whoever you choose",
    meta_desc: "Brocolis is an app for macOS, Linux and Windows that encrypts what you mark ((like this)) on Twitter, forums and messengers — for the contacts you choose, with a post-quantum double ratchet for paired contacts, with nothing to change on the server side.",
    meta_og_title: "Brocolis 🥦 — Selective encryption for your web traffic",
    meta_og_desc: "One click, zero configuration. AES-256-GCM on what you mark, for the contacts you choose. macOS, Linux, Windows.",

    nav_platforms: "Platforms",
    nav_features: "Features",
    nav_quickstart: "Quick start",
    nav_app: "App",
    nav_privacy: "Privacy",
    nav_menu: "Menu",
    nav_download: "Download",
    nav_skip: "Skip to content",

    hero_alt: "Brocolis mascot: a broccoli with hat and sunglasses in front of a laptop",
    hero_tagline: "The app that <strong>encrypts what you mark</strong>, for whoever you choose.",
    hero_lead: "Mark <code class=\"inv\">((your secret))</code> in your messages: Brocolis encrypts it with AES-256-GCM before it leaves your machine, for the contacts you choose — one contact, one key. Twitter, forums and messengers only ever see ciphertext, with nothing to change on the server side.",
    hero_cta: "Download",
    hero_cta_sub: "macOS · Linux · Windows",

    switch_lang: "Language",
    switch_theme: "Theme",
    theme_green: "Green",
    theme_light: "Light",
    theme_dark: "Dark",

    platforms_h: "Supported platforms",
    platforms_intro: "One app, three systems. Every download includes the <strong>Brocolis app</strong>: one-click protection, your contacts, your messages. It sets up the system for you, the right way on each OS:",
    platforms_macos_t: "macOS",
    platforms_macos_d: "Universal <code>Brocolis.app</code>, Intel and Apple Silicon. On click, your password (or Touch ID) authorizes it: certificate in the System keychain (Safari, Chrome, Edge), Firefox policy, web and HTTPS proxies.",
    platforms_macos_cta: "Download for macOS",
    platforms_linux_t: "Linux",
    platforms_linux_d: "<code>.deb</code> and <code>.rpm</code> packages: the app joins your menu. On click, <code>pkexec</code> authorizes it: distro trust store (Debian/Ubuntu, Fedora/RHEL, Arch), Firefox policy, system proxy.",
    platforms_linux_cta: "Download for Linux",
    platforms_windows_t: "Windows",
    platforms_windows_d: "<code>brocolis-app.exe</code>, x86_64. On click, User Account Control authorizes it: <em>Trusted Root Certification Authorities</em> store, Firefox policy, WinHTTP proxy.",
    platforms_windows_cta: "Download for Windows",

    dl_back: "Home",
    dl_h: "Download Brocolis",
    dl_intro: "Pick your system. Then open the <strong>Brocolis app</strong>: create your contacts and turn protection on with one click.",
    dl_version: "These are the <strong>latest builds</strong>, recompiled automatically on every update. For a specific version or the history, see all releases.",
    dl_macos_t: "macOS",
    dl_macos_d: "<code>Brocolis.app</code>, universal Intel + Apple Silicon (<code>.tar.gz</code>).",
    dl_macos_btn: "Download (.tar.gz)",
    dl_macos_note: "Unpack and drag <code>Brocolis.app</code> into Applications. First launch: right-click → Open (unsigned app).",
    dl_linux_t: "Linux",
    dl_linux_d: "x86_64 packages: <code>.deb</code> for Debian/Ubuntu, <code>.rpm</code> for Fedora/RHEL.",
    dl_linux_deb: "Download .deb",
    dl_linux_rpm: "Download .rpm",
    dl_linux_note: "Install the package, then open <strong>Brocolis</strong> from your applications menu.",
    dl_windows_t: "Windows",
    dl_windows_d: "<code>brocolis-app.exe</code> for Windows x86_64 (<code>.zip</code>).",
    dl_windows_btn: "Download (.zip)",
    dl_windows_note: "Unzip, then run <code>brocolis-app.exe</code>. Keep the folder's files together: the app uses them to turn protection on.",
    dl_all: "See all versions on GitHub →",

    s1_h: "One click, zero configuration",
    switch_sr: "App mock-up: a big switch going from “Protection off” to “Protection on”.",
    s1_li: [
        "The system asks for your permission <strong>on click</strong> — password or Touch ID on macOS, <code>pkexec</code> on Linux, User Account Control on Windows. The app never keeps those rights.",
        "The Brocolis certificate joins the <strong>native trust store</strong>: System keychain (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust</code> (Linux), <code>certutil</code> (Windows).",
        "<strong>Firefox</strong> trusts it too, every variant detected, through its enterprise policy.",
        "The system proxy points at Brocolis, running inside the app.",
        "AES-256-GCM encryption starts, for the contacts you picked.",
        "Click again: <strong>everything goes back exactly as it was</strong> — certificate removed, original proxy restored. Quitting the app turns protection off first.",
    ],
    s1_note: "Turning protection on and off requires administrator rights: the system demands them to trust the certificate and set the proxy.",

    s2_h: "Features",
    s2_cards: [
        Card {
            title: "Selective encryption",
            body: "Only segments wrapped in <code>((double parentheses))</code> are encrypted, with AES-256-GCM, as a <code>[[BROCOLIS:v3:…]]</code> token for a paired contact (<code>v1</code>/<code>v2</code> otherwise). The online service only stores ciphertext; the rest of the page works as before.",
        },
        Card {
            title: "One contact, one key",
            body: "Each contact has their own key, created in the app or obtained from a code agreed in person. Choose who the browser encrypts for: one contact, or several at once — a single token each opens with their own key. Paired with a code, a contact switches to the <strong>double ratchet</strong>: the protocol of the Brocolis messenger on iPhone and Android.",
        },
        Card {
            title: "Whole messages",
            body: "In the Message tab, without parentheses, the <strong>whole message</strong> is encrypted, ready to paste anywhere. Paste a message you received: it decrypts, and the app tells you which contact it came from.",
        },
        Card {
            title: "Every browser",
            body: "Safari, Chrome, Edge, Chromium and every Firefox variant trust the certificate. HTTPS interception is transparent: leaf certificates forged on the fly, TLS terminated with <em>rustls</em>.",
        },
    ],

    s3_h: "Quick start",
    s3_li: [
        "<strong>Download</strong> the app for your system and install it: <code>Brocolis.app</code> in Applications, the <code>.deb</code> or <code>.rpm</code> package, or <code>brocolis-app.exe</code>.",
        "<strong>Contacts</strong> tab: create a contact. Their key is created and copied — send it to them over a secure channel — or pair with a code agreed in person.",
        "<strong>Protection</strong> tab: choose who the browser encrypts for, then turn protection on with one click.",
        "Write, marking <code>((what is secret))</code>: your contact reads plaintext, everyone else ciphertext. Click again to put everything back as it was.",
    ],

    app_h: "Three tabs, that's all",
    app_intro: "No terminal, no file to edit: everything happens in the app. Take the tour:",
    app_tabs: ["Protection", "Contacts", "Message"],
    app_captions: [
        "A switch. On, what you mark is encrypted before it leaves your computer, for the contacts picked here. Off, your system goes back exactly to how it was.",
        "One contact, one key. Copy it to send it to them, change it if it leaked, rename or delete the contact. Pairing with a code creates the contact without copying anything, with the double ratchet; each contact shows its protection.",
        "Write, pick the recipients, encrypt: the parts in parentheses, or the whole message. Paste a message you received to read it — the app tells you who it's from.",
    ],
    mock: Mock {
        on_title: "Protection on",
        off_title: "Protection off",
        activate: "Turn protection on",
        deactivate: "Turn protection off",
        browser_to: "The browser encrypts for",
        me: "Me",
        badge: "browser",
        contacts_title: "Your contacts",
        copy: "Copy",
        change_key: "Change key",
        rename: "Rename",
        delete: "Delete",
        new_contact: "+  New contact",
        msg_title: "Encrypt a message",
        message: "See you tomorrow at 6pm",
        to: "To:",
        encrypt: "Encrypt",
        decrypt: "Decrypt",
        result_note: "Whole message encrypted for Alice.",
    },

    s5_h: "How it works",
    s5_intro: "Brocolis sits between your browser and the online service. On the way out, it encrypts the marked segments for the chosen contacts; on the way back, it decrypts the tokens your keys can open.",
    s5_diagram: "Browser ─►  Brocolis (on your machine)  ─►  Online service\n                         │                         │\n   \"hi ((secret))\" ───► \"hi [[BROCOLIS:v3:…]]\"  (encrypted for Alice)\n   \"hi secret\"     ◄─── \"hi [[BROCOLIS:v3:…]]\"  (decrypted for you)",

    s6_h: "Privacy and security",
    s6_li: [
        "<strong>End-to-end</strong> encryption between you and your contacts: the online service never sees plaintext.",
        "<strong>Double ratchet</strong> with a paired contact: a fresh key for every message, erased right away — stealing a key later doesn't open past messages —, set up as a hybrid X25519 + ML-KEM-768 against tomorrow's quantum computer. One contact, one key: changing one doesn't affect the others.",
        "No telemetry, no account: everything runs on your machine. Pairing with a code only exchanges public keys; the shared key itself never travels.",
        "Administrator rights are requested on click and <strong>never kept</strong>; turning protection off removes the certificate and restores the original proxy.",
        "Source code available for audit (proprietary-licensed software).",
    ],

    footer: "Brocolis — selective encryption, written in Rust. Designed and developed by Maxime Nathan Lestage.",
    hero_eyebrow: "App · AES-256-GCM · Rust",
    hero_scroll: "Scroll",
    manifesto_label: "Manifesto",
    manifesto: "Your words stay yours. Brocolis encrypts what you mark before it leaves your machine, for the people you choose, and leaves everything else untouched. No server to convince, no habit to change.",
    marquee: [
        "Selective encryption",
        "One contact, one key",
        "Double ratchet",
        "One click",
        "Written in Rust",
        "macOS · Linux · Windows",
    ],
    footer_h: "Ready to encrypt <em>what matters</em>?",
    footer_top: "Back to top",
    demo_kicker: "Demo",
    demo_h: "One message, four readers",
    demo_sr: "Demo: you write \"Meet me ((tomorrow at 6pm)) at the café.\" The server only stores the encrypted token in place of the marked segment; your contact, who has the key, reads \"tomorrow at 6pm\" in clear; everyone else only sees the token.",
    demo_rows: [
        "You write",
        "The server stores",
        "Your contact reads",
        "Everyone else sees",
    ],
    demo_before: "Meet me ",
    demo_secret: "tomorrow at 6pm",
    demo_after: " at the café.",
    demo_locked: "encrypted",
    demo_unlocked: "decrypted",
};

// ---------------------------------------------------------------------------
// Español
// ---------------------------------------------------------------------------

pub static ES: Strings = Strings {
    meta_title: "Brocolis 🥦 — Cifra lo que marcas, para quien quieras",
    meta_desc: "Brocolis es una aplicación para macOS, Linux y Windows que cifra lo que marcas ((así)) en Twitter, foros y mensajerías — para los contactos que eliges, con doble trinquete poscuántico para tus contactos emparejados, sin cambiar nada en el servidor.",
    meta_og_title: "Brocolis 🥦 — Cifrado selectivo de tu tráfico web",
    meta_og_desc: "Un clic, cero configuración. AES-256-GCM en lo que marcas, para los contactos que eliges. macOS, Linux, Windows.",

    nav_platforms: "Plataformas",
    nav_features: "Características",
    nav_quickstart: "Inicio rápido",
    nav_app: "Aplicación",
    nav_privacy: "Seguridad",
    nav_menu: "Menú",
    nav_download: "Descargar",
    nav_skip: "Ir al contenido",

    hero_alt: "Mascota de Brocolis: un brócoli con sombrero y gafas de sol frente a un portátil",
    hero_tagline: "La aplicación que <strong>cifra lo que marcas</strong>, para quien quieras.",
    hero_lead: "Marca <code class=\"inv\">((tu secreto))</code> en tus mensajes: Brocolis lo cifra con AES-256-GCM antes de que salga de tu equipo, para los contactos que eliges — un contacto, una clave. Twitter, foros y mensajerías solo ven texto cifrado, sin cambiar nada en el servidor.",
    hero_cta: "Descargar",
    hero_cta_sub: "macOS · Linux · Windows",

    switch_lang: "Idioma",
    switch_theme: "Tema",
    theme_green: "Verde",
    theme_light: "Claro",
    theme_dark: "Oscuro",

    platforms_h: "Plataformas soportadas",
    platforms_intro: "Una aplicación, tres sistemas. Cada descarga incluye la <strong>aplicación Brocolis</strong>: la protección con un clic, tus contactos, tus mensajes. Configura el sistema por ti, como corresponde en cada SO:",
    platforms_macos_t: "macOS",
    platforms_macos_d: "<code>Brocolis.app</code> universal, Intel y Apple Silicon. Al hacer clic, tu contraseña (o Touch ID) lo autoriza: certificado en el llavero del Sistema (Safari, Chrome, Edge), política de Firefox, proxies web y HTTPS.",
    platforms_macos_cta: "Descargar para macOS",
    platforms_linux_t: "Linux",
    platforms_linux_d: "Paquetes <code>.deb</code> y <code>.rpm</code>: la aplicación aparece en tu menú. Al hacer clic, <code>pkexec</code> lo autoriza: almacén de confianza de la distribución (Debian/Ubuntu, Fedora/RHEL, Arch), política de Firefox, proxy del sistema.",
    platforms_linux_cta: "Descargar para Linux",
    platforms_windows_t: "Windows",
    platforms_windows_d: "<code>brocolis-app.exe</code>, x86_64. Al hacer clic, el Control de cuentas de usuario lo autoriza: almacén <em>Entidades de certificación raíz de confianza</em>, política de Firefox, proxy WinHTTP.",
    platforms_windows_cta: "Descargar para Windows",

    dl_back: "Inicio",
    dl_h: "Descargar Brocolis",
    dl_intro: "Elige tu sistema. Luego abre la <strong>aplicación Brocolis</strong>: crea tus contactos y activa la protección con un clic.",
    dl_version: "Estas son las <strong>últimas builds</strong>, recompiladas automáticamente en cada actualización. Para una versión concreta o el historial, mira todas las releases.",
    dl_macos_t: "macOS",
    dl_macos_d: "<code>Brocolis.app</code>, universal Intel + Apple Silicon (<code>.tar.gz</code>).",
    dl_macos_btn: "Descargar (.tar.gz)",
    dl_macos_note: "Descomprime y arrastra <code>Brocolis.app</code> a Aplicaciones. Primer inicio: clic derecho → Abrir (aplicación sin firmar).",
    dl_linux_t: "Linux",
    dl_linux_d: "Paquetes x86_64: <code>.deb</code> para Debian/Ubuntu, <code>.rpm</code> para Fedora/RHEL.",
    dl_linux_deb: "Descargar .deb",
    dl_linux_rpm: "Descargar .rpm",
    dl_linux_note: "Instala el paquete y abre <strong>Brocolis</strong> desde el menú de aplicaciones.",
    dl_windows_t: "Windows",
    dl_windows_d: "<code>brocolis-app.exe</code> para Windows x86_64 (<code>.zip</code>).",
    dl_windows_btn: "Descargar (.zip)",
    dl_windows_note: "Descomprime y ejecuta <code>brocolis-app.exe</code>. Mantén juntos los archivos de la carpeta: la aplicación los usa para activar la protección.",
    dl_all: "Ver todas las versiones en GitHub →",

    s1_h: "Un clic, cero configuración",
    switch_sr: "Maqueta de la aplicación: un gran interruptor que pasa de «Protección desactivada» a «Protección activa».",
    s1_li: [
        "El sistema te pide permiso <strong>al hacer clic</strong> — contraseña o Touch ID en macOS, <code>pkexec</code> en Linux, Control de cuentas en Windows. La aplicación nunca conserva esos derechos.",
        "El certificado de Brocolis entra en el <strong>almacén de confianza nativo</strong>: llavero del Sistema (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust</code> (Linux), <code>certutil</code> (Windows).",
        "<strong>Firefox</strong> también confía en él, todas las variantes detectadas, mediante su política empresarial.",
        "El proxy del sistema apunta a Brocolis, que funciona dentro de la aplicación.",
        "El cifrado AES-256-GCM arranca, para los contactos que elegiste.",
        "Vuelve a hacer clic: <strong>todo queda exactamente como antes</strong> — certificado retirado, proxy original restaurado. Salir de la aplicación desactiva primero la protección.",
    ],
    s1_note: "Activar y desactivar la protección requiere derechos de administrador: el sistema los exige para confiar en el certificado y configurar el proxy.",

    s2_h: "Características",
    s2_cards: [
        Card {
            title: "Cifrado selectivo",
            body: "Solo los segmentos <code>((entre dobles paréntesis))</code> se cifran, con AES-256-GCM, como un token <code>[[BROCOLIS:v3:…]]</code> para un contacto emparejado (<code>v1</code>/<code>v2</code> si no). El servicio en línea solo almacena texto cifrado; el resto de la página funciona como antes.",
        },
        Card {
            title: "Un contacto, una clave",
            body: "Cada contacto tiene su clave, creada en la aplicación u obtenida con un código acordado en persona. Elige para quién cifra el navegador: un contacto, o varios a la vez — un solo token, que cada uno abre con la suya. Emparejado con un código, un contacto pasa al <strong>doble trinquete</strong>: el protocolo de la mensajería Brocolis, en iPhone y Android.",
        },
        Card {
            title: "Mensajes enteros",
            body: "En la pestaña Mensaje, sin paréntesis, se cifra <strong>todo el mensaje</strong>, listo para pegar donde quieras. Pega un mensaje recibido: se descifra, y la aplicación te dice de qué contacto viene.",
        },
        Card {
            title: "Todos los navegadores",
            body: "Safari, Chrome, Edge, Chromium y todas las variantes de Firefox confían en el certificado. La interceptación HTTPS es transparente: certificados forjados al vuelo, TLS terminado con <em>rustls</em>.",
        },
    ],

    s3_h: "Inicio rápido",
    s3_li: [
        "<strong>Descarga</strong> la aplicación para tu sistema e instálala: <code>Brocolis.app</code> en Aplicaciones, el paquete <code>.deb</code> o <code>.rpm</code>, o <code>brocolis-app.exe</code>.",
        "Pestaña <strong>Contactos</strong>: crea un contacto. Su clave se crea y se copia — envíasela por un canal seguro — o emparejaos con un código acordado en persona.",
        "Pestaña <strong>Protección</strong>: elige para quién cifra el navegador y activa la protección con un clic.",
        "Escribe marcando <code>((lo que es secreto))</code>: tu contacto lee el texto claro, los demás solo texto cifrado. Vuelve a hacer clic para dejarlo todo como antes.",
    ],

    app_h: "Tres pestañas, nada más",
    app_intro: "Sin terminal, sin archivos que editar: todo se hace en la aplicación. Échale un vistazo:",
    app_tabs: ["Protección", "Contactos", "Mensaje"],
    app_captions: [
        "Un interruptor. Activado, lo que marcas se cifra antes de salir de tu equipo, para los contactos elegidos aquí. Desactivado, tu sistema vuelve exactamente a su estado original.",
        "Un contacto, una clave. Cópiala para enviársela, cámbiala si se filtró, renombra o elimina el contacto. Emparejar con un código crea el contacto sin copiar nada, con doble trinquete; cada contacto muestra su protección.",
        "Escribe, elige los destinatarios, cifra: los fragmentos entre paréntesis, o todo el mensaje. Pega un mensaje recibido para leerlo — la aplicación te dice de quién viene.",
    ],
    mock: Mock {
        on_title: "Protección activa",
        off_title: "Protección desactivada",
        activate: "Activar la protección",
        deactivate: "Desactivar la protección",
        browser_to: "El navegador cifra para",
        me: "Yo",
        badge: "navegador",
        contacts_title: "Tus contactos",
        copy: "Copiar",
        change_key: "Cambiar de clave",
        rename: "Renombrar",
        delete: "Eliminar",
        new_contact: "+  Nuevo contacto",
        msg_title: "Cifrar un mensaje",
        message: "Nos vemos mañana a las 18h",
        to: "Para:",
        encrypt: "Cifrar",
        decrypt: "Descifrar",
        result_note: "Mensaje entero cifrado para Alice.",
    },

    s5_h: "Cómo funciona",
    s5_intro: "Brocolis se sitúa entre tu navegador y el servicio en línea. Al enviar, cifra los segmentos marcados para los contactos elegidos; al recibir, descifra los tokens que tus claves pueden abrir.",
    s5_diagram: "Navegador ─►  Brocolis (en tu equipo)  ─►  Servicio en línea\n                         │                         │\n   «hola ((secreto))» ───► «hola [[BROCOLIS:v3:…]]»  (cifrado para Alice)\n   «hola secreto»     ◄─── «hola [[BROCOLIS:v3:…]]»  (descifrado para ti)",

    s6_h: "Privacidad y seguridad",
    s6_li: [
        "Cifrado <strong>de extremo a extremo</strong> entre tú y tus contactos: el servicio en línea nunca ve el texto claro.",
        "<strong>Doble trinquete</strong> con un contacto emparejado: una clave nueva por mensaje, borrada enseguida — robar una clave más tarde no abre los mensajes pasados —, establecida en híbrido X25519 + ML-KEM-768 frente al ordenador cuántico de mañana. Un contacto, una clave: cambiar la de uno no afecta a los demás.",
        "Sin telemetría, sin cuenta: todo funciona en tu equipo. Emparejar con un código solo intercambia claves públicas; la clave compartida nunca viaja.",
        "Los derechos de administrador se piden al hacer clic y <strong>nunca se conservan</strong>; desactivar la protección retira el certificado y restaura el proxy original.",
        "Código fuente consultable para auditoría (software bajo licencia propietaria).",
    ],

    footer: "Brocolis — cifrado selectivo, escrito en Rust. Diseñado y desarrollado por Maxime Nathan Lestage.",
    hero_eyebrow: "Aplicación · AES-256-GCM · Rust",
    hero_scroll: "Desplazar",
    manifesto_label: "Manifiesto",
    manifesto: "Tus palabras siguen siendo tuyas. Brocolis cifra lo que marcas antes de que salga de tu equipo, para quien tú eliges, y deja todo lo demás intacto. Ningún servidor que convencer, ningún hábito que cambiar.",
    marquee: [
        "Cifrado selectivo",
        "Un contacto, una clave",
        "Doble trinquete",
        "Un clic",
        "Escrito en Rust",
        "macOS · Linux · Windows",
    ],
    footer_h: "¿Listo para cifrar <em>lo que importa</em>?",
    footer_top: "Volver arriba",
    demo_kicker: "Demostración",
    demo_h: "Un mensaje, cuatro lectores",
    demo_sr: "Demostración: escribes «Nos vemos ((mañana a las 18h)) en el café.» El servidor solo almacena el token cifrado en lugar del segmento marcado; tu contacto, que tiene la clave, lee «mañana a las 18h» en claro; los demás solo ven el token.",
    demo_rows: [
        "Tú escribes",
        "El servidor guarda",
        "Tu contacto lee",
        "Los demás ven",
    ],
    demo_before: "Nos vemos ",
    demo_secret: "mañana a las 18h",
    demo_after: " en el café.",
    demo_locked: "cifrado",
    demo_unlocked: "descifrado",
};
