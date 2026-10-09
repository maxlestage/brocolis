//! Boucle `requestAnimationFrame` qui s'arrête d'elle-même.
//!
//! Une boucle d'animation qui tourne en permanence coûte de la batterie pour
//! rien : la plupart du temps, rien ne bouge. Celle-ci ne demande l'image
//! suivante que si la fonction de rendu le réclame (elle renvoie `true`), et se
//! réveille sur demande (`wake`) quand un événement — défilement, pointeur,
//! redimensionnement — rend une nouvelle image nécessaire.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::dom::window;

/// Fonction rappelée à chaque image, avec son horodatage.
type FrameCallback = Closure<dyn FnMut(f64)>;

struct State {
    pending: Cell<Option<i32>>,
    callback: RefCell<Option<FrameCallback>>,
}

impl State {
    fn request(&self) {
        if self.pending.get().is_some() {
            return;
        }
        if let Some(cb) = self.callback.borrow().as_ref() {
            let id = window()
                .request_animation_frame(cb.as_ref().unchecked_ref())
                .ok();
            self.pending.set(id);
        }
    }
}

/// Boucle d'animation. L'abandonner annule l'image en attente.
pub struct FrameLoop {
    state: Rc<State>,
}

impl FrameLoop {
    /// `tick` reçoit l'horodatage de l'image (ms) et renvoie `true` tant
    /// qu'il reste quelque chose à animer.
    pub fn new(mut tick: impl FnMut(f64) -> bool + 'static) -> Self {
        let state = Rc::new(State {
            pending: Cell::new(None),
            callback: RefCell::new(None),
        });
        // Référence faible : la fermeture ne doit pas garder vivant l'état
        // qui la possède, sinon la boucle ne serait jamais libérée.
        let weak = Rc::downgrade(&state);
        let callback = FrameCallback::new(move |now: f64| {
            let Some(state) = weak.upgrade() else { return };
            state.pending.set(None);
            if tick(now) {
                state.request();
            }
        });
        *state.callback.borrow_mut() = Some(callback);
        Self { state }
    }

    /// Programme une image si aucune n'est déjà en attente.
    pub fn wake(&self) {
        self.state.request();
    }
}

impl Drop for FrameLoop {
    fn drop(&mut self) {
        if let Some(id) = self.state.pending.take() {
            let _ = window().cancel_animation_frame(id);
        }
    }
}
