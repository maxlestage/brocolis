//! Démonstration animée du chiffrement sélectif.
//!
//! Quatre lignes, le même message vu par quatre lecteurs : vous l'écrivez en
//! marquant le secret, le serveur ne stocke que le jeton chiffré, vos proches
//! (qui ont la clé) le lisent en clair, les autres ne voient que le jeton. La
//! scène boucle tant qu'elle est à l'écran et s'arrête dès qu'elle en sort.
//!
//! Tout le rendu dépend d'une seule valeur, le temps écoulé dans le cycle :
//! pas d'état intermédiaire à synchroniser. Sans mouvement, la scène est
//! figée sur son état final, complet et lisible.
//!
//! L'animation est cachée aux lecteurs d'écran (elle épellerait des glyphes
//! aléatoires) ; un paragraphe dédié décrit la même chose en une phrase.

use std::cell::Cell;
use std::rc::Rc;

use active::html::*;
use active::{Element, Node, State, fragment_dyn, text_dyn, use_state};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{IntersectionObserver, IntersectionObserverEntry};

use crate::markup::{Translated, keep, tr};
use crate::motion::{FrameLoop, glyph, reduced};
use crate::settings::t;

/// Durée d'un cycle complet, et jalons (ms).
const CYCLE: f64 = 10_500.0;
const TYPE_END: f64 = 2_000.0;
const SEAL: (f64, f64) = (2_500.0, 3_500.0);
const OPEN: (f64, f64) = (3_900.0, 5_000.0);
const FADE: f64 = 9_900.0;
/// État affiché quand rien ne bouge : tout est posé.
const STILL: f64 = 6_000.0;

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Jeton chiffré du cycle `n` — différent à chaque tour, stable pendant le
/// tour (un générateur congruentiel suffit : c'est un décor).
pub(crate) fn token(n: u32) -> String {
    let mut s = n.wrapping_mul(2_654_435_761).wrapping_add(0x9E37_79B9);
    let body: String = (0..18)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            B64[(s % 64) as usize] as char
        })
        .collect();
    format!("[[BROCOLIS:v1:{body}…]]")
}

/// Passe de `from` à `to` caractère par caractère, de gauche à droite ; les
/// caractères pas encore fixés défilent en glyphes aléatoires.
fn morph(from: &str, to: &str, k: f64) -> String {
    let (a, b): (Vec<char>, Vec<char>) = (from.chars().collect(), to.chars().collect());
    let n = a.len().max(b.len());
    (0..n)
        .filter_map(|i| {
            let fixed = (i as f64 + 1.0) / n as f64 <= k;
            if fixed {
                b.get(i).copied()
            } else if k > 0.0 {
                Some(glyph())
            } else {
                a.get(i).copied()
            }
        })
        .collect()
}

fn phase(t: f64, (start, end): (f64, f64)) -> f64 {
    ((t - start) / (end - start)).clamp(0.0, 1.0)
}

/// Le temps de la scène : instant dans le cycle (ms) et numéro du tour.
#[derive(Clone, Copy)]
struct Clock(State<(f64, u32)>);

impl Clock {
    fn time(self) -> f64 {
        self.0.with(|c| c.0)
    }

    fn round(self) -> u32 {
        self.0.with(|c| c.1)
    }

    /// Le secret, tel que tapé : `((…))`.
    fn plain() -> String {
        format!("(({}))", t().demo_secret)
    }

    /// Ligne 1 : le message se tape, caractère par caractère. Renvoie les
    /// trois morceaux déjà tapés (avant, secret, après).
    fn typed(self) -> [String; 3] {
        let t = t();
        let plain = Self::plain();
        let parts = [t.demo_before, plain.as_str(), t.demo_after];
        let total: usize = parts.iter().map(|s| s.chars().count()).sum();
        let mut left = ((self.time() / TYPE_END).min(1.0) * total as f64).round() as usize;
        parts.map(|s| {
            let n = s.chars().count().min(left);
            left -= n;
            s.chars().take(n).collect()
        })
    }

    /// Ligne 2 : le secret se scelle en jeton.
    fn server(self) -> String {
        morph(
            &Self::plain(),
            &token(self.round()),
            phase(self.time(), SEAL),
        )
    }

    /// Ligne 3 : chez les proches, le jeton se rouvre.
    fn friend(self) -> String {
        let open = phase(self.time(), OPEN);
        if open > 0.0 {
            morph(&token(self.round()), t().demo_secret, open)
        } else {
            self.server()
        }
    }

    fn opened(self) -> bool {
        phase(self.time(), OPEN) >= 1.0
    }
}

/// Une ligne de la scène : numéro, lecteur, message et, sauf pour l'auteur,
/// l'état du message chez ce lecteur — verrouillé, ou ouvert quand `open`
/// devient vrai.
fn row(
    n: usize,
    on: impl Fn() -> bool + 'static,
    open: Option<fn(Clock) -> bool>,
    clock: Clock,
    line: Element,
) -> Element {
    let chip = open.map(|open| {
        let open = move || open(clock);
        span()
            .class("demo-chip")
            .class_if("open", open)
            .child(
                span()
                    .class("demo-chip-icon")
                    .text_dyn(move || if open() { "🔓" } else { "🔒" }.to_owned()),
            )
            .child(tr(move |t| {
                if open() {
                    t.demo_unlocked
                } else {
                    t.demo_locked
                }
            }))
    });
    div()
        .class("demo-row")
        .class_if("on", on)
        .child(
            span()
                .class("demo-label")
                .child(span().class("demo-n").text(format!("{:02}", n + 1)))
                .tr(move |t| t.demo_rows[n]),
        )
        .child(line.class("demo-line"))
        .child(chip)
}

/// Message dont le secret est remplacé par `token` (lignes 2 à 4).
fn token_line(token: Element) -> Element {
    p().tr(|t| t.demo_before).child(token).tr(|t| t.demo_after)
}

pub fn demo() -> Node {
    let clock = Clock(use_state((STILL, 0u32)));
    // Le curseur n'existe que pendant la frappe. Un état à part, écrit
    // seulement quand il change : la région qui l'ajoute ou le retire n'est
    // pas reconstruite à chaque image.
    let typing = use_state(STILL < TYPE_END);

    let typed = move |i: usize| text_dyn(move || clock.typed()[i].clone());
    let line0 = p()
        .child(typed(0))
        .child(mark().class("demo-secret").child(typed(1)))
        .child(typed(2))
        .child(fragment_dyn(move || {
            if typing.get() {
                vec![span().class("demo-caret").into()]
            } else {
                Vec::new()
            }
        }));
    let line1 = token_line(
        span()
            .class("demo-token")
            .class_if("sealed", move || phase(clock.time(), SEAL) >= 1.0)
            .text_dyn(move || clock.server()),
    );
    let line2 = token_line(
        span()
            .class("demo-token")
            .class_if("clear", move || clock.opened())
            .text_dyn(move || clock.friend()),
    );
    let line3 = token_line(
        span()
            .class("demo-token sealed")
            .text_dyn(move || token(clock.round())),
    );

    let sealed: fn(Clock) -> bool = |_| false;
    let board = div()
        .class("demo-board")
        .class_if("fading", move || clock.time() >= FADE)
        .attr("aria-hidden", "true")
        .attr("data-reveal", "card")
        .attr("data-tilt", "")
        .attr("data-spot", "")
        .on_mount(move |el| start(el, clock, typing))
        .child(row(0, || true, None, clock, line0))
        .child(row(
            1,
            move || clock.time() >= SEAL.0,
            Some(sealed),
            clock,
            line1,
        ))
        .child(row(
            2,
            move || clock.time() >= OPEN.0,
            Some(Clock::opened),
            clock,
            line2,
        ))
        .child(row(
            3,
            move || clock.time() >= OPEN.0,
            Some(sealed),
            clock,
            line3,
        ));

    section()
        .class("demo")
        .attr("aria-labelledby", "demo-h")
        .child(
            p().class("kicker")
                .attr("data-reveal", "")
                .attr("data-scramble", "")
                .tr(|t| t.demo_kicker),
        )
        .child(
            h2().id("demo-h")
                .class("demo-h")
                .attr("data-reveal", "mask")
                .tr(|t| t.demo_h),
        )
        .child(p().class("sr-only").tr(|t| t.demo_sr))
        .child(board)
        .into()
}

/// La scène boucle tant qu'elle est à l'écran et s'arrête dès qu'elle en
/// sort. Sans mouvement, elle reste figée sur son état final.
fn start(el: &web_sys::Element, clock: Clock, typing: State<bool>) {
    if reduced() {
        return;
    }
    let visible = Rc::new(Cell::new(false));
    let started = Rc::new(Cell::new(None::<f64>));
    let frame = Rc::new(FrameLoop::new({
        let (visible, started) = (visible.clone(), started.clone());
        move |now| {
            if !visible.get() {
                return false;
            }
            let t0 = started.get().unwrap_or(now);
            started.set(Some(t0));
            let elapsed = now - t0;
            let time = elapsed % CYCLE;
            clock.0.set((time, (elapsed / CYCLE) as u32));
            if typing.with(|on| *on != (time < TYPE_END)) {
                typing.set(time < TYPE_END);
            }
            true
        }
    }));
    let on_view = Closure::<dyn FnMut(js_sys::Array)>::new({
        let frame = frame.clone();
        move |entries: js_sys::Array| {
            for entry in entries.iter() {
                let entry: IntersectionObserverEntry = entry.unchecked_into();
                visible.set(entry.is_intersecting());
            }
            if visible.get() {
                frame.wake();
            }
        }
    });
    if let Ok(observer) = IntersectionObserver::new(on_view.as_ref().unchecked_ref()) {
        observer.observe(el);
        // Libéré avec la page : l'observateur est déconnecté, l'image en
        // attente annulée.
        keep(Watch {
            observer,
            _on_view: on_view,
            _frame: frame,
        });
    }
}

struct Watch {
    observer: IntersectionObserver,
    _on_view: Closure<dyn FnMut(js_sys::Array)>,
    _frame: Rc<FrameLoop>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.observer.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::{morph, token};

    #[test]
    fn le_jeton_a_la_forme_brocolis_et_change_a_chaque_tour() {
        let a = token(0);
        assert!(a.starts_with("[[BROCOLIS:v1:") && a.ends_with("…]]"), "{a}");
        assert_eq!(a, token(0), "stable pendant un tour");
        assert_ne!(a, token(1), "différent d'un tour à l'autre");
    }

    #[test]
    fn la_transition_part_de_la_source_et_arrive_a_la_cible() {
        // Aux deux bornes, aucun glyphe aléatoire n'est tiré.
        assert_eq!(morph("((demain))", "[[X]]", 0.0), "((demain))");
        assert_eq!(morph("((demain))", "[[X]]", 1.0), "[[X]]");
        assert_eq!(morph("[[BROCOLIS:v1:abc]]", "demain", 1.0), "demain");
    }
}
