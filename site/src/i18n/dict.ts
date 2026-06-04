export const LANGS = ["fr", "en", "es"] as const;
export type Lang = (typeof LANGS)[number];

export const DICT = {
  fr: {
    "meta.title": "Brocolis 🥦 — Proxy MITM Rust de chiffrement sélectif",
    "meta.desc":
      "Proxy MITM en Rust qui chiffre sélectivement le trafic HTTP/HTTPS sur Twitter, les forums, les messageries — sans rien changer côté serveur.",
    "meta.ogTitle": "Brocolis 🥦 — Chiffrement sélectif de votre trafic web",
    "meta.ogDesc":
      "Une commande, zéro configuration. AES-256-GCM sur les segments que vous marquez.",
    "nav.features": "Fonctionnalités",
    "nav.quickstart": "Démarrage rapide",
    "nav.cli": "CLI",
    "nav.privacy": "Sécurité",
    "nav.menu": "Menu",
    "hero.alt":
      "Mascotte Brocolis : un brocoli en chapeau et lunettes de soleil devant un ordinateur portable",
    "hero.tagline":
      'Proxy MITM Rust pour le <strong>chiffrement sélectif</strong> de votre trafic web.',
    "hero.lead":
      'Marquez <code class="inv">((votre secret))</code> dans vos messages : Brocolis le chiffre en AES-256-GCM avant qu\'il ne quitte votre machine, et le déchiffre pour les utilisateurs qui partagent votre clé. Twitter, forums, messageries — sans rien changer côté serveur.',
    "hero.cta": "Télécharger pour macOS",
    "switch.lang": "Langue",
    "switch.theme": "Thème",
    "theme.green": "Vert",
    "theme.light": "Clair",
    "theme.dark": "Sombre",
    "s1.h": "Une commande, zéro configuration",
    "s1.li1":
      "Mot de passe administrateur demandé <strong>une seule fois</strong> (frontière de sécurité macOS)",
    "s1.li2":
      "Génère un certificat racine et l'approuve dans le <strong>trousseau Système</strong> (Safari, Chrome)",
    "s1.li3":
      "Active la confiance entreprise pour <strong>Firefox</strong> (toutes variantes détectées), ou dépose un profil <code>brocolis-firefox.mobileconfig</code> à approuver dans <em>Réglages Système</em> — sans modifier le bundle Firefox",
    "s1.li4": "Règle le proxy <strong>web et HTTPS</strong> du système",
    "s1.li5": "Démarre le chiffrement sélectif AES-256-GCM",
    "s1.li6":
      "<strong>Restaure exactement l'état initial</strong> à l'arrêt (Ctrl-C) : CA retiré, profil et politiques retirés, proxy remis comme avant",
    "s1.note":
      "Sans <code>sudo</code> ou hors macOS, <code>brocolis auto</code> démarre le proxy <strong>sans toucher au système</strong> (équivalent à <code>--no-system</code>) : pointez votre navigateur sur <code>127.0.0.1:&lt;port&gt;</code> manuellement.",
    "s2.h": "Fonctionnalités",
    "s2.c1.t": "Chiffrement sélectif",
    "s2.c1.d":
      "Seuls les segments <code>((entre doubles parenthèses))</code> sont chiffrés, en AES-256-GCM, sous la forme d'un jeton <code>[[BROCOLIS:v1:…]]</code>. Le serveur distant ne stocke que du chiffré.",
    "s2.c2.t": "MITM transparent",
    "s2.c2.d":
      "Interception HTTPS via <code>CONNECT</code>, certificats feuilles forgés à la volée par une autorité interne (<em>rcgen</em>), TLS terminé côté client avec <em>rustls</em>.",
    "s2.c3.t": "Clés partagées",
    "s2.c3.d":
      "Une clé symétrique 256 bits, partagée par canal sûr. Rotation immédiate via <code>SIGHUP</code>, rechargement via <code>SIGUSR1</code>.",
    "s2.c4.t": "Multi-navigateurs",
    "s2.c4.d":
      "Safari, Chrome (trousseau Système) et toutes les variantes Firefox (politique entreprise ou profil <code>.mobileconfig</code> sans modification de bundle) font confiance au CA automatiquement.",
    "s3.h": "Démarrage rapide",
    "s3.li1":
      "Téléchargez l'archive <code>brocolis-macos-universal.tar.gz</code> via le bouton ci-dessus (binaire universel Intel + Apple Silicon).",
    "s3.li2":
      "Lancez <code>sudo brocolis auto</code>. Le mot de passe administrateur n'est demandé qu'<strong>une fois</strong> (frontière de sécurité macOS).",
    "s3.li3":
      "Tapez vos messages en marquant ce qui doit rester privé : <code>((mon secret))</code>. Vos correspondants partageant la clé voient le clair ; tous les autres ne voient que du chiffré.",
    "s3.li4":
      "<code>Ctrl-C</code> pour tout arrêter : Brocolis remet votre système exactement dans son état initial.",
    "s4.h": "Compiler depuis les sources &amp; CLI manuel",
    "s4.intro":
      "Pour utiliser Brocolis hors macOS, ou tout piloter à la main :",
    "s4.code":
      "# Compiler (Rust stable, depuis les sources)\ncargo build --release          # binaire : target/release/brocolis\n\n# 1. Générer une clé symétrique AES-256 (à partager par canal sûr)\n$ brocolis keygen\nSomopQcHJyVQQAWedZFGY5E4+hxKjZemOB7aGnYr6kw=\n\n# 2. Exporter le certificat racine pour l'installer dans le navigateur\n$ brocolis ca --export brocolis-ca.crt\n\n# 3. Lancer le proxy (clé renseignée dans brocolis.toml, cf. brocolis.example.toml)\n$ brocolis run --config brocolis.toml\nBrocolis écoute sur http://127.0.0.1:8080 (proxy HTTP/HTTPS)",
    "s4.outro":
      "Sur Unix, la gestion des clés se pilote par signaux : <code>SIGHUP</code> régénère la clé (rotation en cas de compromission), <code>SIGUSR1</code> recharge la clé partagée depuis le fichier de configuration.",
    "s5.h": "Comment ça marche",
    "s5.intro":
      "Brocolis s'intercale entre votre navigateur et le service en ligne. À l'envoi, il chiffre les segments marqués ; à la réception, il déchiffre les jetons lisibles avec la clé partagée.",
    "s5.diagram":
      "Navigateur ─►  Brocolis (proxy local)  ─►  Serveur distant\n                       │                              │\n   « salut ((secret)) » ───► « salut [[BROCOLIS:v1:…]] » (chiffré sur le fil)\n   « salut secret »     ◄─── « salut [[BROCOLIS:v1:…]] » (déchiffré pour vous)",
    "s6.h": "Vie privée et sécurité",
    "s6.li1":
      "Chiffrement <strong>de bout en bout</strong> entre utilisateurs partageant la clé — le service distant ne voit jamais le clair.",
    "s6.li2":
      "Aucune télémétrie, aucun serveur, aucun compte — tout tourne en local.",
    "s6.li3":
      "Le CA est <strong>retiré du trousseau</strong> automatiquement à l'arrêt s'il a été ajouté par Brocolis.",
    "s6.li4":
      "Code source consultable pour audit (logiciel sous licence propriétaire).",
    "footer": "Brocolis — proxy MITM en Rust",
  },
  en: {
    "meta.title": "Brocolis 🥦 — Rust MITM proxy for selective encryption",
    "meta.desc":
      "A Rust MITM proxy that selectively encrypts HTTP/HTTPS traffic on Twitter, forums, messengers — with nothing to change on the server side.",
    "meta.ogTitle": "Brocolis 🥦 — Selective encryption for your web traffic",
    "meta.ogDesc":
      "One command, zero configuration. AES-256-GCM on the segments you mark.",
    "nav.features": "Features",
    "nav.quickstart": "Quick start",
    "nav.cli": "CLI",
    "nav.privacy": "Privacy",
    "nav.menu": "Menu",
    "hero.alt":
      "Brocolis mascot: a broccoli with hat and sunglasses in front of a laptop",
    "hero.tagline":
      "Rust MITM proxy for <strong>selective encryption</strong> of your web traffic.",
    "hero.lead":
      'Mark <code class="inv">((your secret))</code> in your messages: Brocolis encrypts it with AES-256-GCM before it leaves your machine, and decrypts it back for users who share your key. Twitter, forums, messengers — with nothing to change on the server side.',
    "hero.cta": "Download for macOS",
    "switch.lang": "Language",
    "switch.theme": "Theme",
    "theme.green": "Green",
    "theme.light": "Light",
    "theme.dark": "Dark",
    "s1.h": "One command, zero configuration",
    "s1.li1":
      "Admin password requested <strong>only once</strong> (a macOS security boundary)",
    "s1.li2":
      "Generates a root certificate and trusts it in the <strong>System keychain</strong> (Safari, Chrome)",
    "s1.li3":
      "Enables enterprise trust for <strong>Firefox</strong> (all detected variants), or drops a <code>brocolis-firefox.mobileconfig</code> profile to approve in <em>System Settings</em> — without modifying the Firefox bundle",
    "s1.li4": "Sets the system <strong>web and HTTPS</strong> proxy",
    "s1.li5": "Starts AES-256-GCM selective encryption",
    "s1.li6":
      "<strong>Restores the exact original state</strong> on exit (Ctrl-C): CA removed, profile and policies removed, proxy reset",
    "s1.note":
      "Without <code>sudo</code> or off macOS, <code>brocolis auto</code> starts the proxy <strong>without touching the system</strong> (equivalent to <code>--no-system</code>): point your browser at <code>127.0.0.1:&lt;port&gt;</code> manually.",
    "s2.h": "Features",
    "s2.c1.t": "Selective encryption",
    "s2.c1.d":
      "Only segments wrapped in <code>((double parentheses))</code> are encrypted, with AES-256-GCM, as a <code>[[BROCOLIS:v1:…]]</code> token. The remote server only ever stores ciphertext.",
    "s2.c2.t": "Transparent MITM",
    "s2.c2.d":
      "HTTPS interception via <code>CONNECT</code>, leaf certificates forged on the fly by an internal authority (<em>rcgen</em>), client-side TLS terminated with <em>rustls</em>.",
    "s2.c3.t": "Shared keys",
    "s2.c3.d":
      "A 256-bit symmetric key, shared over a secure channel. Immediate rotation via <code>SIGHUP</code>, reload via <code>SIGUSR1</code>.",
    "s2.c4.t": "Multi-browser",
    "s2.c4.d":
      "Safari, Chrome (System keychain) and all Firefox variants (enterprise policy or <code>.mobileconfig</code> profile, no bundle modification) trust the CA automatically.",
    "s3.h": "Quick start",
    "s3.li1":
      "Download the <code>brocolis-macos-universal.tar.gz</code> archive via the button above (universal Intel + Apple Silicon binary).",
    "s3.li2":
      "Run <code>sudo brocolis auto</code>. The admin password is requested <strong>only once</strong> (a macOS security boundary).",
    "s3.li3":
      "Type your messages marking what must stay private: <code>((my secret))</code>. Correspondents who share the key see the clear text; everyone else sees only ciphertext.",
    "s3.li4":
      "<code>Ctrl-C</code> to stop everything: Brocolis returns your system to its exact original state.",
    "s4.h": "Build from source &amp; manual CLI",
    "s4.intro": "To use Brocolis off macOS, or drive everything by hand:",
    "s4.code":
      "# Build (stable Rust, from source)\ncargo build --release          # binary: target/release/brocolis\n\n# 1. Generate an AES-256 symmetric key (share it over a secure channel)\n$ brocolis keygen\nSomopQcHJyVQQAWedZFGY5E4+hxKjZemOB7aGnYr6kw=\n\n# 2. Export the root certificate to install it in the browser\n$ brocolis ca --export brocolis-ca.crt\n\n# 3. Start the proxy (key set in brocolis.toml, see brocolis.example.toml)\n$ brocolis run --config brocolis.toml\nBrocolis listening on http://127.0.0.1:8080 (HTTP/HTTPS proxy)",
    "s4.outro":
      "On Unix, key management is driven by signals: <code>SIGHUP</code> regenerates the key (rotation on compromise), <code>SIGUSR1</code> reloads the shared key from the config file.",
    "s5.h": "How it works",
    "s5.intro":
      "Brocolis sits between your browser and the online service. On the way out, it encrypts marked segments; on the way back, it decrypts tokens readable with the shared key.",
    "s5.diagram":
      'Browser ─►  Brocolis (local proxy)  ─►  Remote server\n                    │                              │\n   "hi ((secret))" ───► "hi [[BROCOLIS:v1:…]]" (encrypted on the wire)\n   "hi secret"     ◄─── "hi [[BROCOLIS:v1:…]]" (decrypted for you)',
    "s6.h": "Privacy and security",
    "s6.li1":
      "<strong>End-to-end</strong> encryption between users sharing the key — the remote service never sees the clear text.",
    "s6.li2": "No telemetry, no server, no account — everything runs locally.",
    "s6.li3":
      "The CA is <strong>removed from the keychain</strong> automatically on exit if Brocolis added it.",
    "s6.li4":
      "Source code available for audit (proprietary-licensed software).",
    "footer": "Brocolis — Rust MITM proxy",
  },
  es: {
    "meta.title": "Brocolis 🥦 — Proxy MITM en Rust para cifrado selectivo",
    "meta.desc":
      "Proxy MITM en Rust que cifra selectivamente el tráfico HTTP/HTTPS en Twitter, foros y mensajerías — sin cambiar nada del lado del servidor.",
    "meta.ogTitle": "Brocolis 🥦 — Cifrado selectivo de tu tráfico web",
    "meta.ogDesc":
      "Un comando, cero configuración. AES-256-GCM en los segmentos que marques.",
    "nav.features": "Características",
    "nav.quickstart": "Inicio rápido",
    "nav.cli": "CLI",
    "nav.privacy": "Seguridad",
    "nav.menu": "Menú",
    "hero.alt":
      "Mascota de Brocolis: un brócoli con sombrero y gafas de sol frente a un portátil",
    "hero.tagline":
      "Proxy MITM en Rust para el <strong>cifrado selectivo</strong> de tu tráfico web.",
    "hero.lead":
      'Marca <code class="inv">((tu secreto))</code> en tus mensajes: Brocolis lo cifra con AES-256-GCM antes de que salga de tu máquina, y lo descifra para los usuarios que comparten tu clave. Twitter, foros, mensajerías — sin cambiar nada del lado del servidor.',
    "hero.cta": "Descargar para macOS",
    "switch.lang": "Idioma",
    "switch.theme": "Tema",
    "theme.green": "Verde",
    "theme.light": "Claro",
    "theme.dark": "Oscuro",
    "s1.h": "Un comando, cero configuración",
    "s1.li1":
      "Contraseña de administrador solicitada <strong>una sola vez</strong> (frontera de seguridad de macOS)",
    "s1.li2":
      "Genera un certificado raíz y lo aprueba en el <strong>llavero del Sistema</strong> (Safari, Chrome)",
    "s1.li3":
      "Activa la confianza empresarial para <strong>Firefox</strong> (todas las variantes detectadas), o deposita un perfil <code>brocolis-firefox.mobileconfig</code> para aprobar en <em>Ajustes del Sistema</em> — sin modificar el bundle de Firefox",
    "s1.li4": "Configura el proxy <strong>web y HTTPS</strong> del sistema",
    "s1.li5": "Inicia el cifrado selectivo AES-256-GCM",
    "s1.li6":
      "<strong>Restaura exactamente el estado inicial</strong> al salir (Ctrl-C): CA retirado, perfil y políticas retirados, proxy restaurado",
    "s1.note":
      "Sin <code>sudo</code> o fuera de macOS, <code>brocolis auto</code> arranca el proxy <strong>sin tocar el sistema</strong> (equivalente a <code>--no-system</code>): apunta tu navegador a <code>127.0.0.1:&lt;puerto&gt;</code> manualmente.",
    "s2.h": "Características",
    "s2.c1.t": "Cifrado selectivo",
    "s2.c1.d":
      "Solo los segmentos <code>((entre dobles paréntesis))</code> se cifran, con AES-256-GCM, como un token <code>[[BROCOLIS:v1:…]]</code>. El servidor remoto solo almacena texto cifrado.",
    "s2.c2.t": "MITM transparente",
    "s2.c2.d":
      "Interceptación HTTPS vía <code>CONNECT</code>, certificados hoja forjados al vuelo por una autoridad interna (<em>rcgen</em>), TLS terminado del lado cliente con <em>rustls</em>.",
    "s2.c3.t": "Claves compartidas",
    "s2.c3.d":
      "Una clave simétrica de 256 bits, compartida por un canal seguro. Rotación inmediata con <code>SIGHUP</code>, recarga con <code>SIGUSR1</code>.",
    "s2.c4.t": "Multi-navegador",
    "s2.c4.d":
      "Safari, Chrome (llavero del Sistema) y todas las variantes de Firefox (política empresarial o perfil <code>.mobileconfig</code> sin modificación del bundle) confían en el CA automáticamente.",
    "s3.h": "Inicio rápido",
    "s3.li1":
      "Descarga el archivo <code>brocolis-macos-universal.tar.gz</code> mediante el botón de arriba (binario universal Intel + Apple Silicon).",
    "s3.li2":
      "Ejecuta <code>sudo brocolis auto</code>. La contraseña de administrador solo se pide <strong>una vez</strong> (frontera de seguridad de macOS).",
    "s3.li3":
      "Escribe tus mensajes marcando lo que debe quedar privado: <code>((mi secreto))</code>. Los corresponsales que comparten la clave ven el texto en claro; el resto solo ve cifrado.",
    "s3.li4":
      "<code>Ctrl-C</code> para detener todo: Brocolis devuelve tu sistema exactamente a su estado inicial.",
    "s4.h": "Compilar desde las fuentes y CLI manual",
    "s4.intro":
      "Para usar Brocolis fuera de macOS, o controlarlo todo a mano:",
    "s4.code":
      "# Compilar (Rust estable, desde las fuentes)\ncargo build --release          # binario: target/release/brocolis\n\n# 1. Generar una clave simétrica AES-256 (a compartir por un canal seguro)\n$ brocolis keygen\nSomopQcHJyVQQAWedZFGY5E4+hxKjZemOB7aGnYr6kw=\n\n# 2. Exportar el certificado raíz para instalarlo en el navegador\n$ brocolis ca --export brocolis-ca.crt\n\n# 3. Lanzar el proxy (clave en brocolis.toml, cf. brocolis.example.toml)\n$ brocolis run --config brocolis.toml\nBrocolis escuchando en http://127.0.0.1:8080 (proxy HTTP/HTTPS)",
    "s4.outro":
      "En Unix, la gestión de claves se controla por señales: <code>SIGHUP</code> regenera la clave (rotación tras compromiso), <code>SIGUSR1</code> recarga la clave compartida desde el archivo de configuración.",
    "s5.h": "Cómo funciona",
    "s5.intro":
      "Brocolis se interpone entre tu navegador y el servicio en línea. Al salir, cifra los segmentos marcados; al volver, descifra los tokens legibles con la clave compartida.",
    "s5.diagram":
      "Navegador ─►  Brocolis (proxy local)  ─►  Servidor remoto\n                     │                              │\n   «hola ((secreto))» ───► «hola [[BROCOLIS:v1:…]]» (cifrado en el cable)\n   «hola secreto»     ◄─── «hola [[BROCOLIS:v1:…]]» (descifrado para ti)",
    "s6.h": "Privacidad y seguridad",
    "s6.li1":
      "Cifrado <strong>de extremo a extremo</strong> entre usuarios que comparten la clave — el servicio remoto nunca ve el texto en claro.",
    "s6.li2":
      "Sin telemetría, sin servidor, sin cuenta — todo se ejecuta localmente.",
    "s6.li3":
      "El CA se <strong>retira del llavero</strong> automáticamente al salir si Brocolis lo añadió.",
    "s6.li4":
      "Código fuente consultable para auditoría (software bajo licencia propietaria).",
    "footer": "Brocolis — proxy MITM en Rust",
  },
} as const satisfies Record<Lang, Record<string, string>>;

export type I18nKey = keyof (typeof DICT)["fr"];
