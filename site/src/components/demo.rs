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

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{IntersectionObserver, IntersectionObserverEntry};
use yew::prelude::*;

use crate::motion::{FrameLoop, glyph, reduced};
use crate::settings::use_t;

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

#[function_component]
pub fn Demo() -> Html {
    let t = use_t();
    let clock = use_state(|| (STILL, 0u32));
    let host = use_node_ref();

    {
        let (clock, host) = (clock.clone(), host.clone());
        use_effect_with((), move |()| {
            let mut keep = None;
            if !reduced()
                && let Some(el) = host.cast::<web_sys::Element>()
            {
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
                        clock.set((elapsed % CYCLE, (elapsed / CYCLE) as u32));
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
                if let Ok(io) = IntersectionObserver::new(on_view.as_ref().unchecked_ref()) {
                    io.observe(&el);
                    keep = Some((io, on_view, frame));
                }
            }
            move || {
                if let Some((io, _, _)) = &keep {
                    io.disconnect();
                }
                drop(keep);
            }
        });
    }

    let (time, round) = *clock;
    let plain = format!("(({}))", t.demo_secret);
    let sealed = token(round);

    // Ligne 1 : le message se tape, caractère par caractère.
    let typed_len = {
        let total =
            t.demo_before.chars().count() + plain.chars().count() + t.demo_after.chars().count();
        ((time / TYPE_END).min(1.0) * total as f64).round() as usize
    };
    let mut left = typed_len;
    let mut take = |s: &str| {
        let n = s.chars().count().min(left);
        left -= n;
        s.chars().take(n).collect::<String>()
    };
    let (w_before, w_secret, w_after) = (take(t.demo_before), take(&plain), take(t.demo_after));
    let typing = time < TYPE_END;

    // Ligne 2 : le secret se scelle en jeton. Ligne 3 : il se rouvre.
    let seal = phase(time, SEAL);
    let open = phase(time, OPEN);
    let server = morph(&plain, &sealed, seal);
    let friend = if open > 0.0 {
        morph(&sealed, t.demo_secret, open)
    } else {
        server.clone()
    };

    let row = |n: usize, on: bool, state: Option<(&str, bool)>, body: Html| {
        let chip = state.map(|(label, open)| {
            html! {
                <span class={classes!("demo-chip", open.then_some("open"))}>
                    <span class="demo-chip-icon">{ if open { "🔓" } else { "🔒" } }</span>
                    { label }
                </span>
            }
        });
        html! {
            <div class={classes!("demo-row", on.then_some("on"))}>
                <span class="demo-label">
                    <span class="demo-n">{ format!("{:02}", n + 1) }</span>
                    { t.demo_rows[n] }
                </span>
                <p class="demo-line">{ body }</p>
                { for chip }
            </div>
        }
    };

    let fading = time >= FADE;

    html! {
        <section class="demo" aria-labelledby="demo-h">
            <p class="kicker" data-reveal="" data-scramble="">{ t.demo_kicker }</p>
            <h2 id="demo-h" class="demo-h" data-reveal="mask">{ t.demo_h }</h2>
            <p class="sr-only">{ t.demo_sr }</p>
            <div
                ref={host}
                class={classes!("demo-board", fading.then_some("fading"))}
                aria-hidden="true"
                data-reveal="card"
                data-tilt=""
                data-spot=""
            >
                { row(0, true, None, html! {
                    <>
                        { w_before }
                        <mark class="demo-secret">{ w_secret }</mark>
                        { w_after }
                        if typing { <span class="demo-caret" /> }
                    </>
                }) }
                { row(1, time >= SEAL.0, Some((t.demo_locked, false)), html! {
                    <>
                        { t.demo_before }
                        <span class={classes!("demo-token", (seal >= 1.0).then_some("sealed"))}>{ server }</span>
                        { t.demo_after }
                    </>
                }) }
                { row(2, time >= OPEN.0, Some(if open >= 1.0 { (t.demo_unlocked, true) } else { (t.demo_locked, false) }), html! {
                    <>
                        { t.demo_before }
                        <span class={classes!("demo-token", (open >= 1.0).then_some("clear"))}>{ friend }</span>
                        { t.demo_after }
                    </>
                }) }
                { row(3, time >= OPEN.0, Some((t.demo_locked, false)), html! {
                    <>
                        { t.demo_before }
                        <span class="demo-token sealed">{ sealed.clone() }</span>
                        { t.demo_after }
                    </>
                }) }
            </div>
        </section>
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
