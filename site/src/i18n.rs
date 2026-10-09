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
    pub nav_cli: &'static str,
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
    pub s1_li: [&'static str; 6],
    pub s1_note: &'static str,

    pub s2_h: &'static str,
    pub s2_cards: [Card; 4],

    pub s3_h: &'static str,
    pub s3_li: [&'static str; 4],

    pub s4_h: &'static str,
    pub s4_intro: &'static str,
    pub s4_code: &'static str,
    pub s4_outro: &'static str,

    pub s5_h: &'static str,
    pub s5_intro: &'static str,
    pub s5_diagram: &'static str,

    pub s6_h: &'static str,
    pub s6_li: [&'static str; 4],

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
                id: "cli",
                label: self.nav_cli,
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
    meta_title: "Brocolis 🥦 — Proxy MITM Rust de chiffrement sélectif",
    meta_desc: "Proxy MITM en Rust qui chiffre sélectivement le trafic HTTP/HTTPS sur Twitter, les forums, les messageries. Configuration automatique sur macOS, Linux et Windows — sans rien changer côté serveur.",
    meta_og_title: "Brocolis 🥦 — Chiffrement sélectif de votre trafic web",
    meta_og_desc: "Une commande, zéro configuration. AES-256-GCM sur les segments que vous marquez. macOS, Linux, Windows.",

    nav_platforms: "Plateformes",
    nav_features: "Fonctionnalités",
    nav_quickstart: "Démarrage rapide",
    nav_cli: "CLI",
    nav_privacy: "Sécurité",
    nav_menu: "Menu",
    nav_download: "Télécharger",
    nav_skip: "Aller au contenu",

    hero_alt: "Mascotte Brocolis : un brocoli en chapeau et lunettes de soleil devant un ordinateur portable",
    hero_tagline: "Proxy MITM Rust pour le <strong>chiffrement sélectif</strong> de votre trafic web.",
    hero_lead: "Marquez <code class=\"inv\">((votre secret))</code> dans vos messages : Brocolis le chiffre en AES-256-GCM avant qu'il ne quitte votre machine, et le déchiffre pour les utilisateurs qui partagent votre clé. Twitter, forums, messageries — sans rien changer côté serveur.",
    hero_cta: "Télécharger",
    hero_cta_sub: "macOS · Linux · Windows",

    switch_lang: "Langue",
    switch_theme: "Thème",
    theme_green: "Vert",
    theme_light: "Clair",
    theme_dark: "Sombre",

    platforms_h: "Plateformes supportées",
    platforms_intro: "Chaque téléchargement contient l'<strong>application Brocolis</strong> — un interrupteur, vos contacts, vos messages — et la ligne de commande. <code>brocolis auto</code> détecte l'OS et fait le bon setup système. Téléchargez la build pour votre plateforme :",
    platforms_macos_t: "macOS",
    platforms_macos_d: "Trousseau Système (Safari, Chrome, Edge) + politique entreprise Firefox + profil <code>.mobileconfig</code> alternatif + proxies web/HTTPS via <code>networksetup</code>. Binaire universel Intel + Apple Silicon.",
    platforms_macos_cta: "Télécharger pour macOS",
    platforms_linux_t: "Linux",
    platforms_linux_d: "Magasin de confiance distro (Debian/Ubuntu, Fedora/RHEL, Arch détectés via leurs outils) + politique entreprise Firefox + variables <code>http_proxy</code> dans <code>/etc/profile.d/</code>. Disponible aussi en paquets <code>.deb</code> et <code>.rpm</code>.",
    platforms_linux_cta: "Télécharger pour Linux",
    platforms_windows_t: "Windows",
    platforms_windows_d: "Magasin <em>Local Machine\\Trusted Root</em> via <code>certutil</code> + politique entreprise Firefox + proxy WinHTTP via <code>netsh</code>. Scripts PowerShell <code>install.ps1</code> / <code>uninstall.ps1</code> pour les installs persistantes.",
    platforms_windows_cta: "Télécharger pour Windows",

    dl_back: "Accueil",
    dl_h: "Télécharger Brocolis",
    dl_intro: "Choisissez votre système d'exploitation. Ouvrez ensuite l'<strong>application Brocolis</strong> et activez la protection d'un clic — ou lancez <code>brocolis auto</code> en ligne de commande.",
    dl_version: "Ce sont les <strong>dernières builds</strong>, recompilées automatiquement à chaque mise à jour. Pour une version précise ou l'historique, voir toutes les releases.",
    dl_macos_t: "macOS",
    dl_macos_d: "Application <code>Brocolis.app</code> et ligne de commande, universelles Intel + Apple Silicon (<code>.tar.gz</code>).",
    dl_macos_btn: "Télécharger (.tar.gz)",
    dl_macos_note: "Décompressez et glissez <code>Brocolis.app</code> dans Applications. Premier lancement : clic droit → Ouvrir (application non signée).",
    dl_linux_t: "Linux",
    dl_linux_d: "Paquets x86_64 : <code>.deb</code> pour Debian/Ubuntu, <code>.rpm</code> pour Fedora/RHEL. Application de bureau et ligne de commande.",
    dl_linux_deb: "Télécharger .deb",
    dl_linux_rpm: "Télécharger .rpm",
    dl_linux_note: "Installez le paquet, puis ouvrez <strong>Brocolis</strong> depuis le menu des applications (ou <code>sudo brocolis on</code>).",
    dl_windows_t: "Windows",
    dl_windows_d: "Application <code>brocolis-app.exe</code>, ligne de commande et scripts d'installation, x86_64 (<code>.zip</code>).",
    dl_windows_btn: "Télécharger (.zip)",
    dl_windows_note: "Décompressez, puis lancez <code>brocolis-app.exe</code> (gardez <code>brocolis.exe</code> dans le même dossier).",
    dl_all: "Voir toutes les versions sur GitHub →",

    s1_h: "Une commande, zéro configuration",
    s1_li: [
        "Mot de passe administrateur demandé <strong>une seule fois</strong> (<code>sudo</code> sur macOS/Linux, élévation UAC sur Windows)",
        "Approuve le CA dans le magasin de confiance natif : <strong>trousseau Système</strong> (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust extract-compat</code> (Linux), <code>certutil -addstore Root</code> (Windows)",
        "Active la confiance entreprise pour <strong>Firefox</strong> (toutes variantes détectées). Sur macOS, génère aussi un profil <code>brocolis-firefox.mobileconfig</code> qui ne modifie pas le bundle Firefox",
        "Règle le proxy système : <code>networksetup</code> (macOS), <code>/etc/profile.d/brocolis-proxy.sh</code> avec <code>http_proxy</code>/<code>https_proxy</code> (Linux), <code>netsh winhttp set proxy</code> (Windows)",
        "Démarre le chiffrement sélectif AES-256-GCM",
        "<strong>Restaure exactement l'état initial</strong> à l'arrêt (Ctrl-C) : CA retiré, politiques/profil/variables retirés, proxy remis comme avant",
    ],
    s1_note: "Sans élévation administrateur, <code>brocolis auto</code> démarre le proxy <strong>sans toucher au système</strong> (équivalent à <code>--no-system</code>) : pointez votre navigateur sur <code>127.0.0.1:&lt;port&gt;</code> manuellement.",

    s2_h: "Fonctionnalités",
    s2_cards: [
        Card {
            title: "Chiffrement sélectif",
            body: "Seuls les segments <code>((entre doubles parenthèses))</code> sont chiffrés, en AES-256-GCM, sous la forme d'un jeton <code>[[BROCOLIS:v1:…]]</code>. Le serveur distant ne stocke que du chiffré. Dans l'application, sans parenthèses, c'est <strong>tout le message</strong> qui est chiffré.",
        },
        Card {
            title: "MITM transparent",
            body: "Interception HTTPS via <code>CONNECT</code>, certificats feuilles forgés à la volée par une autorité interne (<em>rcgen</em>), TLS terminé côté client avec <em>rustls</em>.",
        },
        Card {
            title: "Contacts et clés",
            body: "<strong>Un contact, une clé</strong> AES-256, créée dans l'application ou obtenue par un code convenu de vive voix. Chiffrez pour un contact ou pour plusieurs à la fois — un seul jeton, que chacun ouvre avec sa clé — et sachez de qui vient un message.",
        },
        Card {
            title: "Multi-navigateurs",
            body: "Safari, Chrome, Edge, Chromium et toutes les variantes Firefox font confiance au CA automatiquement — via le magasin de confiance natif de l'OS et la politique entreprise Firefox.",
        },
    ],

    s3_h: "Démarrage rapide",
    s3_li: [
        "Téléchargez la build pour votre plateforme depuis la section ci-dessus (<code>.tar.gz</code> macOS universel, <code>.deb</code>/<code>.rpm</code> Linux ou <code>.zip</code> Windows x86_64).",
        "Lancez <code>sudo brocolis auto</code> (macOS / Linux) ou <code>brocolis.exe auto</code> depuis un terminal <em>Exécuter en tant qu'administrateur</em> (Windows). Le mot de passe administrateur n'est demandé qu'<strong>une fois</strong>.",
        "Tapez vos messages en marquant ce qui doit rester privé : <code>((mon secret))</code>. Vos correspondants partageant la clé voient le clair ; tous les autres ne voient que du chiffré.",
        "<code>Ctrl-C</code> pour tout arrêter : Brocolis remet votre système exactement dans son état initial.",
    ],

    s4_h: "Compiler depuis les sources &amp; CLI manuel",
    s4_intro: "Pour compiler vous-même ou tout piloter à la main (les commandes sont identiques sur les trois plateformes) :",
    s4_code: "# Compiler (Rust stable, depuis les sources)\ncargo build --release          # binaire : target/release/brocolis\n\n# 1. Générer une clé symétrique AES-256 (à partager par canal sûr)\n$ brocolis keygen\nSomopQcHJyVQQAWedZFGY5E4+hxKjZemOB7aGnYr6kw=\n\n# 2. Exporter le certificat racine pour l'installer dans le navigateur\n$ brocolis ca --export brocolis-ca.crt\n\n# 3. Lancer le proxy (clé renseignée dans brocolis.toml, cf. brocolis.example.toml)\n$ brocolis run --config brocolis.toml\nBrocolis écoute sur http://127.0.0.1:8080 (proxy HTTP/HTTPS)",
    s4_outro: "Sur Unix, la gestion des clés se pilote par signaux : <code>SIGHUP</code> régénère la clé (rotation en cas de compromission), <code>SIGUSR1</code> recharge la clé partagée depuis le fichier de configuration.",

    s5_h: "Comment ça marche",
    s5_intro: "Brocolis s'intercale entre votre navigateur et le service en ligne. À l'envoi, il chiffre les segments marqués ; à la réception, il déchiffre les jetons lisibles avec la clé partagée.",
    s5_diagram: "Navigateur ─►  Brocolis (proxy local)  ─►  Serveur distant\n                       │                              │\n   « salut ((secret)) » ───► « salut [[BROCOLIS:v1:…]] » (chiffré sur le fil)\n   « salut secret »     ◄─── « salut [[BROCOLIS:v1:…]] » (déchiffré pour vous)",

    s6_h: "Vie privée et sécurité",
    s6_li: [
        "Chiffrement <strong>de bout en bout</strong> entre utilisateurs partageant la clé — le service distant ne voit jamais le clair.",
        "Aucune télémétrie, aucun serveur, aucun compte — tout tourne en local.",
        "Le CA est <strong>retiré du magasin de confiance</strong> automatiquement à l'arrêt s'il a été ajouté par Brocolis.",
        "Code source consultable pour audit (logiciel sous licence propriétaire).",
    ],

    footer: "Brocolis — proxy MITM en Rust. Conçu et développé par Maxime Nathan Lestage.",
    hero_eyebrow: "Proxy MITM · Rust · AES-256-GCM",
    hero_scroll: "Défiler",
    manifesto_label: "Manifeste",
    manifesto: "Vos mots restent les vôtres. Brocolis chiffre ce que vous marquez avant que ça ne quitte votre machine, et laisse tout le reste intact. Aucun serveur à convaincre, aucune application à changer.",
    marquee: [
        "Chiffrement sélectif",
        "AES-256-GCM",
        "MITM transparent",
        "Zéro configuration",
        "Écrit en Rust",
        "macOS · Linux · Windows",
    ],
    footer_h: "Prêt à chiffrer <em>ce qui compte</em> ?",
    footer_top: "Haut de page",
    demo_kicker: "Démonstration",
    demo_h: "Un message, quatre lecteurs",
    demo_sr: "Démonstration : vous écrivez « Rendez-vous ((demain à 18h)) au café. » Le serveur ne stocke que le jeton chiffré à la place du segment marqué ; vos proches qui partagent la clé lisent « demain à 18h » en clair ; tous les autres ne voient que le jeton.",
    demo_rows: [
        "Vous écrivez",
        "Le serveur stocke",
        "Vos proches lisent",
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
    meta_title: "Brocolis 🥦 — Rust MITM proxy for selective encryption",
    meta_desc: "A Rust MITM proxy that selectively encrypts HTTP/HTTPS traffic on Twitter, forums, messengers. Automatic system setup on macOS, Linux and Windows — with nothing to change on the server side.",
    meta_og_title: "Brocolis 🥦 — Selective encryption for your web traffic",
    meta_og_desc: "One command, zero configuration. AES-256-GCM on the segments you mark. macOS, Linux, Windows.",

    nav_platforms: "Platforms",
    nav_features: "Features",
    nav_quickstart: "Quick start",
    nav_cli: "CLI",
    nav_privacy: "Privacy",
    nav_menu: "Menu",
    nav_download: "Download",
    nav_skip: "Skip to content",

    hero_alt: "Brocolis mascot: a broccoli with hat and sunglasses in front of a laptop",
    hero_tagline: "Rust MITM proxy for <strong>selective encryption</strong> of your web traffic.",
    hero_lead: "Mark <code class=\"inv\">((your secret))</code> in your messages: Brocolis encrypts it with AES-256-GCM before it leaves your machine, and decrypts it back for users who share your key. Twitter, forums, messengers — with nothing to change on the server side.",
    hero_cta: "Download",
    hero_cta_sub: "macOS · Linux · Windows",

    switch_lang: "Language",
    switch_theme: "Theme",
    theme_green: "Green",
    theme_light: "Light",
    theme_dark: "Dark",

    platforms_h: "Supported platforms",
    platforms_intro: "Every download includes the <strong>Brocolis app</strong> — one switch, your contacts, your messages — and the command-line tool. <code>brocolis auto</code> detects the OS and runs the right system setup. Download the build for your platform:",
    platforms_macos_t: "macOS",
    platforms_macos_d: "System keychain (Safari, Chrome, Edge) + Firefox enterprise policy + alternative <code>.mobileconfig</code> profile + web/HTTPS proxies via <code>networksetup</code>. Universal binary Intel + Apple Silicon.",
    platforms_macos_cta: "Download for macOS",
    platforms_linux_t: "Linux",
    platforms_linux_d: "Distro trust store (Debian/Ubuntu, Fedora/RHEL, Arch detected via their tools) + Firefox enterprise policy + <code>http_proxy</code> variables in <code>/etc/profile.d/</code>. Also available as <code>.deb</code> and <code>.rpm</code> packages.",
    platforms_linux_cta: "Download for Linux",
    platforms_windows_t: "Windows",
    platforms_windows_d: "<em>Local Machine\\Trusted Root</em> store via <code>certutil</code> + Firefox enterprise policy + WinHTTP proxy via <code>netsh</code>. PowerShell scripts <code>install.ps1</code> / <code>uninstall.ps1</code> for persistent installs.",
    platforms_windows_cta: "Download for Windows",

    dl_back: "Home",
    dl_h: "Download Brocolis",
    dl_intro: "Pick your operating system. Then open the <strong>Brocolis app</strong> and turn protection on with one click — or run <code>brocolis auto</code> from the command line.",
    dl_version: "These are the <strong>latest builds</strong>, recompiled automatically on every update. For a specific version or the history, see all releases.",
    dl_macos_t: "macOS",
    dl_macos_d: "<code>Brocolis.app</code> and the command-line tool, universal Intel + Apple Silicon (<code>.tar.gz</code>).",
    dl_macos_btn: "Download (.tar.gz)",
    dl_macos_note: "Unpack and drag <code>Brocolis.app</code> into Applications. First launch: right-click → Open (unsigned app).",
    dl_linux_t: "Linux",
    dl_linux_d: "x86_64 packages: <code>.deb</code> for Debian/Ubuntu, <code>.rpm</code> for Fedora/RHEL. Desktop app and command-line tool.",
    dl_linux_deb: "Download .deb",
    dl_linux_rpm: "Download .rpm",
    dl_linux_note: "Install the package, then open <strong>Brocolis</strong> from the applications menu (or <code>sudo brocolis on</code>).",
    dl_windows_t: "Windows",
    dl_windows_d: "<code>brocolis-app.exe</code> app, command-line tool and install scripts, x86_64 (<code>.zip</code>).",
    dl_windows_btn: "Download (.zip)",
    dl_windows_note: "Unzip, then run <code>brocolis-app.exe</code> (keep <code>brocolis.exe</code> in the same folder).",
    dl_all: "See all versions on GitHub →",

    s1_h: "One command, zero configuration",
    s1_li: [
        "Admin password requested <strong>only once</strong> (<code>sudo</code> on macOS/Linux, UAC elevation on Windows)",
        "Trusts the CA in the native trust store: <strong>System keychain</strong> (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust extract-compat</code> (Linux), <code>certutil -addstore Root</code> (Windows)",
        "Enables enterprise trust for <strong>Firefox</strong> (all detected variants). On macOS, also generates a <code>brocolis-firefox.mobileconfig</code> profile that doesn't modify the Firefox bundle",
        "Sets the system proxy: <code>networksetup</code> (macOS), <code>/etc/profile.d/brocolis-proxy.sh</code> with <code>http_proxy</code>/<code>https_proxy</code> (Linux), <code>netsh winhttp set proxy</code> (Windows)",
        "Starts AES-256-GCM selective encryption",
        "<strong>Restores the exact original state</strong> on exit (Ctrl-C): CA removed, policies/profile/variables removed, proxy reset",
    ],
    s1_note: "Without admin elevation, <code>brocolis auto</code> starts the proxy <strong>without touching the system</strong> (equivalent to <code>--no-system</code>): point your browser at <code>127.0.0.1:&lt;port&gt;</code> manually.",

    s2_h: "Features",
    s2_cards: [
        Card {
            title: "Selective encryption",
            body: "Only segments wrapped in <code>((double parentheses))</code> are encrypted, with AES-256-GCM, as a <code>[[BROCOLIS:v1:…]]</code> token. The remote server only ever stores ciphertext. In the app, without parentheses, the <strong>whole message</strong> is encrypted.",
        },
        Card {
            title: "Transparent MITM",
            body: "HTTPS interception via <code>CONNECT</code>, leaf certificates forged on the fly by an internal authority (<em>rcgen</em>), client-side TLS terminated with <em>rustls</em>.",
        },
        Card {
            title: "Contacts and keys",
            body: "<strong>One contact, one key</strong> (AES-256), created in the app or obtained from a code agreed in person. Encrypt for one contact or several at once — a single token each opens with their own key — and know who a message comes from.",
        },
        Card {
            title: "Multi-browser",
            body: "Safari, Chrome, Edge, Chromium and all Firefox variants trust the CA automatically — via the native OS trust store and Firefox enterprise policy.",
        },
    ],

    s3_h: "Quick start",
    s3_li: [
        "Download the build for your platform from the section above (universal macOS <code>.tar.gz</code>, Linux <code>.deb</code>/<code>.rpm</code>, or Windows <code>.zip</code> x86_64).",
        "Run <code>sudo brocolis auto</code> (macOS / Linux) or <code>brocolis.exe auto</code> from a <em>Run as administrator</em> terminal (Windows). The admin password is requested <strong>only once</strong>.",
        "Type your messages marking what must stay private: <code>((my secret))</code>. Correspondents who share the key see the clear text; everyone else sees only ciphertext.",
        "<code>Ctrl-C</code> to stop everything: Brocolis returns your system to its exact original state.",
    ],

    s4_h: "Build from source &amp; manual CLI",
    s4_intro: "To build it yourself or drive everything by hand (commands are identical on all three platforms):",
    s4_code: "# Build (stable Rust, from source)\ncargo build --release          # binary: target/release/brocolis\n\n# 1. Generate an AES-256 symmetric key (share it over a secure channel)\n$ brocolis keygen\nSomopQcHJyVQQAWedZFGY5E4+hxKjZemOB7aGnYr6kw=\n\n# 2. Export the root certificate to install it in the browser\n$ brocolis ca --export brocolis-ca.crt\n\n# 3. Start the proxy (key set in brocolis.toml, see brocolis.example.toml)\n$ brocolis run --config brocolis.toml\nBrocolis listening on http://127.0.0.1:8080 (HTTP/HTTPS proxy)",
    s4_outro: "On Unix, key management is driven by signals: <code>SIGHUP</code> regenerates the key (rotation on compromise), <code>SIGUSR1</code> reloads the shared key from the config file.",

    s5_h: "How it works",
    s5_intro: "Brocolis sits between your browser and the online service. On the way out, it encrypts marked segments; on the way back, it decrypts tokens readable with the shared key.",
    s5_diagram: "Browser ─►  Brocolis (local proxy)  ─►  Remote server\n                    │                              │\n   \"hi ((secret))\" ───► \"hi [[BROCOLIS:v1:…]]\" (encrypted on the wire)\n   \"hi secret\"     ◄─── \"hi [[BROCOLIS:v1:…]]\" (decrypted for you)",

    s6_h: "Privacy and security",
    s6_li: [
        "<strong>End-to-end</strong> encryption between users sharing the key — the remote service never sees the clear text.",
        "No telemetry, no server, no account — everything runs locally.",
        "The CA is <strong>removed from the trust store</strong> automatically on exit if Brocolis added it.",
        "Source code available for audit (proprietary-licensed software).",
    ],

    footer: "Brocolis — Rust MITM proxy. Designed and developed by Maxime Nathan Lestage.",
    hero_eyebrow: "MITM proxy · Rust · AES-256-GCM",
    hero_scroll: "Scroll",
    manifesto_label: "Manifesto",
    manifesto: "Your words stay yours. Brocolis encrypts what you mark before it leaves your machine, and leaves everything else untouched. No server to convince, no app to change.",
    marquee: [
        "Selective encryption",
        "AES-256-GCM",
        "Transparent MITM",
        "Zero configuration",
        "Written in Rust",
        "macOS · Linux · Windows",
    ],
    footer_h: "Ready to encrypt <em>what matters</em>?",
    footer_top: "Back to top",
    demo_kicker: "Demo",
    demo_h: "One message, four readers",
    demo_sr: "Demo: you write \"Meet me ((tomorrow at 6pm)) at the café.\" The server only stores the encrypted token in place of the marked segment; friends who share the key read \"tomorrow at 6pm\" in clear; everyone else only sees the token.",
    demo_rows: [
        "You write",
        "The server stores",
        "Your friends read",
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
    meta_title: "Brocolis 🥦 — Proxy MITM en Rust para cifrado selectivo",
    meta_desc: "Proxy MITM en Rust que cifra selectivamente el tráfico HTTP/HTTPS en Twitter, foros y mensajerías. Configuración automática en macOS, Linux y Windows — sin cambiar nada del lado del servidor.",
    meta_og_title: "Brocolis 🥦 — Cifrado selectivo de tu tráfico web",
    meta_og_desc: "Un comando, cero configuración. AES-256-GCM en los segmentos que marques. macOS, Linux, Windows.",

    nav_platforms: "Plataformas",
    nav_features: "Características",
    nav_quickstart: "Inicio rápido",
    nav_cli: "CLI",
    nav_privacy: "Seguridad",
    nav_menu: "Menú",
    nav_download: "Descargar",
    nav_skip: "Ir al contenido",

    hero_alt: "Mascota de Brocolis: un brócoli con sombrero y gafas de sol frente a un portátil",
    hero_tagline: "Proxy MITM en Rust para el <strong>cifrado selectivo</strong> de tu tráfico web.",
    hero_lead: "Marca <code class=\"inv\">((tu secreto))</code> en tus mensajes: Brocolis lo cifra con AES-256-GCM antes de que salga de tu máquina, y lo descifra para los usuarios que comparten tu clave. Twitter, foros, mensajerías — sin cambiar nada del lado del servidor.",
    hero_cta: "Descargar",
    hero_cta_sub: "macOS · Linux · Windows",

    switch_lang: "Idioma",
    switch_theme: "Tema",
    theme_green: "Verde",
    theme_light: "Claro",
    theme_dark: "Oscuro",

    platforms_h: "Plataformas soportadas",
    platforms_intro: "Cada descarga incluye la <strong>aplicación Brocolis</strong> — un interruptor, tus contactos, tus mensajes — y la línea de comandos. <code>brocolis auto</code> detecta el SO y hace la configuración correcta. Descarga la build para tu plataforma:",
    platforms_macos_t: "macOS",
    platforms_macos_d: "Llavero del Sistema (Safari, Chrome, Edge) + política empresarial Firefox + perfil <code>.mobileconfig</code> alternativo + proxies web/HTTPS vía <code>networksetup</code>. Binario universal Intel + Apple Silicon.",
    platforms_macos_cta: "Descargar para macOS",
    platforms_linux_t: "Linux",
    platforms_linux_d: "Almacén de confianza distro (Debian/Ubuntu, Fedora/RHEL, Arch detectados vía sus herramientas) + política empresarial Firefox + variables <code>http_proxy</code> en <code>/etc/profile.d/</code>. También disponible como paquetes <code>.deb</code> y <code>.rpm</code>.",
    platforms_linux_cta: "Descargar para Linux",
    platforms_windows_t: "Windows",
    platforms_windows_d: "Almacén <em>Local Machine\\Trusted Root</em> vía <code>certutil</code> + política empresarial Firefox + proxy WinHTTP vía <code>netsh</code>. Scripts PowerShell <code>install.ps1</code> / <code>uninstall.ps1</code> para instalaciones persistentes.",
    platforms_windows_cta: "Descargar para Windows",

    dl_back: "Inicio",
    dl_h: "Descargar Brocolis",
    dl_intro: "Elige tu sistema operativo. Abre luego la <strong>aplicación Brocolis</strong> y activa la protección con un clic — o ejecuta <code>brocolis auto</code> en la línea de comandos.",
    dl_version: "Estas son las <strong>últimas builds</strong>, recompiladas automáticamente en cada actualización. Para una versión concreta o el historial, mira todas las releases.",
    dl_macos_t: "macOS",
    dl_macos_d: "Aplicación <code>Brocolis.app</code> y línea de comandos, universales Intel + Apple Silicon (<code>.tar.gz</code>).",
    dl_macos_btn: "Descargar (.tar.gz)",
    dl_macos_note: "Descomprime y arrastra <code>Brocolis.app</code> a Aplicaciones. Primer inicio: clic derecho → Abrir (aplicación sin firmar).",
    dl_linux_t: "Linux",
    dl_linux_d: "Paquetes x86_64: <code>.deb</code> para Debian/Ubuntu, <code>.rpm</code> para Fedora/RHEL. Aplicación de escritorio y línea de comandos.",
    dl_linux_deb: "Descargar .deb",
    dl_linux_rpm: "Descargar .rpm",
    dl_linux_note: "Instala el paquete y abre <strong>Brocolis</strong> desde el menú de aplicaciones (o <code>sudo brocolis on</code>).",
    dl_windows_t: "Windows",
    dl_windows_d: "Aplicación <code>brocolis-app.exe</code>, línea de comandos y scripts de instalación, x86_64 (<code>.zip</code>).",
    dl_windows_btn: "Descargar (.zip)",
    dl_windows_note: "Descomprime y ejecuta <code>brocolis-app.exe</code> (deja <code>brocolis.exe</code> en la misma carpeta).",
    dl_all: "Ver todas las versiones en GitHub →",

    s1_h: "Un comando, cero configuración",
    s1_li: [
        "Contraseña de administrador solicitada <strong>una sola vez</strong> (<code>sudo</code> en macOS/Linux, elevación UAC en Windows)",
        "Aprueba el CA en el almacén de confianza nativo: <strong>llavero del Sistema</strong> (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust extract-compat</code> (Linux), <code>certutil -addstore Root</code> (Windows)",
        "Activa la confianza empresarial para <strong>Firefox</strong> (todas las variantes detectadas). En macOS, también genera un perfil <code>brocolis-firefox.mobileconfig</code> que no modifica el bundle de Firefox",
        "Configura el proxy del sistema: <code>networksetup</code> (macOS), <code>/etc/profile.d/brocolis-proxy.sh</code> con <code>http_proxy</code>/<code>https_proxy</code> (Linux), <code>netsh winhttp set proxy</code> (Windows)",
        "Inicia el cifrado selectivo AES-256-GCM",
        "<strong>Restaura exactamente el estado inicial</strong> al salir (Ctrl-C): CA retirado, políticas/perfil/variables retiradas, proxy restaurado",
    ],
    s1_note: "Sin elevación de administrador, <code>brocolis auto</code> arranca el proxy <strong>sin tocar el sistema</strong> (equivalente a <code>--no-system</code>): apunta tu navegador a <code>127.0.0.1:&lt;puerto&gt;</code> manualmente.",

    s2_h: "Características",
    s2_cards: [
        Card {
            title: "Cifrado selectivo",
            body: "Solo los segmentos <code>((entre dobles paréntesis))</code> se cifran, con AES-256-GCM, como un token <code>[[BROCOLIS:v1:…]]</code>. El servidor remoto solo almacena texto cifrado. En la aplicación, sin paréntesis, se cifra <strong>todo el mensaje</strong>.",
        },
        Card {
            title: "MITM transparente",
            body: "Interceptación HTTPS vía <code>CONNECT</code>, certificados hoja forjados al vuelo por una autoridad interna (<em>rcgen</em>), TLS terminado del lado cliente con <em>rustls</em>.",
        },
        Card {
            title: "Contactos y claves",
            body: "<strong>Un contacto, una clave</strong> AES-256, creada en la aplicación u obtenida con un código acordado en persona. Cifra para un contacto o para varios a la vez — un solo token, que cada uno abre con su clave — y sabe de quién viene un mensaje.",
        },
        Card {
            title: "Multi-navegador",
            body: "Safari, Chrome, Edge, Chromium y todas las variantes de Firefox confían en el CA automáticamente — vía el almacén de confianza nativo del SO y la política empresarial Firefox.",
        },
    ],

    s3_h: "Inicio rápido",
    s3_li: [
        "Descarga la build para tu plataforma desde la sección de arriba (<code>.tar.gz</code> macOS universal, <code>.deb</code>/<code>.rpm</code> Linux, o <code>.zip</code> Windows x86_64).",
        "Ejecuta <code>sudo brocolis auto</code> (macOS / Linux) o <code>brocolis.exe auto</code> desde un terminal <em>Ejecutar como administrador</em> (Windows). La contraseña de administrador solo se pide <strong>una vez</strong>.",
        "Escribe tus mensajes marcando lo que debe quedar privado: <code>((mi secreto))</code>. Los corresponsales que comparten la clave ven el texto en claro; el resto solo ve cifrado.",
        "<code>Ctrl-C</code> para detener todo: Brocolis devuelve tu sistema exactamente a su estado inicial.",
    ],

    s4_h: "Compilar desde las fuentes y CLI manual",
    s4_intro: "Para compilarlo tú mismo o controlarlo todo a mano (los comandos son idénticos en las tres plataformas):",
    s4_code: "# Compilar (Rust estable, desde las fuentes)\ncargo build --release          # binario: target/release/brocolis\n\n# 1. Generar una clave simétrica AES-256 (a compartir por un canal seguro)\n$ brocolis keygen\nSomopQcHJyVQQAWedZFGY5E4+hxKjZemOB7aGnYr6kw=\n\n# 2. Exportar el certificado raíz para instalarlo en el navegador\n$ brocolis ca --export brocolis-ca.crt\n\n# 3. Lanzar el proxy (clave en brocolis.toml, cf. brocolis.example.toml)\n$ brocolis run --config brocolis.toml\nBrocolis escuchando en http://127.0.0.1:8080 (proxy HTTP/HTTPS)",
    s4_outro: "En Unix, la gestión de claves se controla por señales: <code>SIGHUP</code> regenera la clave (rotación tras compromiso), <code>SIGUSR1</code> recarga la clave compartida desde el archivo de configuración.",

    s5_h: "Cómo funciona",
    s5_intro: "Brocolis se interpone entre tu navegador y el servicio en línea. Al salir, cifra los segmentos marcados; al volver, descifra los tokens legibles con la clave compartida.",
    s5_diagram: "Navegador ─►  Brocolis (proxy local)  ─►  Servidor remoto\n                     │                              │\n   «hola ((secreto))» ───► «hola [[BROCOLIS:v1:…]]» (cifrado en el cable)\n   «hola secreto»     ◄─── «hola [[BROCOLIS:v1:…]]» (descifrado para ti)",

    s6_h: "Privacidad y seguridad",
    s6_li: [
        "Cifrado <strong>de extremo a extremo</strong> entre usuarios que comparten la clave — el servicio remoto nunca ve el texto en claro.",
        "Sin telemetría, sin servidor, sin cuenta — todo se ejecuta localmente.",
        "El CA se <strong>retira del almacén de confianza</strong> automáticamente al salir si Brocolis lo añadió.",
        "Código fuente consultable para auditoría (software bajo licencia propietaria).",
    ],

    footer: "Brocolis — proxy MITM en Rust. Diseñado y desarrollado por Maxime Nathan Lestage.",
    hero_eyebrow: "Proxy MITM · Rust · AES-256-GCM",
    hero_scroll: "Desplazar",
    manifesto_label: "Manifiesto",
    manifesto: "Tus palabras siguen siendo tuyas. Brocolis cifra lo que marcas antes de que salga de tu máquina, y deja todo lo demás intacto. Ningún servidor que convencer, ninguna aplicación que cambiar.",
    marquee: [
        "Cifrado selectivo",
        "AES-256-GCM",
        "MITM transparente",
        "Cero configuración",
        "Escrito en Rust",
        "macOS · Linux · Windows",
    ],
    footer_h: "¿Listo para cifrar <em>lo que importa</em>?",
    footer_top: "Volver arriba",
    demo_kicker: "Demostración",
    demo_h: "Un mensaje, cuatro lectores",
    demo_sr: "Demostración: escribes «Nos vemos ((mañana a las 18h)) en el café.» El servidor solo almacena el token cifrado en lugar del segmento marcado; tus contactos que comparten la clave leen «mañana a las 18h» en claro; los demás solo ven el token.",
    demo_rows: [
        "Tú escribes",
        "El servidor guarda",
        "Tus contactos leen",
        "Los demás ven",
    ],
    demo_before: "Nos vemos ",
    demo_secret: "mañana a las 18h",
    demo_after: " en el café.",
    demo_locked: "cifrado",
    demo_unlocked: "descifrado",
};
