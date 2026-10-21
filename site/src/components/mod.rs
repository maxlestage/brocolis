//! Les composants du site : des fonctions qui renvoient une vue `active`.
//!
//! Leurs noms évitent ceux des balises (`footer`, `header`, `nav`…), que les
//! modules importent toutes depuis `active::html`.

pub mod ambient;
pub mod app_mock;
pub mod curtain;
pub mod demo;
pub mod download;
pub mod footer;
pub mod hero;
pub mod landing;
pub mod manifesto;
pub mod marquee;
pub mod navbar;
pub mod protocol;
pub mod sections;
pub mod skip_link;
pub mod switchers;

pub use ambient::ambient;
pub use curtain::curtain;
pub use download::download_page;
pub use landing::landing;
pub use protocol::protocol_page;
