//! Les textes de la page « Protocole v3 » (`#/protocole`), dans les trois
//! langues. À part de `i18n` parce qu'ils sont longs ; même principe : une
//! valeur par langue, du même type, donc une traduction oubliée ne compile
//! pas.
//!
//! Le contenu suit `docs/SECURITE.md` du dépôt de la messagerie (§1, §4, §7,
//! §8) : ce qui est écrit ici est ce que le code fait.

/// Une ligne du comparatif ancien format / v3.
pub struct Row {
    pub label: &'static str,
    pub old: &'static str,
    pub new: &'static str,
}

/// Une étape du déroulé.
pub struct Step {
    pub title: &'static str,
    pub body: &'static str,
    /// Le détail technique (balisage autorisé, formules en `<code>`).
    pub detail: &'static str,
}

/// Un champ du jeton v3.
pub struct Field {
    pub name: &'static str,
    pub size: &'static str,
    pub desc: &'static str,
    /// Seulement dans un premier message (type 0x01).
    pub first_only: bool,
}

/// Une ligne du modèle de menace.
pub struct Threat {
    pub who: &'static str,
    pub can: &'static str,
    pub we: &'static str,
}

/// Une plateforme et ce qu'elle fait du v3.
pub struct Platform {
    pub name: &'static str,
    pub status: &'static str,
}

/// Une question fréquente.
pub struct Faq {
    pub q: &'static str,
    pub a: &'static str,
}

/// Les messages du simulateur, dans l'ordre où ils partent.
pub const SIM_LINES: usize = 8;

pub struct Proto {
    pub meta_title: &'static str,
    pub nav: &'static str,
    pub cta: &'static str,
    pub back: &'static str,
    pub kicker: &'static str,
    pub title: &'static str,
    pub lead: &'static str,
    pub toc: [&'static str; 7],

    pub cmp_h: &'static str,
    pub cmp_intro: &'static str,
    pub cmp_cols: [&'static str; 3],
    pub cmp_rows: [Row; 7],

    pub sim_h: &'static str,
    pub sim_intro: &'static str,
    pub sim_you: &'static str,
    pub sim_alice: &'static str,
    pub sim_send: &'static str,
    pub sim_reply: &'static str,
    pub sim_spy: &'static str,
    pub sim_reset: &'static str,
    pub sim_empty: &'static str,
    pub sim_key: &'static str,
    pub sim_erased: &'static str,
    pub sim_turn: &'static str,
    pub sim_spied: &'static str,
    pub sim_exposed: &'static str,
    pub sim_safe_past: &'static str,
    pub sim_safe_healed: &'static str,
    /// `{n}` : messages que l'espion lit ; `{total}` : messages envoyés.
    pub sim_score: &'static str,
    pub sim_note: &'static str,
    /// Vos lignes, puis celles d'Alice, dans l'ordre où elles partent.
    pub sim_you_texts: [&'static str; SIM_LINES],
    pub sim_alice_texts: [&'static str; SIM_LINES],

    pub steps_h: &'static str,
    pub steps: [Step; 5],

    pub token_h: &'static str,
    pub token_intro: &'static str,
    pub token_first: &'static str,
    pub token_fields: [Field; 9],
    pub token_note: &'static str,

    pub threat_h: &'static str,
    pub threat_cols: [&'static str; 3],
    pub threats: [Threat; 6],

    pub where_h: &'static str,
    pub where_intro: &'static str,
    pub platforms: [Platform; 4],

    pub labels_h: &'static str,
    pub labels_intro: &'static str,
    pub labels: [Row; 4],

    pub limits_h: &'static str,
    pub limits: [&'static str; 5],

    pub faq_h: &'static str,
    pub faqs: [Faq; 7],

    pub spec_h: &'static str,
    pub spec_body: &'static str,
    pub spec_cta: &'static str,

    /// Le v4 (`docs/SECURITE.md` §9) : cartes (`q` titre, `a` texte),
    /// comparatif Signal / v3 / v4, prix et honnêteté sur l'audit.
    pub toc_v4: &'static str,
    pub v4_h: &'static str,
    pub v4_intro: &'static str,
    pub v4_cols: [&'static str; 4],
    pub v4_rows: [[&'static str; 4]; 6],
    pub v4_cards: [Faq; 4],
    pub v4_cost: &'static str,
    pub v4_honest: &'static str,
    /// Les libellés v4, en tête de la liste des libellés.
    pub labels_v4: [Row; 2],
}

// ---------------------------------------------------------------------------
// Français
// ---------------------------------------------------------------------------

pub static FR: Proto = Proto {
    meta_title: "Protocole v3 : le double cliquet — Brocolis 🥦",
    nav: "Protocole v3",
    cta: "Tout comprendre du protocole v3",
    back: "Accueil",
    kicker: "Protocole v3",
    title: "Une clé neuve pour chaque message",
    lead: "Avec un contact appairé, Brocolis ne chiffre plus vos <code>((secrets))</code> avec une clé fixe. Il ouvre une <strong>session à double cliquet</strong>, comme Signal : chaque message a sa propre clé, effacée aussitôt, et la session s'établit en <strong>hybride X25519 + ML-KEM-768</strong>, à l'épreuve de l'ordinateur quantique de demain. C'est le protocole de la messagerie Brocolis — iPhone, Android, discussion du site — et brocolis le parle désormais sur macOS, Linux et Windows.",
    toc: [
        "Ce qui change",
        "Essayez le cliquet",
        "Pas à pas",
        "Anatomie d'un jeton",
        "Contre qui",
        "Où, et ce qui s'affiche",
        "Questions",
    ],

    cmp_h: "Ce qui change",
    cmp_intro: "L'ancien format (<code>v1</code>/<code>v2</code>) reste lisible pour toujours, et sert encore aux groupes. Mais à deux, la différence est nette :",
    cmp_cols: ["", "Ancien format (v1)", "Double cliquet (v3)"],
    cmp_rows: [
        Row {
            label: "La clé",
            old: "Une seule, la même pour tous les messages",
            new: "Une neuve par message, effacée aussitôt utilisée",
        },
        Row {
            label: "Une clé volée plus tard",
            old: "Ouvre tout l'historique gardé par le serveur",
            new: "N'ouvre rien de ce qui est déjà parti (confidentialité persistante)",
        },
        Row {
            label: "Un espion passager",
            old: "Lit tout, tant que la clé n'est pas changée",
            new: "Perd la main au premier aller-retour (la session guérit)",
        },
        Row {
            label: "L'ordinateur quantique",
            old: "Hors sujet (AES-256 seul), mais l'appairage X25519 est cassable",
            new: "Établissement hybride X25519 + ML-KEM-768 : il faut casser les deux",
        },
        Row {
            label: "La longueur du message",
            old: "Se lit sur la taille du jeton",
            new: "Masquée : complétée au multiple de 160 octets",
        },
        Row {
            label: "Un message rejoué",
            old: "Se déchiffre à nouveau",
            new: "Reconnu et ignoré",
        },
        Row {
            label: "L'empreinte d'appairage",
            old: "Porte sur la clé partagée",
            new: "Porte sur la clé et les deux identités : un paquet substitué se voit",
        },
    ],

    sim_h: "Essayez le cliquet",
    sim_intro: "Écrivez à Alice, laissez-la répondre, puis faites entrer un espion qui copie l'état de votre ordinateur à un instant donné. Observez ce qu'il peut lire : rien de ce qui est parti avant, et plus rien dès que le cliquet a tourné de votre côté.",
    sim_you: "Vous",
    sim_alice: "Alice",
    sim_send: "Vous écrivez",
    sim_reply: "Alice répond",
    sim_spy: "Un espion copie l'état",
    sim_reset: "Recommencer",
    sim_empty: "Aucun message pour l'instant : écrivez à Alice.",
    sim_key: "clé",
    sim_erased: "effacée",
    sim_turn: "Le cliquet tourne : nouvel échange Diffie-Hellman",
    sim_spied: "Ici, un espion copie l'état",
    sim_exposed: "lisible par l'espion",
    sim_safe_past: "hors de portée : sa clé était déjà effacée",
    sim_safe_healed: "hors de portée : la session a guéri",
    sim_score: "L'espion lit {n} message(s) sur {total}.",
    sim_note: "Simulation pédagogique : les « clés » affichées sont des repères, pas les vraies. Le vrai protocole fait la même chose avec HKDF-SHA256, HMAC-SHA256 et X25519 (et ML-KEM-768 à l'établissement).",
    sim_you_texts: [
        "Salut Alice !",
        "On se voit demain ?",
        "Le code de la porte : 4291",
        "N'oublie pas le dossier.",
        "Je prends le train de 17h.",
        "Tu as reçu ma photo ?",
        "Je t'appelle ce soir.",
        "Bonne nuit 🥦",
    ],
    sim_alice_texts: [
        "Coucou 🥦",
        "Oui, 18h au café ?",
        "Noté, merci !",
        "Promis !",
        "Je viens te chercher.",
        "Pas encore, réessaie.",
        "Ça marche.",
        "À demain !",
    ],

    steps_h: "Pas à pas",
    steps: [
        Step {
            title: "1. Chaque appareil a son paquet de clés",
            body: "Une identité longue durée, une clé pré-signée renouvelée chaque semaine et une clé ML-KEM-768. Le tout est signé par l'identité et publié à l'appairage, dans le salon de rendez-vous, avec la clé publique habituelle.",
            detail: "<code>IK X25519 ‖ IK Ed25519 ‖ SPK ‖ PQPK ‖ signatures Ed25519</code> — 166 octets, ou 1 418 avec ML-KEM.",
        },
        Step {
            title: "2. L'appairage vérifie les deux identités",
            body: "Les deux côtés dérivent la même clé partagée, comme avant. Mais l'empreinte à comparer de vive voix couvre désormais la clé et les deux identités : un serveur qui glisserait son propre paquet ferait afficher deux empreintes différentes.",
            detail: "<code>SHA-256(\"Brocolis_v3_Appairage\" ‖ K ‖ min(P_A, P_B) ‖ max(P_A, P_B))[0..8]</code>",
        },
        Step {
            title: "3. Le premier message établit la session",
            body: "Comme Signal PQXDH : trois échanges Diffie-Hellman et une encapsulation ML-KEM-768 sont mélangés en une clé de départ. Ce premier message porte de quoi la reconstruire de l'autre côté — Alice n'a pas besoin d'être en ligne.",
            detail: "<code>SK = HKDF(32 × 0x00, 32 × 0xFF ‖ DH1 ‖ DH2 ‖ DH3 ‖ SS_ML-KEM, \"Brocolis_v3_X25519_MLKEM768_SHA-256\")</code>",
        },
        Step {
            title: "4. Le cliquet avance à chaque message",
            body: "Chaque message dérive sa clé de la précédente, puis l'efface : impossible de revenir en arrière. À chaque réponse, un nouvel échange Diffie-Hellman fait tourner le grand cliquet, et un état volé ne sert plus à rien.",
            detail: "<code>mk = HMAC(ck, 0x01)</code>, <code>ck' = HMAC(ck, 0x02)</code>, <code>(RK, CK) = HKDF(RK, DH(cliquet, cliquet distant))</code>",
        },
        Step {
            title: "5. La réception tolère le désordre, pas le rejeu",
            body: "Un message peut arriver en retard ou en double — copier-coller, page rechargée. Les clés sautées sont gardées (1 000 au plus, 30 jours), un message rejoué ne trouve plus sa clé et il est ignoré. Le clair d'un message lu est gardé sur votre appareil : la page se recharge sans rien perdre.",
            detail: "Déchiffrement transactionnel : un échec laisse l'état exactement tel qu'il était.",
        },
    ],

    token_h: "Anatomie d'un jeton",
    token_intro: "Un jeton <code>[[BROCOLIS:v3:…]]</code> est du base64 standard ; voici ce qu'il contient, octet par octet. Le serveur du site ne voit que cette suite : rien ne dit qui écrit ni ce qui est écrit.",
    token_first: "premier message seulement",
    token_fields: [
        Field {
            name: "type",
            size: "1",
            desc: "0x01 premier message, 0x02 message",
            first_only: false,
        },
        Field {
            name: "session",
            size: "8",
            desc: "SHA-256(IK_A ‖ IK_B), tronqué",
            first_only: false,
        },
        Field {
            name: "IK_A",
            size: "32",
            desc: "identité X25519 de l'expéditeur",
            first_only: true,
        },
        Field {
            name: "EK_A",
            size: "32",
            desc: "clé éphémère",
            first_only: true,
        },
        Field {
            name: "mode",
            size: "1",
            desc: "X25519 seul, hybride ML-KEM, ou clé partagée",
            first_only: true,
        },
        Field {
            name: "CT",
            size: "1 088",
            desc: "encapsulation ML-KEM-768",
            first_only: true,
        },
        Field {
            name: "SPK · PQPK",
            size: "4 + 4",
            desc: "quelles clés pré-signées ont servi",
            first_only: true,
        },
        Field {
            name: "en-tête",
            size: "40",
            desc: "clé de cliquet (32), n° de chaîne, n° de message",
            first_only: false,
        },
        Field {
            name: "chiffré",
            size: "160·k + 16",
            desc: "AES-256-GCM du texte complété, et son tag",
            first_only: false,
        },
    ],
    token_note: "Toute autre longueur est refusée, toute clé publique non canonique aussi, et l'en-tête est authentifié avec le chiffré : modifier un seul octet fait échouer le déchiffrement.",

    threat_h: "Contre qui",
    threat_cols: ["Qui", "Ce qu'il peut faire", "Ce que vous gardez"],
    threats: [
        Threat {
            who: "Le service en ligne (Twitter, un forum…)",
            can: "Lire, garder, recopier tout ce que vous publiez",
            we: "Il ne stocke que des jetons ; un jeton modifié est refusé, un jeton rejoué ignoré",
        },
        Threat {
            who: "Le serveur d'appairage",
            can: "Substituer des clés publiques ou des paquets",
            we: "L'empreinte comparée de vive voix le trahit",
        },
        Threat {
            who: "Qui garde vos jetons des années",
            can: "Voler une clé plus tard",
            we: "Les messages passés restent fermés : leurs clés n'existent plus",
        },
        Threat {
            who: "L'ordinateur quantique de demain",
            can: "Casser X25519 rétroactivement",
            we: "Il faudrait aussi casser ML-KEM-768",
        },
        Threat {
            who: "Un logiciel espion de passage",
            can: "Copier l'état des sessions à un instant",
            we: "La session guérit dès la réponse suivante",
        },
        Threat {
            who: "Qui regarde la taille des jetons",
            can: "Deviner la longueur d'un message",
            we: "Tout est complété au multiple de 160 octets",
        },
    ],

    where_h: "Où, et ce qui s'affiche",
    where_intro: "Un seul protocole, une implémentation de référence en Rust, et des vecteurs de test que chaque plateforme retrouve octet pour octet : un message chiffré sur un ordinateur s'ouvre sur un iPhone, et inversement.",
    platforms: [
        Platform {
            name: "brocolis — macOS, Linux, Windows",
            status: "Proxy, application et ligne de commande : appairage, envoi et réception en v3, hybride ML-KEM-768",
        },
        Platform {
            name: "iPhone",
            status: "App, clavier, Partager, iMessage ; hybride dès iOS 26, X25519 seul avant",
        },
        Platform {
            name: "Android",
            status: "App, Partager, messagerie ; hybride ML-KEM-768",
        },
        Platform {
            name: "Discussion du site de la messagerie",
            status: "Dans le navigateur, sessions gardées au coffre ; hybride ML-KEM-768",
        },
    ],
    labels_h: "Chaque contact dit comment part le prochain message",
    labels_intro: "Dans l'onglet Contacts de l'application, <code>brocolis contacts</code> et toutes les apps de la messagerie :",
    labels: [
        Row {
            label: "🔐 double cliquet (post-quantique)",
            old: "",
            new: "Contact appairé, session hybride X25519 + ML-KEM-768",
        },
        Row {
            label: "🔐 double cliquet (classique)",
            old: "",
            new: "Contact appairé dont l'appareil ne sait pas encore faire ML-KEM",
        },
        Row {
            label: "🔐 double cliquet, clé partagée (PSK)",
            old: "",
            new: "Pas d'appairage, mais vous avez tous deux une version à jour : le cliquet part de la clé partagée",
        },
        Row {
            label: "ancien format",
            old: "",
            new: "Groupe, plusieurs destinataires ou correspondant pas encore à jour : v1/v2",
        },
    ],

    limits_h: "Ce que le v3 ne fait pas",
    limits: [
        "<strong>Une clé de groupe</strong> (connue de trois personnes ou plus) reste en v1/v2. Les <strong>groupes à cliquet</strong> (Sender Keys) fonctionnent dans brocolis : <code>brocolis group-create</code>, puis <code>encrypt --group</code> ; dans les applications, l'écran pour en créer est à venir.",
        "<strong>La page déchiffrée</strong> reste lisible par le JavaScript du site consulté : c'est le principe d'un proxy qui déchiffre dans la page.",
        "<strong>Les métadonnées</strong> du service (qui publie quoi, quand) ne changent pas.",
        "<strong>Un appareil déverrouillé</strong> entre de mauvaises mains lit ce que vous lisez.",
        "<strong>Un contact sans nouvelles depuis 30 jours</strong> sans session ouverte repasse en ancien format, jusqu'au prochain appairage.",
    ],

    faq_h: "Questions",
    faqs: [
        Faq {
            q: "Dois-je faire quelque chose ?",
            a: "Appairez-vous avec vos contacts (onglet Contacts, ou <code>brocolis pair --name</code>). S'ils ont une version à jour, le contact passe en double cliquet tout seul, et l'empreinte à comparer couvre vos deux appareils.",
        },
        Faq {
            q: "Mes anciens messages ?",
            a: "Ils restent lisibles : les jetons v1 et v2 s'ouvrent pour toujours, avec les mêmes clés.",
        },
        Faq {
            q: "Et si mon correspondant n'est pas à jour ?",
            a: "Rien ne casse : sans paquet de clés de son côté, l'appairage et les messages restent en ancien format, et l'application le dit.",
        },
        Faq {
            q: "Pourquoi un message ne se rouvre-t-il qu'une fois ?",
            a: "Parce que sa clé est effacée dès qu'il est lu. Brocolis garde le clair des messages ouverts sur votre appareil (les 2 000 derniers) : une page rechargée ou une conversation relue s'affichent quand même.",
        },
        Faq {
            q: "Où sont rangées les sessions ?",
            a: "Dans le dossier <code>v3/</code> à côté de <code>brocolis.toml</code>, lisible par vous seul. Le proxy, l'application et la ligne de commande s'y succèdent une transaction à la fois : aucun message n'est jamais chiffré deux fois avec la même clé.",
        },
        Faq {
            q: "Et si le dossier v3 est perdu ?",
            a: "Vos contacts restent. La session se rouvre au prochain message que l'un de vous deux envoie ; les jetons v3 déjà lus mais oubliés restent fermés, c'est le prix de la confidentialité persistante.",
        },
        Faq {
            q: "Peut-on vérifier ?",
            a: "Oui : la spécification est publique, l'implémentation de référence en Rust est livrée avec brocolis (<code>proto/</code>), avec ses vecteurs de test et les vecteurs Wycheproof des primitives.",
        },
    ],

    toc_v4: "Le v4, au-delà de Signal",
    v4_h: "Protocole v4 : au-delà de Signal",
    v4_intro: "Le v3 avait deux faiblesses. Le post-quantique s'arrêtait à l'établissement : un attaquant qui garde tes jetons et casse X25519 plus tard lisait la suite. Et tout ce qui entoure le chiffré restait en clair : identifiant de session, compteurs, identité de l'expéditeur du premier message. Le v4 corrige les deux, et va plus loin que Signal sur chacun.",
    v4_cols: ["", "Signal (2025)", "Brocolis v3", "Brocolis v4"],
    v4_rows: [
        [
            "ML-KEM dans le cliquet",
            "Par épisodes, en morceaux sur plusieurs messages (SPQR)",
            "Non : seulement à l'établissement",
            "À chaque tour, dès le premier message reçu",
        ],
        ["En-têtes de cliquet chiffrés", "Non", "Non", "Oui"],
        [
            "Identifiant de session visible",
            "Sans objet (son serveur)",
            "Oui, 8 octets",
            "Non",
        ],
        [
            "Identité de l'expéditeur du premier message",
            "Protégée par le serveur (sealed sender)",
            "En clair",
            "Chiffrée, X25519 + ML-KEM",
        ],
        [
            "Type de message visible",
            "Sans objet",
            "Oui",
            "Non : dans le chiffré",
        ],
        [
            "Retour forcé à l'ancien protocole",
            "Sans objet",
            "Possible par le serveur",
            "Détecté : l'empreinte v4 diffère",
        ],
    ],
    v4_cards: [
        Faq {
            q: "Triple cliquet",
            a: "Chaque nouvelle chaîne d'envoi mélange un échange X25519 et une encapsulation ML-KEM-768 neuve. Casser X25519 un jour ne suffit plus, même pour les messages d'après.",
        },
        Faq {
            q: "En-têtes chiffrés",
            a: "La clé de cliquet, les compteurs et le type de message voyagent chiffrés sous des clés d'en-tête qui tournent elles aussi. Rien ne relie deux messages d'une même conversation.",
        },
        Faq {
            q: "Expéditeur caché",
            a: "Le premier message ne dit plus qui l'envoie : l'identité est scellée sous X25519 et ML-KEM, et seul le destinataire l'ouvre.",
        },
        Faq {
            q: "Ni déclassement, ni substitution",
            a: "La capacité v4 est signée ; si le serveur la retire d'un côté, les empreintes comparées de vive voix diffèrent. Un contact qui revient avec une autre identité déclenche une alerte, et rien ne part avant ta confirmation.",
        },
    ],
    v4_cost: "Le prix : un message v4 pèse environ 2,5 Ko (3,4 Ko une fois encodé) contre 225 octets en v3, car chaque message porte une clé ML-KEM. Le v4 part dès que les deux appareils savent faire ML-KEM ; sinon c'est le v3, lisible pour toujours.",
    v4_honest: "Honnêteté : le v4 n'a pas encore été audité par un tiers. Ses vecteurs de test sont vérifiés par une implémentation indépendante et son déchiffrement est soumis au fuzzing, mais « mieux que Signal » se prouve par un audit, pas par une page web.",
    labels_v4: [
        Row {
            label: "🔐 triple cliquet (post-quantique à chaque message)",
            old: "",
            new: "Appairé à deux, les deux appareils publient leur capacité v4 : protocole v4",
        },
        Row {
            label: "🔐 triple cliquet, clé partagée (PSK)",
            old: "",
            new: "Le v4 à partir de la clé partagée, quand un message v4 est déjà arrivé sur cette clé",
        },
    ],
    spec_h: "La spécification",
    spec_body: "Tout ce qui est décrit ici est écrit, avec ses formules et ses cas limites, dans <code>docs/SECURITE.md</code> de la messagerie Brocolis. Brocolis, l'iPhone, Android et le site en suivent le même texte.",
    spec_cta: "Lire la spécification",
};

// ---------------------------------------------------------------------------
// English
// ---------------------------------------------------------------------------

pub static EN: Proto = Proto {
    meta_title: "The v3 protocol: the double ratchet — Brocolis 🥦",
    nav: "v3 protocol",
    cta: "Everything about the v3 protocol",
    back: "Home",
    kicker: "v3 protocol",
    title: "A fresh key for every message",
    lead: "With a paired contact, Brocolis no longer encrypts your <code>((secrets))</code> with a fixed key. It opens a <strong>double-ratchet session</strong>, like Signal: every message has its own key, erased right away, and the session is set up as a <strong>hybrid X25519 + ML-KEM-768</strong>, ready for tomorrow's quantum computer. It's the protocol of the Brocolis messenger — iPhone, Android, the site's chat — and brocolis now speaks it on macOS, Linux and Windows.",
    toc: [
        "What changes",
        "Try the ratchet",
        "Step by step",
        "Anatomy of a token",
        "Against whom",
        "Where, and what you see",
        "Questions",
    ],

    cmp_h: "What changes",
    cmp_intro: "The old format (<code>v1</code>/<code>v2</code>) stays readable forever, and still serves groups. But one-to-one, the difference is clear:",
    cmp_cols: ["", "Old format (v1)", "Double ratchet (v3)"],
    cmp_rows: [
        Row {
            label: "The key",
            old: "Just one, the same for every message",
            new: "A fresh one per message, erased as soon as it's used",
        },
        Row {
            label: "A key stolen later",
            old: "Opens the whole history the server kept",
            new: "Opens nothing already sent (forward secrecy)",
        },
        Row {
            label: "Passing spyware",
            old: "Reads everything until the key is changed",
            new: "Loses its grip at the first round trip (the session heals)",
        },
        Row {
            label: "The quantum computer",
            old: "Out of scope (AES-256 alone), but X25519 pairing can be broken",
            new: "Hybrid X25519 + ML-KEM-768 setup: both must be broken",
        },
        Row {
            label: "Message length",
            old: "Shows in the token size",
            new: "Hidden: padded to a multiple of 160 bytes",
        },
        Row {
            label: "A replayed message",
            old: "Decrypts again",
            new: "Recognised and ignored",
        },
        Row {
            label: "The pairing fingerprint",
            old: "Covers the shared key",
            new: "Covers the key and both identities: a swapped bundle shows",
        },
    ],

    sim_h: "Try the ratchet",
    sim_intro: "Write to Alice, let her reply, then bring in spyware that copies your computer's state at one moment. Watch what it can read: nothing that left before, and nothing more once the ratchet has turned on your side.",
    sim_you: "You",
    sim_alice: "Alice",
    sim_send: "You write",
    sim_reply: "Alice replies",
    sim_spy: "Spyware copies the state",
    sim_reset: "Start over",
    sim_empty: "No messages yet: write to Alice.",
    sim_key: "key",
    sim_erased: "erased",
    sim_turn: "The ratchet turns: a fresh Diffie-Hellman exchange",
    sim_spied: "Here, spyware copies the state",
    sim_exposed: "readable by the spy",
    sim_safe_past: "out of reach: its key was already erased",
    sim_safe_healed: "out of reach: the session healed",
    sim_score: "The spy reads {n} of {total} message(s).",
    sim_note: "Teaching simulation: the \"keys\" shown are markers, not the real ones. The real protocol does the same with HKDF-SHA256, HMAC-SHA256 and X25519 (and ML-KEM-768 at setup).",
    sim_you_texts: [
        "Hi Alice!",
        "See you tomorrow?",
        "Door code: 4291",
        "Don't forget the file.",
        "I'm on the 5pm train.",
        "Did you get my photo?",
        "I'll call you tonight.",
        "Good night 🥦",
    ],
    sim_alice_texts: [
        "Hey 🥦",
        "Yes, 6pm at the café?",
        "Got it, thanks!",
        "Promise!",
        "I'll pick you up.",
        "Not yet, try again.",
        "Sounds good.",
        "See you tomorrow!",
    ],

    steps_h: "Step by step",
    steps: [
        Step {
            title: "1. Each device has its key bundle",
            body: "A long-term identity, a signed prekey renewed every week and an ML-KEM-768 key. All of it is signed by the identity and published when pairing, in the meeting room, along with the usual public key.",
            detail: "<code>IK X25519 ‖ IK Ed25519 ‖ SPK ‖ PQPK ‖ Ed25519 signatures</code> — 166 bytes, or 1,418 with ML-KEM.",
        },
        Step {
            title: "2. Pairing checks both identities",
            body: "Both sides derive the same shared key, as before. But the fingerprint you compare in person now covers the key and both identities: a server slipping in its own bundle would make the two fingerprints differ.",
            detail: "<code>SHA-256(\"Brocolis_v3_Appairage\" ‖ K ‖ min(P_A, P_B) ‖ max(P_A, P_B))[0..8]</code>",
        },
        Step {
            title: "3. The first message sets up the session",
            body: "Like Signal PQXDH: three Diffie-Hellman exchanges and one ML-KEM-768 encapsulation are mixed into a starting key. That first message carries what the other side needs to rebuild it — Alice doesn't have to be online.",
            detail: "<code>SK = HKDF(32 × 0x00, 32 × 0xFF ‖ DH1 ‖ DH2 ‖ DH3 ‖ SS_ML-KEM, \"Brocolis_v3_X25519_MLKEM768_SHA-256\")</code>",
        },
        Step {
            title: "4. The ratchet moves with every message",
            body: "Each message derives its key from the previous one, then erases it: there's no going back. With every reply, a fresh Diffie-Hellman exchange turns the big ratchet, and a stolen state becomes useless.",
            detail: "<code>mk = HMAC(ck, 0x01)</code>, <code>ck' = HMAC(ck, 0x02)</code>, <code>(RK, CK) = HKDF(RK, DH(ratchet, remote ratchet))</code>",
        },
        Step {
            title: "5. Receiving tolerates disorder, not replays",
            body: "A message may arrive late or twice — copy-paste, a reloaded page. Skipped keys are kept (1,000 at most, 30 days), a replayed message no longer finds its key and is ignored. The plaintext of a message you've read stays on your device: the page reloads without losing anything.",
            detail: "Transactional decryption: a failure leaves the state exactly as it was.",
        },
    ],

    token_h: "Anatomy of a token",
    token_intro: "A <code>[[BROCOLIS:v3:…]]</code> token is standard base64; here's what it holds, byte by byte. The site's server only sees this sequence: nothing says who writes, nor what.",
    token_first: "first message only",
    token_fields: [
        Field {
            name: "type",
            size: "1",
            desc: "0x01 first message, 0x02 message",
            first_only: false,
        },
        Field {
            name: "session",
            size: "8",
            desc: "SHA-256(IK_A ‖ IK_B), truncated",
            first_only: false,
        },
        Field {
            name: "IK_A",
            size: "32",
            desc: "the sender's X25519 identity",
            first_only: true,
        },
        Field {
            name: "EK_A",
            size: "32",
            desc: "ephemeral key",
            first_only: true,
        },
        Field {
            name: "mode",
            size: "1",
            desc: "X25519 only, ML-KEM hybrid, or shared key",
            first_only: true,
        },
        Field {
            name: "CT",
            size: "1,088",
            desc: "ML-KEM-768 encapsulation",
            first_only: true,
        },
        Field {
            name: "SPK · PQPK",
            size: "4 + 4",
            desc: "which signed prekeys were used",
            first_only: true,
        },
        Field {
            name: "header",
            size: "40",
            desc: "ratchet key (32), chain no., message no.",
            first_only: false,
        },
        Field {
            name: "ciphertext",
            size: "160·k + 16",
            desc: "AES-256-GCM of the padded text, and its tag",
            first_only: false,
        },
    ],
    token_note: "Any other length is refused, so is any non-canonical public key, and the header is authenticated with the ciphertext: changing a single byte makes decryption fail.",

    threat_h: "Against whom",
    threat_cols: ["Who", "What they can do", "What you keep"],
    threats: [
        Threat {
            who: "The online service (Twitter, a forum…)",
            can: "Read, keep, copy everything you post",
            we: "It only stores tokens; a modified token is refused, a replayed one ignored",
        },
        Threat {
            who: "The pairing server",
            can: "Swap public keys or bundles",
            we: "The fingerprint compared in person gives it away",
        },
        Threat {
            who: "Whoever keeps your tokens for years",
            can: "Steal a key later",
            we: "Past messages stay closed: their keys no longer exist",
        },
        Threat {
            who: "Tomorrow's quantum computer",
            can: "Break X25519 retroactively",
            we: "It would also have to break ML-KEM-768",
        },
        Threat {
            who: "Passing spyware",
            can: "Copy the session state at one moment",
            we: "The session heals with the next reply",
        },
        Threat {
            who: "Whoever looks at token sizes",
            can: "Guess a message's length",
            we: "Everything is padded to a multiple of 160 bytes",
        },
    ],

    where_h: "Where, and what you see",
    where_intro: "One protocol, one reference implementation in Rust, and test vectors every platform matches byte for byte: a message encrypted on a computer opens on an iPhone, and the other way round.",
    platforms: [
        Platform {
            name: "brocolis — macOS, Linux, Windows",
            status: "Proxy, app and command line: pairing, sending and receiving in v3, ML-KEM-768 hybrid",
        },
        Platform {
            name: "iPhone",
            status: "App, keyboard, Share, iMessage; hybrid from iOS 26, X25519 only before",
        },
        Platform {
            name: "Android",
            status: "App, Share, messenger; ML-KEM-768 hybrid",
        },
        Platform {
            name: "The messenger site's chat",
            status: "In the browser, sessions kept in the vault; ML-KEM-768 hybrid",
        },
    ],
    labels_h: "Each contact tells you how the next message leaves",
    labels_intro: "In the app's Contacts tab, <code>brocolis contacts</code> and every messenger app:",
    labels: [
        Row {
            label: "🔐 double ratchet (post-quantum)",
            old: "",
            new: "Paired contact, hybrid X25519 + ML-KEM-768 session",
        },
        Row {
            label: "🔐 double ratchet (classic)",
            old: "",
            new: "Paired contact whose device can't do ML-KEM yet",
        },
        Row {
            label: "🔐 double ratchet, shared key (PSK)",
            old: "",
            new: "No pairing, but you both run an up-to-date version: the ratchet starts from the shared key",
        },
        Row {
            label: "legacy format",
            old: "",
            new: "Group, several recipients or a contact not yet up to date: v1/v2",
        },
    ],

    limits_h: "What v3 doesn't do",
    limits: [
        "<strong>A group key</strong> (known to three people or more) stays on v1/v2. <strong>Ratchet groups</strong> (Sender Keys) work in brocolis: <code>brocolis group-create</code>, then <code>encrypt --group</code>; in the apps, the screen to create one is coming.",
        "<strong>The decrypted page</strong> stays readable by the visited site's JavaScript: that's how a proxy decrypting in the page works.",
        "<strong>The service's metadata</strong> (who posts what, when) doesn't change.",
        "<strong>An unlocked device</strong> in the wrong hands reads what you read.",
        "<strong>A contact silent for 30 days</strong> with no open session goes back to the legacy format, until the next pairing.",
    ],

    faq_h: "Questions",
    faqs: [
        Faq {
            q: "Do I have to do anything?",
            a: "Pair with your contacts (Contacts tab, or <code>brocolis pair --name</code>). If they're up to date, the contact switches to the double ratchet on its own, and the fingerprint you compare covers both your devices.",
        },
        Faq {
            q: "My old messages?",
            a: "They stay readable: v1 and v2 tokens open forever, with the same keys.",
        },
        Faq {
            q: "What if my contact isn't up to date?",
            a: "Nothing breaks: without a key bundle on their side, pairing and messages stay in the legacy format, and the app says so.",
        },
        Faq {
            q: "Why does a message only open once?",
            a: "Because its key is erased as soon as it's read. Brocolis keeps the plaintext of opened messages on your device (the last 2,000): a reloaded page or a re-read conversation still shows.",
        },
        Faq {
            q: "Where are sessions stored?",
            a: "In the <code>v3/</code> folder next to <code>brocolis.toml</code>, readable by you alone. The proxy, the app and the command line take turns, one transaction at a time: no message is ever encrypted twice with the same key.",
        },
        Faq {
            q: "What if the v3 folder is lost?",
            a: "Your contacts stay. The session reopens with the next message either of you sends; v3 tokens already read but forgotten stay closed — that's the price of forward secrecy.",
        },
        Faq {
            q: "Can it be checked?",
            a: "Yes: the specification is public, and the Rust reference implementation ships with brocolis (<code>proto/</code>), with its test vectors and the primitives' Wycheproof vectors.",
        },
    ],

    toc_v4: "v4, beyond Signal",
    v4_h: "v4 protocol: beyond Signal",
    v4_intro: "v3 had two weaknesses. Post-quantum protection stopped at setup: an attacker who keeps your tokens and breaks X25519 later could read what followed. And everything around the ciphertext stayed in clear: session id, counters, the first message's sender identity. v4 fixes both, and goes further than Signal on each.",
    v4_cols: ["", "Signal (2025)", "Brocolis v3", "Brocolis v4"],
    v4_rows: [
        [
            "ML-KEM in the ratchet",
            "In episodes, chunked over several messages (SPQR)",
            "No: only at setup",
            "Every turn, from the first message received",
        ],
        ["Encrypted ratchet headers", "No", "No", "Yes"],
        [
            "Visible session id",
            "n/a (its server)",
            "Yes, 8 bytes",
            "No",
        ],
        [
            "Sender identity in the first message",
            "Protected by the server (sealed sender)",
            "In clear",
            "Encrypted, X25519 + ML-KEM",
        ],
        [
            "Visible message type",
            "n/a",
            "Yes",
            "No: inside the ciphertext",
        ],
        [
            "Forced fallback to the old protocol",
            "n/a",
            "Possible for the server",
            "Detected: the v4 fingerprint differs",
        ],
    ],
    v4_cards: [
        Faq {
            q: "Triple ratchet",
            a: "Every new sending chain mixes an X25519 exchange and a fresh ML-KEM-768 encapsulation. Breaking X25519 someday is no longer enough, even for later messages.",
        },
        Faq {
            q: "Encrypted headers",
            a: "The ratchet key, counters and message type travel encrypted under header keys that rotate too. Nothing links two messages of the same conversation.",
        },
        Faq {
            q: "Hidden sender",
            a: "The first message no longer says who sent it: the identity is sealed under X25519 and ML-KEM, and only the recipient opens it.",
        },
        Faq {
            q: "No downgrade, no swap",
            a: "The v4 capability is signed; if the server strips it on one side, the fingerprints you compare out loud differ. A contact coming back with another identity raises an alert, and nothing is sent until you confirm.",
        },
    ],
    v4_cost: "The price: a v4 message weighs about 2.5 KB (3.4 KB once encoded) versus 225 bytes in v3, since every message carries an ML-KEM key. v4 kicks in as soon as both devices can do ML-KEM; otherwise it's v3, readable forever.",
    v4_honest: "Honesty: v4 hasn't been audited by a third party yet. Its test vectors are checked by an independent implementation and its decryption is fuzzed, but “better than Signal” is proven by an audit, not a web page.",
    labels_v4: [
        Row {
            label: "🔐 triple ratchet (post-quantum on every message)",
            old: "",
            new: "Paired one-to-one, both devices publish their v4 capability: v4 protocol",
        },
        Row {
            label: "🔐 triple ratchet, shared key (PSK)",
            old: "",
            new: "v4 from the shared key, once a v4 message has arrived on that key",
        },
    ],
    spec_h: "The specification",
    spec_body: "Everything described here is written down, with its formulas and edge cases, in the Brocolis messenger's <code>docs/SECURITE.md</code>. Brocolis, the iPhone, Android and the site follow the same text.",
    spec_cta: "Read the specification",
};

// ---------------------------------------------------------------------------
// Español
// ---------------------------------------------------------------------------

pub static ES: Proto = Proto {
    meta_title: "Protocolo v3: el doble trinquete — Brocolis 🥦",
    nav: "Protocolo v3",
    cta: "Todo sobre el protocolo v3",
    back: "Inicio",
    kicker: "Protocolo v3",
    title: "Una clave nueva para cada mensaje",
    lead: "Con un contacto emparejado, Brocolis ya no cifra tus <code>((secretos))</code> con una clave fija. Abre una <strong>sesión de doble trinquete</strong>, como Signal: cada mensaje tiene su propia clave, borrada enseguida, y la sesión se establece en <strong>híbrido X25519 + ML-KEM-768</strong>, preparada para el ordenador cuántico de mañana. Es el protocolo de la mensajería Brocolis — iPhone, Android, el chat del sitio — y brocolis ya lo habla en macOS, Linux y Windows.",
    toc: [
        "Qué cambia",
        "Prueba el trinquete",
        "Paso a paso",
        "Anatomía de un token",
        "Contra quién",
        "Dónde, y qué se muestra",
        "Preguntas",
    ],

    cmp_h: "Qué cambia",
    cmp_intro: "El formato antiguo (<code>v1</code>/<code>v2</code>) sigue legible para siempre, y aún sirve para los grupos. Pero entre dos, la diferencia es clara:",
    cmp_cols: ["", "Formato antiguo (v1)", "Doble trinquete (v3)"],
    cmp_rows: [
        Row {
            label: "La clave",
            old: "Una sola, la misma para todos los mensajes",
            new: "Una nueva por mensaje, borrada en cuanto se usa",
        },
        Row {
            label: "Una clave robada más tarde",
            old: "Abre todo el historial guardado por el servidor",
            new: "No abre nada de lo ya enviado (confidencialidad persistente)",
        },
        Row {
            label: "Un espía de paso",
            old: "Lo lee todo mientras no se cambie la clave",
            new: "Pierde el control en la primera ida y vuelta (la sesión se cura)",
        },
        Row {
            label: "El ordenador cuántico",
            old: "Fuera de alcance (solo AES-256), pero el emparejamiento X25519 se puede romper",
            new: "Establecimiento híbrido X25519 + ML-KEM-768: hay que romper los dos",
        },
        Row {
            label: "La longitud del mensaje",
            old: "Se lee en el tamaño del token",
            new: "Oculta: completada hasta un múltiplo de 160 bytes",
        },
        Row {
            label: "Un mensaje repetido",
            old: "Se descifra otra vez",
            new: "Reconocido e ignorado",
        },
        Row {
            label: "La huella de emparejamiento",
            old: "Cubre la clave compartida",
            new: "Cubre la clave y las dos identidades: un paquete sustituido se nota",
        },
    ],

    sim_h: "Prueba el trinquete",
    sim_intro: "Escribe a Alice, deja que responda y luego haz entrar a un espía que copia el estado de tu ordenador en un momento dado. Mira lo que puede leer: nada de lo que salió antes, y nada más en cuanto el trinquete ha girado de tu lado.",
    sim_you: "Tú",
    sim_alice: "Alice",
    sim_send: "Escribes",
    sim_reply: "Alice responde",
    sim_spy: "Un espía copia el estado",
    sim_reset: "Volver a empezar",
    sim_empty: "Aún no hay mensajes: escribe a Alice.",
    sim_key: "clave",
    sim_erased: "borrada",
    sim_turn: "El trinquete gira: nuevo intercambio Diffie-Hellman",
    sim_spied: "Aquí, un espía copia el estado",
    sim_exposed: "legible por el espía",
    sim_safe_past: "fuera de alcance: su clave ya estaba borrada",
    sim_safe_healed: "fuera de alcance: la sesión se curó",
    sim_score: "El espía lee {n} de {total} mensaje(s).",
    sim_note: "Simulación pedagógica: las «claves» mostradas son marcas, no las reales. El protocolo real hace lo mismo con HKDF-SHA256, HMAC-SHA256 y X25519 (y ML-KEM-768 al establecer la sesión).",
    sim_you_texts: [
        "¡Hola Alice!",
        "¿Nos vemos mañana?",
        "Código de la puerta: 4291",
        "No olvides el informe.",
        "Cojo el tren de las 17h.",
        "¿Te llegó mi foto?",
        "Te llamo esta noche.",
        "Buenas noches 🥦",
    ],
    sim_alice_texts: [
        "Hola 🥦",
        "Sí, ¿a las 18h en el café?",
        "Anotado, ¡gracias!",
        "¡Prometido!",
        "Paso a buscarte.",
        "Todavía no, reinténtalo.",
        "Perfecto.",
        "¡Hasta mañana!",
    ],

    steps_h: "Paso a paso",
    steps: [
        Step {
            title: "1. Cada equipo tiene su paquete de claves",
            body: "Una identidad duradera, una clave prefirmada renovada cada semana y una clave ML-KEM-768. Todo firmado por la identidad y publicado al emparejar, en la sala de encuentro, junto a la clave pública de siempre.",
            detail: "<code>IK X25519 ‖ IK Ed25519 ‖ SPK ‖ PQPK ‖ firmas Ed25519</code> — 166 bytes, o 1 418 con ML-KEM.",
        },
        Step {
            title: "2. El emparejamiento verifica las dos identidades",
            body: "Los dos lados derivan la misma clave compartida, como antes. Pero la huella que comparáis en persona cubre ahora la clave y las dos identidades: un servidor que colara su propio paquete haría mostrar dos huellas distintas.",
            detail: "<code>SHA-256(\"Brocolis_v3_Appairage\" ‖ K ‖ min(P_A, P_B) ‖ max(P_A, P_B))[0..8]</code>",
        },
        Step {
            title: "3. El primer mensaje establece la sesión",
            body: "Como Signal PQXDH: tres intercambios Diffie-Hellman y una encapsulación ML-KEM-768 se mezclan en una clave inicial. Ese primer mensaje lleva lo necesario para reconstruirla al otro lado — Alice no tiene que estar conectada.",
            detail: "<code>SK = HKDF(32 × 0x00, 32 × 0xFF ‖ DH1 ‖ DH2 ‖ DH3 ‖ SS_ML-KEM, \"Brocolis_v3_X25519_MLKEM768_SHA-256\")</code>",
        },
        Step {
            title: "4. El trinquete avanza con cada mensaje",
            body: "Cada mensaje deriva su clave de la anterior y luego la borra: no hay vuelta atrás. Con cada respuesta, un nuevo intercambio Diffie-Hellman gira el gran trinquete, y un estado robado deja de servir.",
            detail: "<code>mk = HMAC(ck, 0x01)</code>, <code>ck' = HMAC(ck, 0x02)</code>, <code>(RK, CK) = HKDF(RK, DH(trinquete, trinquete remoto))</code>",
        },
        Step {
            title: "5. La recepción tolera el desorden, no la repetición",
            body: "Un mensaje puede llegar tarde o dos veces — copiar y pegar, una página recargada. Las claves saltadas se guardan (1 000 como máximo, 30 días), un mensaje repetido ya no encuentra su clave y se ignora. El texto de un mensaje leído se queda en tu equipo: la página se recarga sin perder nada.",
            detail: "Descifrado transaccional: un fallo deja el estado exactamente como estaba.",
        },
    ],

    token_h: "Anatomía de un token",
    token_intro: "Un token <code>[[BROCOLIS:v3:…]]</code> es base64 estándar; esto es lo que contiene, byte a byte. El servidor del sitio solo ve esta secuencia: nada dice quién escribe ni qué.",
    token_first: "solo el primer mensaje",
    token_fields: [
        Field {
            name: "tipo",
            size: "1",
            desc: "0x01 primer mensaje, 0x02 mensaje",
            first_only: false,
        },
        Field {
            name: "sesión",
            size: "8",
            desc: "SHA-256(IK_A ‖ IK_B), truncado",
            first_only: false,
        },
        Field {
            name: "IK_A",
            size: "32",
            desc: "identidad X25519 del remitente",
            first_only: true,
        },
        Field {
            name: "EK_A",
            size: "32",
            desc: "clave efímera",
            first_only: true,
        },
        Field {
            name: "modo",
            size: "1",
            desc: "solo X25519, híbrido ML-KEM o clave compartida",
            first_only: true,
        },
        Field {
            name: "CT",
            size: "1 088",
            desc: "encapsulación ML-KEM-768",
            first_only: true,
        },
        Field {
            name: "SPK · PQPK",
            size: "4 + 4",
            desc: "qué claves prefirmadas se usaron",
            first_only: true,
        },
        Field {
            name: "cabecera",
            size: "40",
            desc: "clave de trinquete (32), n.º de cadena, n.º de mensaje",
            first_only: false,
        },
        Field {
            name: "cifrado",
            size: "160·k + 16",
            desc: "AES-256-GCM del texto completado, y su etiqueta",
            first_only: false,
        },
    ],
    token_note: "Cualquier otra longitud se rechaza, igual que cualquier clave pública no canónica, y la cabecera se autentica con el cifrado: cambiar un solo byte hace fallar el descifrado.",

    threat_h: "Contra quién",
    threat_cols: ["Quién", "Qué puede hacer", "Qué conservas"],
    threats: [
        Threat {
            who: "El servicio en línea (Twitter, un foro…)",
            can: "Leer, guardar, copiar todo lo que publicas",
            we: "Solo almacena tokens; un token modificado se rechaza, uno repetido se ignora",
        },
        Threat {
            who: "El servidor de emparejamiento",
            can: "Sustituir claves públicas o paquetes",
            we: "La huella comparada en persona lo delata",
        },
        Threat {
            who: "Quien guarda tus tokens durante años",
            can: "Robar una clave más tarde",
            we: "Los mensajes pasados siguen cerrados: sus claves ya no existen",
        },
        Threat {
            who: "El ordenador cuántico de mañana",
            can: "Romper X25519 retroactivamente",
            we: "También tendría que romper ML-KEM-768",
        },
        Threat {
            who: "Un programa espía de paso",
            can: "Copiar el estado de las sesiones en un momento",
            we: "La sesión se cura con la siguiente respuesta",
        },
        Threat {
            who: "Quien mira el tamaño de los tokens",
            can: "Adivinar la longitud de un mensaje",
            we: "Todo se completa hasta un múltiplo de 160 bytes",
        },
    ],

    where_h: "Dónde, y qué se muestra",
    where_intro: "Un solo protocolo, una implementación de referencia en Rust y vectores de prueba que cada plataforma reproduce byte a byte: un mensaje cifrado en un ordenador se abre en un iPhone, y al revés.",
    platforms: [
        Platform {
            name: "brocolis — macOS, Linux, Windows",
            status: "Proxy, aplicación y línea de comandos: emparejamiento, envío y recepción en v3, híbrido ML-KEM-768",
        },
        Platform {
            name: "iPhone",
            status: "App, teclado, Compartir, iMessage; híbrido desde iOS 26, solo X25519 antes",
        },
        Platform {
            name: "Android",
            status: "App, Compartir, mensajería; híbrido ML-KEM-768",
        },
        Platform {
            name: "El chat del sitio de la mensajería",
            status: "En el navegador, sesiones guardadas en la caja fuerte; híbrido ML-KEM-768",
        },
    ],
    labels_h: "Cada contacto dice cómo sale el próximo mensaje",
    labels_intro: "En la pestaña Contactos de la aplicación, <code>brocolis contacts</code> y todas las apps de la mensajería:",
    labels: [
        Row {
            label: "🔐 doble trinquete (poscuántico)",
            old: "",
            new: "Contacto emparejado, sesión híbrida X25519 + ML-KEM-768",
        },
        Row {
            label: "🔐 doble trinquete (clásico)",
            old: "",
            new: "Contacto emparejado cuyo equipo aún no sabe hacer ML-KEM",
        },
        Row {
            label: "🔐 doble trinquete, clave compartida (PSK)",
            old: "",
            new: "Sin emparejar, pero los dos tenéis una versión al día: el trinquete parte de la clave compartida",
        },
        Row {
            label: "formato antiguo",
            old: "",
            new: "Grupo, varios destinatarios o un contacto aún sin actualizar: v1/v2",
        },
    ],

    limits_h: "Lo que el v3 no hace",
    limits: [
        "<strong>Una clave de grupo</strong> (conocida por tres personas o más) sigue en v1/v2. Los <strong>grupos con trinquete</strong> (Sender Keys) funcionan en brocolis: <code>brocolis group-create</code> y luego <code>encrypt --group</code>; en las aplicaciones, la pantalla para crearlos llegará.",
        "<strong>La página descifrada</strong> sigue siendo legible por el JavaScript del sitio visitado: así funciona un proxy que descifra en la página.",
        "<strong>Los metadatos</strong> del servicio (quién publica qué, cuándo) no cambian.",
        "<strong>Un equipo desbloqueado</strong> en malas manos lee lo que tú lees.",
        "<strong>Un contacto sin noticias desde hace 30 días</strong> y sin sesión abierta vuelve al formato antiguo, hasta el próximo emparejamiento.",
    ],

    faq_h: "Preguntas",
    faqs: [
        Faq {
            q: "¿Tengo que hacer algo?",
            a: "Empareja con tus contactos (pestaña Contactos, o <code>brocolis pair --name</code>). Si están al día, el contacto pasa solo al doble trinquete, y la huella que comparáis cubre vuestros dos equipos.",
        },
        Faq {
            q: "¿Y mis mensajes antiguos?",
            a: "Siguen legibles: los tokens v1 y v2 se abren para siempre, con las mismas claves.",
        },
        Faq {
            q: "¿Y si mi contacto no está al día?",
            a: "No se rompe nada: sin paquete de claves de su lado, el emparejamiento y los mensajes siguen en formato antiguo, y la aplicación lo indica.",
        },
        Faq {
            q: "¿Por qué un mensaje solo se abre una vez?",
            a: "Porque su clave se borra en cuanto se lee. Brocolis guarda el texto de los mensajes abiertos en tu equipo (los últimos 2 000): una página recargada o una conversación releída se siguen mostrando.",
        },
        Faq {
            q: "¿Dónde se guardan las sesiones?",
            a: "En la carpeta <code>v3/</code> junto a <code>brocolis.toml</code>, legible solo por ti. El proxy, la aplicación y la línea de comandos se turnan, una transacción a la vez: ningún mensaje se cifra nunca dos veces con la misma clave.",
        },
        Faq {
            q: "¿Y si se pierde la carpeta v3?",
            a: "Tus contactos se quedan. La sesión se reabre con el próximo mensaje que envíe cualquiera de los dos; los tokens v3 ya leídos pero olvidados siguen cerrados: es el precio de la confidencialidad persistente.",
        },
        Faq {
            q: "¿Se puede verificar?",
            a: "Sí: la especificación es pública, y la implementación de referencia en Rust viene con brocolis (<code>proto/</code>), con sus vectores de prueba y los vectores Wycheproof de las primitivas.",
        },
    ],

    toc_v4: "El v4, más allá de Signal",
    v4_h: "Protocolo v4: más allá de Signal",
    v4_intro: "El v3 tenía dos debilidades. Lo poscuántico se detenía en el establecimiento: un atacante que guarda tus tokens y rompe X25519 más adelante leía lo que seguía. Y todo lo que rodea al cifrado quedaba en claro: identificador de sesión, contadores, identidad del remitente del primer mensaje. El v4 corrige ambas, y va más lejos que Signal en cada una.",
    v4_cols: ["", "Signal (2025)", "Brocolis v3", "Brocolis v4"],
    v4_rows: [
        [
            "ML-KEM en el trinquete",
            "Por episodios, en trozos sobre varios mensajes (SPQR)",
            "No: solo al establecer",
            "En cada vuelta, desde el primer mensaje recibido",
        ],
        ["Cabeceras del trinquete cifradas", "No", "No", "Sí"],
        [
            "Identificador de sesión visible",
            "No aplica (su servidor)",
            "Sí, 8 bytes",
            "No",
        ],
        [
            "Identidad del remitente del primer mensaje",
            "Protegida por el servidor (sealed sender)",
            "En claro",
            "Cifrada, X25519 + ML-KEM",
        ],
        [
            "Tipo de mensaje visible",
            "No aplica",
            "Sí",
            "No: dentro del cifrado",
        ],
        [
            "Vuelta forzada al protocolo antiguo",
            "No aplica",
            "Posible para el servidor",
            "Detectada: la huella v4 difiere",
        ],
    ],
    v4_cards: [
        Faq {
            q: "Triple trinquete",
            a: "Cada nueva cadena de envío mezcla un intercambio X25519 y una encapsulación ML-KEM-768 nueva. Romper X25519 algún día ya no basta, ni para los mensajes siguientes.",
        },
        Faq {
            q: "Cabeceras cifradas",
            a: "La clave del trinquete, los contadores y el tipo de mensaje viajan cifrados con claves de cabecera que también giran. Nada une dos mensajes de una misma conversación.",
        },
        Faq {
            q: "Remitente oculto",
            a: "El primer mensaje ya no dice quién lo envía: la identidad va sellada con X25519 y ML-KEM, y solo el destinatario la abre.",
        },
        Faq {
            q: "Ni degradación, ni sustitución",
            a: "La capacidad v4 va firmada; si el servidor la quita de un lado, las huellas que comparáis en voz alta difieren. Un contacto que vuelve con otra identidad dispara una alerta, y no sale nada hasta que lo confirmes.",
        },
    ],
    v4_cost: "El precio: un mensaje v4 pesa unos 2,5 KB (3,4 KB codificado) frente a 225 bytes en v3, porque cada mensaje lleva una clave ML-KEM. El v4 se usa en cuanto ambos dispositivos saben hacer ML-KEM; si no, es el v3, legible para siempre.",
    v4_honest: "Honestidad: el v4 aún no lo ha auditado un tercero. Sus vectores de prueba los verifica una implementación independiente y su descifrado pasa por fuzzing, pero «mejor que Signal» se demuestra con una auditoría, no con una página web.",
    labels_v4: [
        Row {
            label: "🔐 triple trinquete (poscuántico en cada mensaje)",
            old: "",
            new: "Emparejados de dos en dos, ambos dispositivos publican su capacidad v4: protocolo v4",
        },
        Row {
            label: "🔐 triple trinquete, clave compartida (PSK)",
            old: "",
            new: "El v4 a partir de la clave compartida, cuando ya ha llegado un mensaje v4 con esa clave",
        },
    ],
    spec_h: "La especificación",
    spec_body: "Todo lo descrito aquí está escrito, con sus fórmulas y casos límite, en <code>docs/SECURITE.md</code> de la mensajería Brocolis. Brocolis, el iPhone, Android y el sitio siguen el mismo texto.",
    spec_cta: "Leer la especificación",
};
