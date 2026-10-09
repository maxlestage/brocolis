//! Le mouvement du site, inspiré de zamocorp.com : scène d'ouverture épinglée
//! où la mascotte se forme dans la poussière, balayage de lumière, curseur
//! anneau, boutons aimantés, apparitions au défilement, manifeste qui s'allume
//! mot à mot, barre de progression, barre de navigation qui s'efface.
//!
//! Un seul contrôleur, une seule boucle d'animation. Les composants Yew ne
//! savent rien de tout ça : ils posent des attributs (`data-reveal`,
//! `data-magnetic`, `data-spot`) et des classes, et ce module les découvre dans
//! le DOM. Le DOM est surveillé (`MutationObserver`) : un changement de page ou
//! de langue est pris en compte sans que les composants aient à prévenir.
//!
//! Rien ne passe par l'état de Yew : les valeurs qui changent à chaque image
//! sont écrites en variables CSS (`--progress`, `--p`, `--mx`…), et c'est la
//! feuille de style qui en fait du mouvement. Yew ne réconcilie que les
//! attributs qu'il a lui-même posés, il ne touche donc jamais à ceux-là.
//!
//! Le défilement reste celui du navigateur — pas de défilement « lissé » qui
//! détourne la molette : c'est ce qui casse le plus souvent l'accessibilité et
//! la recherche dans la page sur ce genre de site.
//!
//! Sous `prefers-reduced-motion: reduce`, seul l'état de la barre de
//! navigation est suivi ; le reste est statique et entièrement lisible.

mod dust;
mod frame;

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{
    CanvasRenderingContext2d, Element, HtmlCanvasElement, HtmlElement, HtmlImageElement,
    IntersectionObserver, IntersectionObserverEntry, IntersectionObserverInit, MutationObserver,
    MutationObserverInit, PointerEvent,
};

use crate::dom::{Listener, document, matches_media, root, set_root_class, window};
use dust::{Dust, SETTLED, context_2d};
pub use frame::FrameLoop;

/// Vrai si l'utilisateur demande moins de mouvement.
pub fn reduced() -> bool {
    matches_media("(prefers-reduced-motion: reduce)")
}

/// Pose sur `<html>` les classes dont dépend la feuille de style, AVANT le
/// premier rendu : `motion` (les animations sont permises) et `fine` (souris
/// ou pavé tactile, donc curseur et aimantation ont un sens).
pub fn flag_document() {
    let motion = !reduced();
    set_root_class("motion", motion);
    set_root_class(
        "fine",
        motion && matches_media("(hover: hover) and (pointer: fine)"),
    );
}

fn set_var(el: &HtmlElement, name: &str, value: f64) {
    let _ = el.style().set_property(name, &format!("{value:.4}"));
}

fn rect(el: &Element) -> (f64, f64, f64, f64) {
    let r = el.get_bounding_client_rect();
    (r.left(), r.top(), r.width(), r.height())
}

/// La scène d'ouverture : une section haute dont l'intérieur reste épinglé
/// (`position: sticky`) pendant que la poussière se rassemble.
struct Hero {
    pin: HtmlElement,
    stage: Element,
    art: Element,
    img: HtmlImageElement,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    dust: Option<Dust>,
    /// Taille de la scène en pixels CSS, à laquelle le canevas est réglé.
    size: (f64, f64),
    progress: f64,
    _load: Listener,
}

struct Pointer {
    x: f64,
    y: f64,
    /// Position affichée de l'anneau : elle rattrape le pointeur avec retard.
    rx: f64,
    ry: f64,
    seen: bool,
}

struct Inner {
    animated: bool,
    fine: bool,
    /// Le DOM a changé : refaire l'inventaire des éléments animés.
    dirty: bool,
    /// Le défilement ou la taille ont changé : recalculer ce qui en dépend.
    moved: bool,
    last_y: f64,
    nav_hidden: bool,
    scrolled: bool,
    hero: Option<Hero>,
    manifesto: Option<HtmlElement>,
    parallax: Vec<(HtmlElement, f64)>,
    cursor: Option<HtmlElement>,
    pointer: Pointer,
    magnet: Option<HtmlElement>,
    reveal: Option<IntersectionObserver>,
}

/// Le contrôleur. Le garder en vie suffit ; le lâcher retire tout.
pub struct Motion {
    _frame: Rc<FrameLoop>,
    _listeners: Vec<Listener>,
    _mutations: (MutationObserver, Closure<dyn FnMut()>),
    _reveal: Option<Closure<dyn FnMut(js_sys::Array, IntersectionObserver)>>,
}

impl Motion {
    pub fn start() -> Self {
        let animated = !reduced();
        let fine = animated && matches_media("(hover: hover) and (pointer: fine)");

        // Apparitions : un élément `data-reveal` reçoit `data-in` la première
        // fois qu'il entre dans la vue, puis n'est plus observé.
        let reveal_cb = animated.then(|| {
            Closure::<dyn FnMut(js_sys::Array, IntersectionObserver)>::new(
                |entries: js_sys::Array, observer: IntersectionObserver| {
                    for entry in entries.iter() {
                        let entry: IntersectionObserverEntry = entry.unchecked_into();
                        if entry.is_intersecting() {
                            let target = entry.target();
                            let _ = target.set_attribute("data-in", "");
                            observer.unobserve(&target);
                        }
                    }
                },
            )
        });
        let reveal = reveal_cb.as_ref().and_then(|cb| {
            let init = IntersectionObserverInit::new();
            init.set_root_margin("0px 0px -8% 0px");
            IntersectionObserver::new_with_options(cb.as_ref().unchecked_ref(), &init).ok()
        });

        let inner = Rc::new(RefCell::new(Inner {
            animated,
            fine,
            dirty: true,
            moved: true,
            last_y: window().scroll_y().unwrap_or(0.0),
            nav_hidden: false,
            scrolled: false,
            hero: None,
            manifesto: None,
            parallax: Vec::new(),
            cursor: None,
            pointer: Pointer {
                x: 0.0,
                y: 0.0,
                rx: 0.0,
                ry: 0.0,
                seen: false,
            },
            magnet: None,
            reveal,
        }));

        let frame = {
            let inner = inner.clone();
            Rc::new(FrameLoop::new(move |now| inner.borrow_mut().tick(now)))
        };

        let mut listeners = Vec::new();
        {
            let (inner, frame) = (inner.clone(), frame.clone());
            listeners.push(Listener::on_window("scroll", move |_| {
                inner.borrow_mut().moved = true;
                frame.wake();
            }));
        }
        {
            let (inner, frame) = (inner.clone(), frame.clone());
            listeners.push(Listener::on_window("resize", move |_| {
                // La taille du canevas et la géométrie de la scène changent :
                // tout l'inventaire est à refaire.
                inner.borrow_mut().dirty = true;
                frame.wake();
            }));
        }
        if fine {
            {
                let (inner, frame) = (inner.clone(), frame.clone());
                listeners.push(Listener::on_window("pointermove", move |ev| {
                    if let Some(ev) = ev.dyn_ref::<PointerEvent>() {
                        inner.borrow_mut().on_pointer(ev);
                        frame.wake();
                    }
                }));
            }
            let inner = inner.clone();
            if let Some(html) = document().document_element() {
                listeners.push(Listener::on(html.into(), "pointerleave", move |_| {
                    inner.borrow_mut().release_magnet();
                }));
            }
        }

        // Tout ajout ou retrait de nœud — nouvelle page, nouvelle langue —
        // relance l'inventaire à l'image suivante. Seul attribut suivi :
        // `data-loaded`, posé quand l'image du héros a fini de charger. Les
        // autres (`style`, `data-in`) sont écrits par ce module lui-même : les
        // suivre ferait tourner l'inventaire en boucle.
        let on_mutation = {
            let (inner, frame) = (inner.clone(), frame.clone());
            Closure::<dyn FnMut()>::new(move || {
                inner.borrow_mut().dirty = true;
                frame.wake();
            })
        };
        let observer = MutationObserver::new(on_mutation.as_ref().unchecked_ref())
            .expect("MutationObserver est disponible dans tout navigateur cible");
        let init = MutationObserverInit::new();
        init.set_child_list(true);
        init.set_subtree(true);
        init.set_attributes(true);
        init.set_attribute_filter(&js_sys::Array::of1(&"data-loaded".into()));
        if let Some(body) = document().body() {
            let _ = observer.observe_with_options(&body, &init);
        }

        frame.wake();
        Self {
            _frame: frame,
            _listeners: listeners,
            _mutations: (observer, on_mutation),
            _reveal: reveal_cb,
        }
    }
}

impl Drop for Motion {
    fn drop(&mut self) {
        self._mutations.0.disconnect();
    }
}

impl Inner {
    /// Une image. Renvoie `true` s'il faut la suivante.
    fn tick(&mut self, now: f64) -> bool {
        if self.dirty {
            self.dirty = false;
            self.moved = true;
            self.scan();
        }
        if self.moved {
            self.moved = false;
            self.on_scroll();
        }
        let mut busy = self.follow_cursor();
        busy |= self.draw_hero(now);
        busy
    }

    /// Inventaire des éléments animés présents dans le DOM.
    fn scan(&mut self) {
        let doc = document();
        if let Some(io) = &self.reveal
            && let Ok(list) = doc.query_selector_all("[data-reveal]:not([data-in])")
        {
            for i in 0..list.length() {
                if let Some(el) = list.get(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                    io.observe(&el);
                }
            }
        }

        self.cursor = doc
            .query_selector(".cursor")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into().ok());

        if !self.animated {
            return;
        }

        self.manifesto = doc
            .query_selector(".manifesto")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into().ok());

        self.parallax.clear();
        if let Ok(list) = doc.query_selector_all("[data-speed]") {
            for i in 0..list.length() {
                let Some(el) = list.get(i).and_then(|n| n.dyn_into::<HtmlElement>().ok()) else {
                    continue;
                };
                let speed = el
                    .get_attribute("data-speed")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                self.parallax.push((el, speed));
            }
        }

        self.scan_hero();
    }

    fn scan_hero(&mut self) {
        let doc = document();
        let Some(pin) = doc
            .query_selector(".hero-pin")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<HtmlElement>().ok())
        else {
            self.hero = None;
            return;
        };

        // Même scène qu'avant (simple changement de langue, redimensionnement) :
        // on garde la poussière déjà échantillonnée, on recale le canevas.
        let same = self
            .hero
            .as_ref()
            .is_some_and(|h| h.pin.is_same_node(Some(&pin)));
        if !same {
            let q = |sel: &str| pin.query_selector(sel).ok().flatten();
            let (Some(stage), Some(art), Some(img), Some(canvas)) = (
                q(".hero-stage"),
                q(".hero-art"),
                q(".hero-art img").and_then(|e| e.dyn_into::<HtmlImageElement>().ok()),
                q("canvas.hero-dust").and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok()),
            ) else {
                self.hero = None;
                return;
            };
            let Some(ctx) = context_2d(&canvas) else {
                self.hero = None;
                return;
            };
            // L'image n'est peut-être pas encore décodée. Son chargement pose
            // `data-loaded` sur la scène, attribut que le `MutationObserver`
            // suit : l'inventaire est alors refait et la poussière échantillonnée.
            let load = Listener::on(img.clone().into(), "load", {
                let pin = pin.clone();
                move |_| {
                    let _ = pin.set_attribute("data-loaded", "");
                }
            });
            self.hero = Some(Hero {
                pin,
                stage,
                art,
                img,
                canvas,
                ctx,
                dust: None,
                size: (0.0, 0.0),
                progress: -1.0,
                _load: load,
            });
        }

        let Some(hero) = self.hero.as_mut() else {
            return;
        };
        if hero.dust.is_none() {
            hero.dust = Dust::sample(&hero.img);
        }
        // Canevas à la taille de la scène, en pixels physiques (plafonnés à 2
        // pour ne pas multiplier le coût par 9 sur un téléphone à 3x).
        let (_, _, w, h) = rect(&hero.stage);
        let dpr = window().device_pixel_ratio().min(2.0);
        hero.canvas.set_width((w * dpr) as u32);
        hero.canvas.set_height((h * dpr) as u32);
        let _ = hero.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
        hero.size = (w, h);
        hero.progress = -1.0;
    }

    fn on_scroll(&mut self) {
        let win = window();
        let y = win.scroll_y().unwrap_or(0.0);
        let vh = win
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0);
        let doc_h = document()
            .document_element()
            .map_or(vh, |e| f64::from(e.scroll_height()));

        // Barre de navigation : fond dès qu'on quitte le haut de page, et
        // effacement quand on descend (elle revient dès qu'on remonte).
        let scrolled = y > 8.0;
        if scrolled != self.scrolled {
            self.scrolled = scrolled;
            set_root_class("scrolled", scrolled);
        }
        let hidden = if y < vh * 0.5 {
            false
        } else if y > self.last_y + 3.0 {
            true
        } else if y < self.last_y - 3.0 {
            false
        } else {
            self.nav_hidden
        };
        if hidden != self.nav_hidden {
            self.nav_hidden = hidden;
            set_root_class("nav-hidden", hidden);
        }
        self.last_y = y;

        if !self.animated {
            return;
        }

        if let Some(root) = root() {
            set_var(
                &root,
                "--progress",
                (y / (doc_h - vh).max(1.0)).clamp(0.0, 1.0),
            );
        }

        if let Some(m) = &self.manifesto {
            let (_, top, _, height) = rect(m);
            let p = (vh * 0.85 - top) / (height + vh * 0.25);
            set_var(m, "--p", p.clamp(0.0, 1.0));
        }

        for (el, speed) in &self.parallax {
            let (_, top, _, height) = rect(el);
            let offset = (top + height / 2.0 - vh / 2.0) * speed;
            let _ = el.style().set_property("--py", &format!("{offset:.1}px"));
        }
    }

    fn draw_hero(&mut self, now: f64) -> bool {
        let Some(hero) = self.hero.as_mut() else {
            return false;
        };

        if hero.dust.is_none() {
            return false;
        }

        let (_, top, _, height) = rect(&hero.pin);
        let (_, stage_top, w, h) = rect(&hero.stage);
        // Hors de la vue : rien à dessiner, et surtout rien à animer.
        if top + height < 0.0 || stage_top > h {
            return false;
        }
        let progress = (-top / (height - h).max(1.0)).clamp(0.0, 1.0);
        set_var(&hero.pin, "--hero", progress);

        let settled = progress >= SETTLED;
        // Posée et immobile : inutile de redessiner la même image.
        if settled && (hero.progress - progress).abs() < f64::EPSILON {
            return false;
        }
        hero.progress = progress;

        let (sx, sy, _, _) = rect(&hero.stage);
        let (ax, ay, aw, _) = rect(&hero.art);
        if let Some(dust) = &hero.dust {
            dust.draw(&hero.ctx, (w, h), (ax - sx, ay - sy, aw), progress, now);
        }
        // Tant que des grains flottent, ils dérivent : il faut l'image suivante.
        !settled
    }

    fn on_pointer(&mut self, ev: &PointerEvent) {
        let (x, y) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
        let p = &mut self.pointer;
        if !p.seen {
            // Premier mouvement : l'anneau apparaît sous le pointeur, il ne
            // traverse pas l'écran depuis le coin.
            p.seen = true;
            p.rx = x;
            p.ry = y;
            if let Some(c) = &self.cursor {
                let _ = c.set_attribute("data-on", "");
            }
        }
        p.x = x;
        p.y = y;

        let target = ev.target().and_then(|t| t.dyn_into::<Element>().ok());
        let closest = |sel: &str| target.as_ref().and_then(|t| t.closest(sel).ok().flatten());

        if let Some(c) = &self.cursor {
            let hover = closest("a, button, [role=button], pre[tabindex]").is_some();
            let _ = if hover {
                c.set_attribute("data-hover", "")
            } else {
                c.remove_attribute("data-hover")
            };
        }

        // Projecteur des cartes : position du pointeur dans la carte.
        if let Some(card) = closest("[data-spot]").and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
            let (l, t, _, _) = rect(&card);
            let _ = card
                .style()
                .set_property("--sx", &format!("{:.0}px", x - l));
            let _ = card
                .style()
                .set_property("--sy", &format!("{:.0}px", y - t));
        }

        // Aimantation : l'élément glisse vers le pointeur, d'un tiers de
        // l'écart à son centre.
        let magnet = closest("[data-magnetic]").and_then(|e| e.dyn_into::<HtmlElement>().ok());
        if let Some(m) = &magnet {
            let (l, t, w, h) = rect(m);
            // L'élément s'est déjà déplacé de --mx/--my : on raisonne sur son
            // centre au repos pour ne pas entrer en oscillation.
            let style = m.style();
            let px = |n: &str| {
                style
                    .get_property_value(n)
                    .ok()
                    .and_then(|v| v.trim_end_matches("px").parse::<f64>().ok())
                    .unwrap_or(0.0)
            };
            let (cx, cy) = (l + w / 2.0 - px("--mx"), t + h / 2.0 - px("--my"));
            let _ = style.set_property("--mx", &format!("{:.1}px", (x - cx) * 0.3));
            let _ = style.set_property("--my", &format!("{:.1}px", (y - cy) * 0.3));
        }
        let changed = match (&self.magnet, &magnet) {
            (Some(a), Some(b)) => !a.is_same_node(Some(b)),
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.release_magnet();
            self.magnet = magnet;
        }
    }

    fn release_magnet(&mut self) {
        if let Some(m) = self.magnet.take() {
            let _ = m.style().remove_property("--mx");
            let _ = m.style().remove_property("--my");
        }
    }

    /// L'anneau rattrape le pointeur. Renvoie `true` tant qu'il n'y est pas.
    fn follow_cursor(&mut self) -> bool {
        if !self.fine || !self.pointer.seen {
            return false;
        }
        let Some(c) = &self.cursor else { return false };
        let p = &mut self.pointer;
        p.rx += (p.x - p.rx) * 0.2;
        p.ry += (p.y - p.ry) * 0.2;
        let _ = c.style().set_property(
            "transform",
            &format!("translate3d({:.1}px, {:.1}px, 0)", p.rx, p.ry),
        );
        (p.x - p.rx).abs() > 0.1 || (p.y - p.ry).abs() > 0.1
    }
}
