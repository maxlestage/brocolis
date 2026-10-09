use std::cell::Cell;
use std::rc::Rc;

use yew::prelude::*;

use crate::dom::{session_get, session_set, set_root_class};
use crate::motion::{FrameLoop, reduced};

const SEEN_KEY: &str = "brocolis.intro";

/// Durée du compteur, puis de la levée du rideau (ms). La levée doit rester
/// alignée sur la transition `.curtain[data-phase="up"]` de la feuille de style.
const COUNT_MS: f64 = 1100.0;
const HOLD_MS: f64 = 180.0;
const LIFT_MS: f64 = 1000.0;

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Count(u32),
    Up,
    Gone,
}

fn ease_out_expo(x: f64) -> f64 {
    if x >= 1.0 {
        1.0
    } else {
        1.0 - 2f64.powf(-10.0 * x)
    }
}

/// Rideau d'ouverture : la marque et un compteur 0 → 100, puis le rideau se
/// lève et le titre du héros se révèle dessous (classe `ready` sur `<html>`).
///
/// Une fois par session d'onglet seulement — le revoir à chaque rechargement
/// lasse vite — et jamais sous `prefers-reduced-motion`. Il ne bloque aucun
/// clic (`pointer-events: none`) : il ne fait qu'habiller l'arrivée.
#[function_component]
pub fn Curtain() -> Html {
    let phase = use_state(|| {
        if reduced() || session_get(SEEN_KEY).is_some() {
            Phase::Gone
        } else {
            Phase::Count(0)
        }
    });

    {
        let phase = phase.clone();
        use_effect_with((), move |()| {
            let lifted = Rc::new(Cell::new(false));
            let animation = (*phase != Phase::Gone).then(|| {
                session_set(SEEN_KEY, "1");
                let start = Rc::new(Cell::new(None::<f64>));
                let anim = FrameLoop::new(move |now| {
                    let t0 = start.get().unwrap_or(now);
                    start.set(Some(t0));
                    let elapsed = now - t0;
                    if elapsed < COUNT_MS {
                        let n = (ease_out_expo(elapsed / COUNT_MS) * 100.0).round() as u32;
                        phase.set(Phase::Count(n.min(100)));
                        true
                    } else if elapsed < COUNT_MS + HOLD_MS {
                        phase.set(Phase::Count(100));
                        true
                    } else if elapsed < COUNT_MS + HOLD_MS + LIFT_MS {
                        if !lifted.get() {
                            lifted.set(true);
                            set_root_class("ready", true);
                            phase.set(Phase::Up);
                        }
                        true
                    } else {
                        phase.set(Phase::Gone);
                        false
                    }
                });
                anim.wake();
                anim
            });
            if animation.is_none() {
                set_root_class("ready", true);
            }
            move || drop(animation)
        });
    }

    let (count, up) = match *phase {
        Phase::Gone => return html! {},
        Phase::Count(n) => (n, false),
        Phase::Up => (100, true),
    };

    html! {
        <div class="curtain" data-phase={if up { "up" } else { "count" }} aria-hidden="true">
            <div class="curtain-brand">
                <span class="curtain-leaf">{ "🥦" }</span>
                <span class="curtain-word">{ "Brocolis" }</span>
            </div>
            <div class="curtain-count">
                <span>{ format!("{count:03}") }</span>
                <span class="curtain-bar" style={format!("--n: {}", f64::from(count) / 100.0)} />
            </div>
        </div>
    }
}
