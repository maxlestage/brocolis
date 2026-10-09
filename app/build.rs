//! Sous Windows, l'icône et les informations de version sont des ressources
//! intégrées à l'exécutable : l'explorateur et la barre des tâches les lisent
//! là, pas dans la fenêtre. Ailleurs, rien à faire (l'icône de fenêtre est
//! posée à l'exécution, et le paquet fournit celle du lanceur).

fn main() {
    println!("cargo:rerun-if-changed=assets/brocolis.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/brocolis.ico")
        .set("ProductName", "Brocolis")
        .set("FileDescription", "Brocolis — chiffrement sélectif")
        .set("CompanyName", "Maxime Nathan Lestage")
        .set("LegalCopyright", "© Maxime Nathan Lestage");
    if let Err(e) = res.compile() {
        // Sans compilateur de ressources, l'application reste utilisable :
        // seule l'icône de l'exécutable manque. On le signale sans échouer.
        println!("cargo:warning=icône Windows non intégrée : {e}");
    }
}
