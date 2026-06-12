export const LANGS = ["fr", "en", "es"] as const;
export type Lang = (typeof LANGS)[number];

export const DICT = {
  fr: {
    "meta.title": "Brocolis 🥦 — Proxy MITM Rust de chiffrement sélectif",
    "meta.desc":
      "Proxy MITM en Rust qui chiffre sélectivement le trafic HTTP/HTTPS sur Twitter, les forums, les messageries. Configuration automatique sur macOS, Linux et Windows — sans rien changer côté serveur.",
    "meta.ogTitle": "Brocolis 🥦 — Chiffrement sélectif de votre trafic web",
    "meta.ogDesc":
      "Une commande, zéro configuration. AES-256-GCM sur les segments que vous marquez. macOS, Linux, Windows.",
    "nav.platforms": "Plateformes",
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
    "hero.cta": "Télécharger",
    "hero.cta.sub": "macOS · Linux · Windows",
    "switch.lang": "Langue",
    "switch.theme": "Thème",
    "theme.green": "Vert",
    "theme.light": "Clair",
    "theme.dark": "Sombre",
    "platforms.h": "Plateformes supportées",
    "platforms.intro":
      "<code>brocolis auto</code> détecte l'OS et fait le bon setup système — sans librairie spécifique côté code. Téléchargez la build pour votre plateforme :",
    "platforms.macos.t": "macOS",
    "platforms.macos.d":
      "Trousseau Système (Safari, Chrome, Edge) + politique entreprise Firefox + profil <code>.mobileconfig</code> alternatif + proxies web/HTTPS via <code>networksetup</code>. Binaire universel Intel + Apple Silicon.",
    "platforms.macos.cta": "Télécharger pour macOS",
    "platforms.linux.t": "Linux",
    "platforms.linux.d":
      "Magasin de confiance distro (Debian/Ubuntu, Fedora/RHEL, Arch détectés via leurs outils) + politique entreprise Firefox + variables <code>http_proxy</code> dans <code>/etc/profile.d/</code>. Disponible aussi en paquets <code>.deb</code> et <code>.rpm</code>.",
    "platforms.linux.cta": "Télécharger pour Linux",
    "platforms.windows.t": "Windows",
    "platforms.windows.d":
      "Magasin <em>Local Machine\\Trusted Root</em> via <code>certutil</code> + politique entreprise Firefox + proxy WinHTTP via <code>netsh</code>. Scripts PowerShell <code>install.ps1</code> / <code>uninstall.ps1</code> pour les installs persistantes.",
    "platforms.windows.cta": "Télécharger pour Windows",
    "nav.download": "Télécharger",
    "dl.back": "Accueil",
    "dl.h": "Télécharger Brocolis",
    "dl.intro":
      "Choisissez votre système d'exploitation. <code>brocolis auto</code> fait ensuite tout le réglage système — il suffit de lancer la commande.",
    "dl.version":
      "Ce sont les <strong>dernières builds</strong>, recompilées automatiquement à chaque mise à jour. Pour une version précise ou l'historique, voir toutes les releases.",
    "dl.macos.t": "macOS",
    "dl.macos.d":
      "Binaire universel Intel + Apple Silicon (<code>.tar.gz</code>).",
    "dl.macos.btn": "Télécharger (.tar.gz)",
    "dl.macos.note": "Décompressez, puis <code>sudo ./brocolis auto</code>.",
    "dl.linux.t": "Linux",
    "dl.linux.d":
      "Paquets x86_64 : <code>.deb</code> pour Debian/Ubuntu, <code>.rpm</code> pour Fedora/RHEL.",
    "dl.linux.deb": "Télécharger .deb",
    "dl.linux.rpm": "Télécharger .rpm",
    "dl.linux.note":
      "Installez le paquet, puis <code>sudo brocolis auto</code>.",
    "dl.windows.t": "Windows",
    "dl.windows.d":
      "Exécutable x86_64 + scripts d'installation (<code>.zip</code>).",
    "dl.windows.btn": "Télécharger (.zip)",
    "dl.windows.note":
      "Décompressez, puis lancez <code>brocolis auto</code> dans un terminal administrateur.",
    "dl.all": "Voir toutes les versions sur GitHub →",
    "s1.h": "Une commande, zéro configuration",
    "s1.li1":
      "Mot de passe administrateur demandé <strong>une seule fois</strong> (<code>sudo</code> sur macOS/Linux, élévation UAC sur Windows)",
    "s1.li2":
      "Approuve le CA dans le magasin de confiance natif : <strong>trousseau Système</strong> (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust extract-compat</code> (Linux), <code>certutil -addstore Root</code> (Windows)",
    "s1.li3":
      "Active la confiance entreprise pour <strong>Firefox</strong> (toutes variantes détectées). Sur macOS, génère aussi un profil <code>brocolis-firefox.mobileconfig</code> qui ne modifie pas le bundle Firefox",
    "s1.li4":
      "Règle le proxy système : <code>networksetup</code> (macOS), <code>/etc/profile.d/brocolis-proxy.sh</code> avec <code>http_proxy</code>/<code>https_proxy</code> (Linux), <code>netsh winhttp set proxy</code> (Windows)",
    "s1.li5": "Démarre le chiffrement sélectif AES-256-GCM",
    "s1.li6":
      "<strong>Restaure exactement l'état initial</strong> à l'arrêt (Ctrl-C) : CA retiré, politiques/profil/variables retirés, proxy remis comme avant",
    "s1.note":
      "Sans élévation administrateur, <code>brocolis auto</code> démarre le proxy <strong>sans toucher au système</strong> (équivalent à <code>--no-system</code>) : pointez votre navigateur sur <code>127.0.0.1:&lt;port&gt;</code> manuellement.",
    "s2.h": "Fonctionnalités",
    "s2.c1.t": "Chiffrement sélectif",
    "s2.c1.d":
      "Seuls les segments <code>((entre doubles parenthèses))</code> sont chiffrés, en AES-256-GCM, sous la forme d'un jeton <code>[[BROCOLIS:v1:…]]</code>. Le serveur distant ne stocke que du chiffré.",
    "s2.c2.t": "MITM transparent",
    "s2.c2.d":
      "Interception HTTPS via <code>CONNECT</code>, certificats feuilles forgés à la volée par une autorité interne (<em>rcgen</em>), TLS terminé côté client avec <em>rustls</em>.",
    "s2.c3.t": "Clés partagées",
    "s2.c3.d":
      "Une clé symétrique 256 bits, partagée par canal sûr. Rotation immédiate via <code>SIGHUP</code>, rechargement via <code>SIGUSR1</code> (Unix).",
    "s2.c4.t": "Multi-navigateurs",
    "s2.c4.d":
      "Safari, Chrome, Edge, Chromium et toutes les variantes Firefox font confiance au CA automatiquement — via le magasin de confiance natif de l'OS et la politique entreprise Firefox.",
    "s3.h": "Démarrage rapide",
    "s3.li1":
      "Téléchargez la build pour votre plateforme depuis la section ci-dessus (<code>.tar.gz</code> macOS universel, <code>.deb</code>/<code>.rpm</code> Linux ou <code>.zip</code> Windows x86_64).",
    "s3.li2":
      "Lancez <code>sudo brocolis auto</code> (macOS / Linux) ou <code>brocolis.exe auto</code> depuis un terminal <em>Exécuter en tant qu'administrateur</em> (Windows). Le mot de passe administrateur n'est demandé qu'<strong>une fois</strong>.",
    "s3.li3":
      "Tapez vos messages en marquant ce qui doit rester privé : <code>((mon secret))</code>. Vos correspondants partageant la clé voient le clair ; tous les autres ne voient que du chiffré.",
    "s3.li4":
      "<code>Ctrl-C</code> pour tout arrêter : Brocolis remet votre système exactement dans son état initial.",
    "s4.h": "Compiler depuis les sources &amp; CLI manuel",
    "s4.intro":
      "Pour compiler vous-même ou tout piloter à la main (les commandes sont identiques sur les trois plateformes) :",
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
      "Le CA est <strong>retiré du magasin de confiance</strong> automatiquement à l'arrêt s'il a été ajouté par Brocolis.",
    "s6.li4":
      "Code source consultable pour audit (logiciel sous licence propriétaire).",
    "footer": "Brocolis — proxy MITM en Rust",
  },
  en: {
    "meta.title": "Brocolis 🥦 — Rust MITM proxy for selective encryption",
    "meta.desc":
      "A Rust MITM proxy that selectively encrypts HTTP/HTTPS traffic on Twitter, forums, messengers. Automatic system setup on macOS, Linux and Windows — with nothing to change on the server side.",
    "meta.ogTitle": "Brocolis 🥦 — Selective encryption for your web traffic",
    "meta.ogDesc":
      "One command, zero configuration. AES-256-GCM on the segments you mark. macOS, Linux, Windows.",
    "nav.platforms": "Platforms",
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
    "hero.cta": "Download",
    "hero.cta.sub": "macOS · Linux · Windows",
    "switch.lang": "Language",
    "switch.theme": "Theme",
    "theme.green": "Green",
    "theme.light": "Light",
    "theme.dark": "Dark",
    "platforms.h": "Supported platforms",
    "platforms.intro":
      "<code>brocolis auto</code> detects the OS and runs the right system setup — no platform-specific library on the code side. Download the build for your platform:",
    "platforms.macos.t": "macOS",
    "platforms.macos.d":
      "System keychain (Safari, Chrome, Edge) + Firefox enterprise policy + alternative <code>.mobileconfig</code> profile + web/HTTPS proxies via <code>networksetup</code>. Universal binary Intel + Apple Silicon.",
    "platforms.macos.cta": "Download for macOS",
    "platforms.linux.t": "Linux",
    "platforms.linux.d":
      "Distro trust store (Debian/Ubuntu, Fedora/RHEL, Arch detected via their tools) + Firefox enterprise policy + <code>http_proxy</code> variables in <code>/etc/profile.d/</code>. Also available as <code>.deb</code> and <code>.rpm</code> packages.",
    "platforms.linux.cta": "Download for Linux",
    "platforms.windows.t": "Windows",
    "platforms.windows.d":
      "<em>Local Machine\\Trusted Root</em> store via <code>certutil</code> + Firefox enterprise policy + WinHTTP proxy via <code>netsh</code>. PowerShell scripts <code>install.ps1</code> / <code>uninstall.ps1</code> for persistent installs.",
    "platforms.windows.cta": "Download for Windows",
    "nav.download": "Download",
    "dl.back": "Home",
    "dl.h": "Download Brocolis",
    "dl.intro":
      "Pick your operating system. <code>brocolis auto</code> then handles the whole system setup — just run the command.",
    "dl.version":
      "These are the <strong>latest builds</strong>, recompiled automatically on every update. For a specific version or the history, see all releases.",
    "dl.macos.t": "macOS",
    "dl.macos.d":
      "Universal Intel + Apple Silicon binary (<code>.tar.gz</code>).",
    "dl.macos.btn": "Download (.tar.gz)",
    "dl.macos.note": "Unpack, then <code>sudo ./brocolis auto</code>.",
    "dl.linux.t": "Linux",
    "dl.linux.d":
      "x86_64 packages: <code>.deb</code> for Debian/Ubuntu, <code>.rpm</code> for Fedora/RHEL.",
    "dl.linux.deb": "Download .deb",
    "dl.linux.rpm": "Download .rpm",
    "dl.linux.note": "Install the package, then <code>sudo brocolis auto</code>.",
    "dl.windows.t": "Windows",
    "dl.windows.d":
      "x86_64 executable + install scripts (<code>.zip</code>).",
    "dl.windows.btn": "Download (.zip)",
    "dl.windows.note":
      "Unzip, then run <code>brocolis auto</code> in an administrator terminal.",
    "dl.all": "See all versions on GitHub →",
    "s1.h": "One command, zero configuration",
    "s1.li1":
      "Admin password requested <strong>only once</strong> (<code>sudo</code> on macOS/Linux, UAC elevation on Windows)",
    "s1.li2":
      "Trusts the CA in the native trust store: <strong>System keychain</strong> (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust extract-compat</code> (Linux), <code>certutil -addstore Root</code> (Windows)",
    "s1.li3":
      "Enables enterprise trust for <strong>Firefox</strong> (all detected variants). On macOS, also generates a <code>brocolis-firefox.mobileconfig</code> profile that doesn't modify the Firefox bundle",
    "s1.li4":
      "Sets the system proxy: <code>networksetup</code> (macOS), <code>/etc/profile.d/brocolis-proxy.sh</code> with <code>http_proxy</code>/<code>https_proxy</code> (Linux), <code>netsh winhttp set proxy</code> (Windows)",
    "s1.li5": "Starts AES-256-GCM selective encryption",
    "s1.li6":
      "<strong>Restores the exact original state</strong> on exit (Ctrl-C): CA removed, policies/profile/variables removed, proxy reset",
    "s1.note":
      "Without admin elevation, <code>brocolis auto</code> starts the proxy <strong>without touching the system</strong> (equivalent to <code>--no-system</code>): point your browser at <code>127.0.0.1:&lt;port&gt;</code> manually.",
    "s2.h": "Features",
    "s2.c1.t": "Selective encryption",
    "s2.c1.d":
      "Only segments wrapped in <code>((double parentheses))</code> are encrypted, with AES-256-GCM, as a <code>[[BROCOLIS:v1:…]]</code> token. The remote server only ever stores ciphertext.",
    "s2.c2.t": "Transparent MITM",
    "s2.c2.d":
      "HTTPS interception via <code>CONNECT</code>, leaf certificates forged on the fly by an internal authority (<em>rcgen</em>), client-side TLS terminated with <em>rustls</em>.",
    "s2.c3.t": "Shared keys",
    "s2.c3.d":
      "A 256-bit symmetric key, shared over a secure channel. Immediate rotation via <code>SIGHUP</code>, reload via <code>SIGUSR1</code> (Unix).",
    "s2.c4.t": "Multi-browser",
    "s2.c4.d":
      "Safari, Chrome, Edge, Chromium and all Firefox variants trust the CA automatically — via the native OS trust store and Firefox enterprise policy.",
    "s3.h": "Quick start",
    "s3.li1":
      "Download the build for your platform from the section above (universal macOS <code>.tar.gz</code>, Linux <code>.deb</code>/<code>.rpm</code>, or Windows <code>.zip</code> x86_64).",
    "s3.li2":
      "Run <code>sudo brocolis auto</code> (macOS / Linux) or <code>brocolis.exe auto</code> from a <em>Run as administrator</em> terminal (Windows). The admin password is requested <strong>only once</strong>.",
    "s3.li3":
      "Type your messages marking what must stay private: <code>((my secret))</code>. Correspondents who share the key see the clear text; everyone else sees only ciphertext.",
    "s3.li4":
      "<code>Ctrl-C</code> to stop everything: Brocolis returns your system to its exact original state.",
    "s4.h": "Build from source &amp; manual CLI",
    "s4.intro":
      "To build it yourself or drive everything by hand (commands are identical on all three platforms):",
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
      "The CA is <strong>removed from the trust store</strong> automatically on exit if Brocolis added it.",
    "s6.li4":
      "Source code available for audit (proprietary-licensed software).",
    "footer": "Brocolis — Rust MITM proxy",
  },
  es: {
    "meta.title": "Brocolis 🥦 — Proxy MITM en Rust para cifrado selectivo",
    "meta.desc":
      "Proxy MITM en Rust que cifra selectivamente el tráfico HTTP/HTTPS en Twitter, foros y mensajerías. Configuración automática en macOS, Linux y Windows — sin cambiar nada del lado del servidor.",
    "meta.ogTitle": "Brocolis 🥦 — Cifrado selectivo de tu tráfico web",
    "meta.ogDesc":
      "Un comando, cero configuración. AES-256-GCM en los segmentos que marques. macOS, Linux, Windows.",
    "nav.platforms": "Plataformas",
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
    "hero.cta": "Descargar",
    "hero.cta.sub": "macOS · Linux · Windows",
    "switch.lang": "Idioma",
    "switch.theme": "Tema",
    "theme.green": "Verde",
    "theme.light": "Claro",
    "theme.dark": "Oscuro",
    "platforms.h": "Plataformas soportadas",
    "platforms.intro":
      "<code>brocolis auto</code> detecta el SO y hace la configuración correcta — sin librería específica de plataforma del lado del código. Descarga la build para tu plataforma:",
    "platforms.macos.t": "macOS",
    "platforms.macos.d":
      "Llavero del Sistema (Safari, Chrome, Edge) + política empresarial Firefox + perfil <code>.mobileconfig</code> alternativo + proxies web/HTTPS vía <code>networksetup</code>. Binario universal Intel + Apple Silicon.",
    "platforms.macos.cta": "Descargar para macOS",
    "platforms.linux.t": "Linux",
    "platforms.linux.d":
      "Almacén de confianza distro (Debian/Ubuntu, Fedora/RHEL, Arch detectados vía sus herramientas) + política empresarial Firefox + variables <code>http_proxy</code> en <code>/etc/profile.d/</code>. También disponible como paquetes <code>.deb</code> y <code>.rpm</code>.",
    "platforms.linux.cta": "Descargar para Linux",
    "platforms.windows.t": "Windows",
    "platforms.windows.d":
      "Almacén <em>Local Machine\\Trusted Root</em> vía <code>certutil</code> + política empresarial Firefox + proxy WinHTTP vía <code>netsh</code>. Scripts PowerShell <code>install.ps1</code> / <code>uninstall.ps1</code> para instalaciones persistentes.",
    "platforms.windows.cta": "Descargar para Windows",
    "nav.download": "Descargar",
    "dl.back": "Inicio",
    "dl.h": "Descargar Brocolis",
    "dl.intro":
      "Elige tu sistema operativo. <code>brocolis auto</code> se encarga de toda la configuración del sistema — solo ejecuta el comando.",
    "dl.version":
      "Estas son las <strong>últimas builds</strong>, recompiladas automáticamente en cada actualización. Para una versión concreta o el historial, mira todas las releases.",
    "dl.macos.t": "macOS",
    "dl.macos.d":
      "Binario universal Intel + Apple Silicon (<code>.tar.gz</code>).",
    "dl.macos.btn": "Descargar (.tar.gz)",
    "dl.macos.note": "Descomprime y luego <code>sudo ./brocolis auto</code>.",
    "dl.linux.t": "Linux",
    "dl.linux.d":
      "Paquetes x86_64: <code>.deb</code> para Debian/Ubuntu, <code>.rpm</code> para Fedora/RHEL.",
    "dl.linux.deb": "Descargar .deb",
    "dl.linux.rpm": "Descargar .rpm",
    "dl.linux.note": "Instala el paquete y luego <code>sudo brocolis auto</code>.",
    "dl.windows.t": "Windows",
    "dl.windows.d":
      "Ejecutable x86_64 + scripts de instalación (<code>.zip</code>).",
    "dl.windows.btn": "Descargar (.zip)",
    "dl.windows.note":
      "Descomprime y ejecuta <code>brocolis auto</code> en una terminal de administrador.",
    "dl.all": "Ver todas las versiones en GitHub →",
    "s1.h": "Un comando, cero configuración",
    "s1.li1":
      "Contraseña de administrador solicitada <strong>una sola vez</strong> (<code>sudo</code> en macOS/Linux, elevación UAC en Windows)",
    "s1.li2":
      "Aprueba el CA en el almacén de confianza nativo: <strong>llavero del Sistema</strong> (macOS), <code>update-ca-certificates</code> / <code>update-ca-trust</code> / <code>trust extract-compat</code> (Linux), <code>certutil -addstore Root</code> (Windows)",
    "s1.li3":
      "Activa la confianza empresarial para <strong>Firefox</strong> (todas las variantes detectadas). En macOS, también genera un perfil <code>brocolis-firefox.mobileconfig</code> que no modifica el bundle de Firefox",
    "s1.li4":
      "Configura el proxy del sistema: <code>networksetup</code> (macOS), <code>/etc/profile.d/brocolis-proxy.sh</code> con <code>http_proxy</code>/<code>https_proxy</code> (Linux), <code>netsh winhttp set proxy</code> (Windows)",
    "s1.li5": "Inicia el cifrado selectivo AES-256-GCM",
    "s1.li6":
      "<strong>Restaura exactamente el estado inicial</strong> al salir (Ctrl-C): CA retirado, políticas/perfil/variables retiradas, proxy restaurado",
    "s1.note":
      "Sin elevación de administrador, <code>brocolis auto</code> arranca el proxy <strong>sin tocar el sistema</strong> (equivalente a <code>--no-system</code>): apunta tu navegador a <code>127.0.0.1:&lt;puerto&gt;</code> manualmente.",
    "s2.h": "Características",
    "s2.c1.t": "Cifrado selectivo",
    "s2.c1.d":
      "Solo los segmentos <code>((entre dobles paréntesis))</code> se cifran, con AES-256-GCM, como un token <code>[[BROCOLIS:v1:…]]</code>. El servidor remoto solo almacena texto cifrado.",
    "s2.c2.t": "MITM transparente",
    "s2.c2.d":
      "Interceptación HTTPS vía <code>CONNECT</code>, certificados hoja forjados al vuelo por una autoridad interna (<em>rcgen</em>), TLS terminado del lado cliente con <em>rustls</em>.",
    "s2.c3.t": "Claves compartidas",
    "s2.c3.d":
      "Una clave simétrica de 256 bits, compartida por un canal seguro. Rotación inmediata con <code>SIGHUP</code>, recarga con <code>SIGUSR1</code> (Unix).",
    "s2.c4.t": "Multi-navegador",
    "s2.c4.d":
      "Safari, Chrome, Edge, Chromium y todas las variantes de Firefox confían en el CA automáticamente — vía el almacén de confianza nativo del SO y la política empresarial Firefox.",
    "s3.h": "Inicio rápido",
    "s3.li1":
      "Descarga la build para tu plataforma desde la sección de arriba (<code>.tar.gz</code> macOS universal, <code>.deb</code>/<code>.rpm</code> Linux, o <code>.zip</code> Windows x86_64).",
    "s3.li2":
      "Ejecuta <code>sudo brocolis auto</code> (macOS / Linux) o <code>brocolis.exe auto</code> desde un terminal <em>Ejecutar como administrador</em> (Windows). La contraseña de administrador solo se pide <strong>una vez</strong>.",
    "s3.li3":
      "Escribe tus mensajes marcando lo que debe quedar privado: <code>((mi secreto))</code>. Los corresponsales que comparten la clave ven el texto en claro; el resto solo ve cifrado.",
    "s3.li4":
      "<code>Ctrl-C</code> para detener todo: Brocolis devuelve tu sistema exactamente a su estado inicial.",
    "s4.h": "Compilar desde las fuentes y CLI manual",
    "s4.intro":
      "Para compilarlo tú mismo o controlarlo todo a mano (los comandos son idénticos en las tres plataformas):",
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
      "El CA se <strong>retira del almacén de confianza</strong> automáticamente al salir si Brocolis lo añadió.",
    "s6.li4":
      "Código fuente consultable para auditoría (software bajo licencia propietaria).",
    "footer": "Brocolis — proxy MITM en Rust",
  },
} as const satisfies Record<Lang, Record<string, string>>;

export type I18nKey = keyof (typeof DICT)["fr"];
