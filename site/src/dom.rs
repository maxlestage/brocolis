//! Les quelques accès au DOM qui ne passent pas par le rendu de Yew : titre de
//! l'onglet, balises `<meta>`, attributs de `<html>`, stockage local,
//! défilement, écouteurs d'événements globaux.
//!
//! Tout y est tolérant à l'échec. Le stockage local lève une exception en
//! navigation privée sur certains navigateurs, et `querySelector` peut ne rien
//! trouver : aucun de ces cas ne doit empêcher le site de s'afficher.

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{Document, Event, EventTarget, Window};

pub fn window() -> Window {
    web_sys::window().expect("pas de `window` : ce code ne tourne que dans un navigateur")
}

pub fn document() -> Document {
    window()
        .document()
        .expect("pas de `document` : ce code ne tourne que dans un navigateur")
}

/// Lit une clé du stockage local, ou `None` si le stockage est indisponible.
pub fn storage_get(key: &str) -> Option<String> {
    window().local_storage().ok()??.get_item(key).ok()?
}

/// Écrit une clé dans le stockage local. Échec silencieux : une préférence
/// non mémorisée n'est pas une raison de casser la page.
pub fn storage_set(key: &str, value: &str) {
    if let Ok(Some(store)) = window().local_storage() {
        let _ = store.set_item(key, value);
    }
}

/// Langue préférée du navigateur, telle que déclarée par `navigator.language`.
pub fn navigator_language() -> Option<String> {
    window().navigator().language()
}

/// Positionne un attribut sur `<html>`.
pub fn set_root_attribute(name: &str, value: &str) {
    if let Some(root) = document().document_element() {
        let _ = root.set_attribute(name, value);
    }
}

/// Met à jour le `content` d'une balise `<meta>` déjà présente dans le
/// document. Si le sélecteur ne correspond à rien, il n'y a rien à faire.
pub fn set_meta(selector: &str, content: &str) {
    if let Ok(Some(el)) = document().query_selector(selector) {
        let _ = el.set_attribute("content", content);
    }
}

pub fn set_title(title: &str) {
    document().set_title(title);
}

/// Remonte en haut de page. La douceur du défilement vient de la CSS
/// (`scroll-behavior`), elle-même désactivée sous
/// `prefers-reduced-motion: reduce` — donc rien à décider ici.
pub fn scroll_to_top() {
    window().scroll_to_with_x_and_y(0.0, 0.0);
}

/// Amène une section dans la vue par son identifiant.
pub fn scroll_to_id(id: &str) {
    if let Some(el) = document().get_element_by_id(id) {
        el.scroll_into_view();
    }
}

/// Déplace le focus clavier sur un élément et l'amène dans la vue.
///
/// Le focus, pas seulement le défilement : c'est tout l'intérêt du lien
/// d'évitement, qui doit sortir la tabulation de la barre de navigation.
/// La cible doit porter `tabindex="-1"` si ce n'est pas un élément
/// naturellement focalisable.
pub fn focus_id(id: &str) {
    if let Some(el) = document().get_element_by_id(id) {
        if let Some(focusable) = el.dyn_ref::<web_sys::HtmlElement>() {
            let _ = focusable.focus();
        }
        el.scroll_into_view();
    }
}

/// Écouteur d'événement global dont la durée de vie est celle de la valeur :
/// il se retire du DOM à la destruction. C'est ce qui permet de le rendre
/// depuis un effet de Yew sans fuite ni écouteur fantôme.
pub struct Listener {
    target: EventTarget,
    kind: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}

impl Listener {
    /// Écoute un événement sur `window`.
    pub fn on_window(kind: &'static str, handler: impl FnMut(Event) + 'static) -> Self {
        let target: EventTarget = window().into();
        let callback = Closure::<dyn FnMut(Event)>::new(handler);
        let _ = target.add_event_listener_with_callback(kind, callback.as_ref().unchecked_ref());
        Self {
            target,
            kind,
            callback,
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.kind, self.callback.as_ref().unchecked_ref());
    }
}
