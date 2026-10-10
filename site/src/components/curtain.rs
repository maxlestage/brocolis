use active::html::*;
use active::{Node, fragment_dyn, use_state};

use crate::dom::{session_get, session_set, set_root_class};
use crate::markup::keep;
use crate::motion::{FrameLoop, reduced};

const SEEN_KEY: &str = "brocolis.intro";

/// Durée du compteur, puis de la levée du rideau (ms). La levée doit rester
/// alignée sur la transition `.curtain[data-phase="up"]` de la feuille de style.
const COUNT_MS: f64 = 1100.0;
const HOLD_MS: f64 = 180.0;
const LIFT_MS: f64 = 1000.0;

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
pub fn curtain() -> Node {
    if reduced() || session_get(SEEN_KEY).is_some() {
        set_root_class("ready", true);
        return Node::Empty;
    }
    session_set(SEEN_KEY, "1");

    let count = use_state(0u32);
    let up = use_state(false);
    let shown = use_state(true);

    let mut start = None::<f64>;
    let animation = FrameLoop::new(move |now| {
        let elapsed = now - *start.get_or_insert(now);
        if elapsed < COUNT_MS {
            let n = ((ease_out_expo(elapsed / COUNT_MS) * 100.0).round() as u32).min(100);
            if count.with(|c| *c != n) {
                count.set(n);
            }
            true
        } else if elapsed < COUNT_MS + HOLD_MS {
            if count.with(|c| *c != 100) {
                count.set(100);
            }
            true
        } else if elapsed < COUNT_MS + HOLD_MS + LIFT_MS {
            if !up.get() {
                set_root_class("ready", true);
                up.set(true);
            }
            true
        } else {
            shown.set(false);
            false
        }
    });
    animation.wake();
    keep(animation);

    // Seule cette région lit `shown` : le rideau est retiré du DOM une fois
    // levé, le compteur et la phase sont liés sans le reconstruire.
    fragment_dyn(move || {
        if !shown.get() {
            return Vec::new();
        }
        vec![
            div()
                .class("curtain")
                .attr_dyn("data-phase", move || {
                    if up.get() { "up" } else { "count" }.to_owned()
                })
                .attr("aria-hidden", "true")
                .child(
                    div()
                        .class("curtain-brand")
                        .child(span().class("curtain-leaf").text("🥦"))
                        .child(span().class("curtain-word").text("Brocolis")),
                )
                .child(
                    div()
                        .class("curtain-count")
                        .child(span().text_dyn(move || format!("{:03}", count.get())))
                        .child(span().class("curtain-bar").attr_dyn("style", move || {
                            format!("--n: {}", f64::from(count.get()) / 100.0)
                        })),
                )
                .into(),
        ]
    })
}
