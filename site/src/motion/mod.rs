//! Le mouvement du site, inspiré de zamocorp.com.
//!
//! Un seul contrôleur, une seule boucle d'animation. Les composants ne
//! savent rien de tout ça : ils posent des attributs, et ce module les découvre
//! dans le DOM. Le DOM est surveillé (`MutationObserver`) : un changement de
//! page ou de langue est pris en compte sans que les composants aient à
//! prévenir.
//!
//! Attributs reconnus :
//!
//! | attribut          | effet                                                    |
//! |-------------------|----------------------------------------------------------|
//! | `data-reveal`     | reçoit `data-in` en entrant dans la vue (la CSS anime)   |
//! | `data-scramble`   | texte « déchiffré » à l'apparition et au survol          |
//! | `data-progress`   | reçoit `--p` (0 → 1) selon sa traversée de l'écran       |
//! | `data-speed`      | reçoit `--py`, décalage de parallaxe                     |
//! | `data-marquee`    | piste défilante, accélérée par la vitesse de défilement  |
//! | `data-magnetic`   | attiré par le pointeur (`--mx`, `--my`)                  |
//! | `data-tilt`       | incliné en 3D sous le pointeur (`--rx`, `--ry`)          |
//! | `data-spot`       | projecteur sous le pointeur (`--sx`, `--sy`)             |
//! | `data-cursor`     | libellé affiché dans le curseur au survol                |
//!
//! Sur `<html>` : `--progress` (avancement de la page), `--velocity` (vitesse
//! de défilement lissée, -1 → 1), `--pointer-x` / `--pointer-y` (position du
//! pointeur, -1 → 1), `--spy` (rang de la section courante), et les classes
//! `scrolled`, `nav-hidden`, `spy-on`.
//!
//! Rien ne passe par les états des vues : les valeurs qui changent à chaque
//! image sont écrites en variables CSS, et c'est la feuille de style qui en
//! fait du mouvement. `active` ne met à jour que les attributs qu'une vue a
//! liés, il ne touche donc jamais à ceux-là.
//!
//! Le défilement reste celui du navigateur — pas de défilement « lissé » qui
//! détourne la molette : c'est ce qui casse le plus souvent l'accessibilité et
//! la recherche dans la page sur ce genre de site.
//!
//! Sous `prefers-reduced-motion: reduce`, seul l'état de la barre de
//! navigation et la section courante sont suivis ; le reste est statique et
//! entièrement lisible.

mod dust;
mod frame;
mod scramble;

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
use scramble::Job;
pub use scramble::glyph;

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

fn set_px(el: &HtmlElement, name: &str, value: f64) {
    let _ = el.style().set_property(name, &format!("{value:.1}px"));
}

fn rect(el: &Element) -> (f64, f64, f64, f64) {
    let r = el.get_bounding_client_rect();
    (r.left(), r.top(), r.width(), r.height())
}

fn all(selector: &str) -> Vec<Element> {
    let Ok(list) = document().query_selector_all(selector) else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.get(i))
        .filter_map(|n| n.dyn_into::<Element>().ok())
        .collect()
}

fn viewport() -> (f64, f64) {
    let win = window();
    let num = |v: Result<wasm_bindgen::JsValue, _>| v.ok().and_then(|v| v.as_f64());
    (
        num(win.inner_width()).unwrap_or(1280.0),
        num(win.inner_height()).unwrap_or(800.0),
    )
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
    progress: f64,
    _load: Listener,
}

/// Comment `--p` est calculé pour un élément `data-progress`.
#[derive(Clone, Copy)]
enum Span {
    /// 0 quand le haut entre par le bas de l'écran, 1 quand le bas sort par
    /// le haut : la traversée complète.
    Through,
    /// 0 quand le haut entre, 1 quand l'élément est entièrement visible.
    Enter,
    /// Pour la lecture : 1 quand le bas a remonté aux trois quarts de l'écran.
    Read,
}

struct Tracked {
    el: HtmlElement,
    span: Span,
    last: f64,
}

struct Marquee {
    host: Element,
    track: HtmlElement,
    dir: f64,
    offset: f64,
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
    last_now: f64,
    last_y: f64,
    /// Vitesse de défilement lissée (px/ms) et sens du dernier mouvement.
    velocity: f64,
    sample: f64,
    heading: f64,
    nav_hidden: bool,
    scrolled: bool,
    spy: Option<usize>,
    spy_on: bool,
    hero: Option<Hero>,
    tracked: Vec<Tracked>,
    parallax: Vec<(HtmlElement, f64)>,
    marquees: Vec<Marquee>,
    sections: Vec<Element>,
    nav_links: Vec<Element>,
    cursor: Option<HtmlElement>,
    cursor_label: Option<String>,
    pointer: Pointer,
    magnet: Option<HtmlElement>,
    tilt: Option<HtmlElement>,
    hovered: Option<Element>,
    reveal: Option<IntersectionObserver>,
    scrambles: Vec<Job>,
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

        let inner = Rc::new(RefCell::new(Inner {
            animated,
            fine,
            dirty: true,
            moved: true,
            last_now: 0.0,
            last_y: window().scroll_y().unwrap_or(0.0),
            velocity: 0.0,
            sample: 0.0,
            heading: 1.0,
            nav_hidden: false,
            scrolled: false,
            spy: None,
            spy_on: false,
            hero: None,
            tracked: Vec::new(),
            parallax: Vec::new(),
            marquees: Vec::new(),
            sections: Vec::new(),
            nav_links: Vec::new(),
            cursor: None,
            cursor_label: None,
            pointer: Pointer {
                x: 0.0,
                y: 0.0,
                rx: 0.0,
                ry: 0.0,
                seen: false,
            },
            magnet: None,
            tilt: None,
            hovered: None,
            reveal: None,
            scrambles: Vec::new(),
        }));

        let frame = {
            let inner = inner.clone();
            Rc::new(FrameLoop::new(move |now| inner.borrow_mut().tick(now)))
        };

        // Apparitions : un élément `data-reveal` reçoit `data-in` la première
        // fois qu'il entre dans la vue, puis n'est plus observé. S'il porte
        // aussi `data-scramble`, son texte se déchiffre à ce moment-là.
        let reveal_cb = animated.then(|| {
            let (inner, frame) = (inner.clone(), frame.clone());
            Closure::<dyn FnMut(js_sys::Array, IntersectionObserver)>::new(
                move |entries: js_sys::Array, observer: IntersectionObserver| {
                    let mut started = false;
                    for entry in entries.iter() {
                        let entry: IntersectionObserverEntry = entry.unchecked_into();
                        if !entry.is_intersecting() {
                            continue;
                        }
                        let target = entry.target();
                        let _ = target.set_attribute("data-in", "");
                        observer.unobserve(&target);
                        if target.has_attribute("data-scramble") {
                            inner.borrow_mut().scramble(&target, 120.0);
                            started = true;
                        }
                    }
                    if started {
                        frame.wake();
                    }
                },
            )
        });
        inner.borrow_mut().reveal = reveal_cb.as_ref().and_then(|cb| {
            let init = IntersectionObserverInit::new();
            init.set_root_margin("0px 0px -8% 0px");
            IntersectionObserver::new_with_options(cb.as_ref().unchecked_ref(), &init).ok()
        });

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
                    let mut inner = inner.borrow_mut();
                    inner.release_magnet();
                    inner.release_tilt();
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
        let dt = if self.last_now > 0.0 {
            (now - self.last_now).clamp(0.0, 64.0)
        } else {
            16.0
        };
        self.last_now = now;

        if self.dirty {
            self.dirty = false;
            self.moved = true;
            self.scan();
        }
        let scrolled_now = self.moved;
        if self.moved {
            self.moved = false;
            self.on_scroll(dt);
        }
        if !self.animated {
            return false;
        }
        let mut busy = self.ease_velocity(scrolled_now);
        busy |= self.follow_cursor();
        busy |= self.draw_hero(now);
        busy |= self.run_marquees(dt);
        busy |= self.run_scrambles(now);
        busy
    }

    /// Inventaire des éléments animés présents dans le DOM.
    fn scan(&mut self) {
        let doc = document();
        if let Some(io) = &self.reveal {
            for el in all("[data-reveal]:not([data-in]), [data-scramble]:not([data-in])") {
                io.observe(&el);
            }
        }

        self.cursor = doc
            .query_selector(".cursor")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into().ok());
        self.sections = all("main section[id]");
        self.nav_links = all(".nav-links a[href^='#']");
        // Les liens viennent peut-être d'être recréés (changement de langue) :
        // l'état courant doit être reposé dessus.
        self.spy = None;

        if !self.animated {
            return;
        }

        self.tracked = all("[data-progress]")
            .into_iter()
            .filter_map(|e| {
                let span = match e.get_attribute("data-progress").as_deref() {
                    Some("enter") => Span::Enter,
                    Some("read") => Span::Read,
                    _ => Span::Through,
                };
                Some(Tracked {
                    el: e.dyn_into().ok()?,
                    span,
                    last: -1.0,
                })
            })
            .collect();

        self.parallax = all("[data-speed]")
            .into_iter()
            .filter_map(|e| {
                let speed = e.get_attribute("data-speed")?.parse().ok()?;
                Some((e.dyn_into().ok()?, speed))
            })
            .collect();

        // Une piste déjà connue garde son décalage : sinon un simple
        // changement de langue la ferait sauter au début.
        let old = std::mem::take(&mut self.marquees);
        self.marquees = all("[data-marquee]")
            .into_iter()
            .filter_map(|host| {
                let track: HtmlElement = host
                    .query_selector(".marquee-track")
                    .ok()??
                    .dyn_into()
                    .ok()?;
                let offset = old
                    .iter()
                    .find(|m| m.host.is_same_node(Some(&host)))
                    .map_or(0.0, |m| m.offset);
                let dir = if host.get_attribute("data-marquee").as_deref() == Some("-1") {
                    -1.0
                } else {
                    1.0
                };
                Some(Marquee {
                    host,
                    track,
                    dir,
                    offset,
                })
            })
            .collect();

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
        hero.progress = -1.0;
    }

    fn on_scroll(&mut self, dt: f64) {
        let y = window().scroll_y().unwrap_or(0.0);
        let (_, vh) = viewport();
        let doc_h = document()
            .document_element()
            .map_or(vh, |e| f64::from(e.scroll_height()));
        let dy = y - self.last_y;
        self.sample = dy / dt.max(1.0);
        if dy.abs() > 0.5 {
            self.heading = dy.signum();
        }

        // Barre de navigation : fond dès qu'on quitte le haut de page, et
        // effacement quand on descend (elle revient dès qu'on remonte).
        let scrolled = y > 8.0;
        if scrolled != self.scrolled {
            self.scrolled = scrolled;
            set_root_class("scrolled", scrolled);
        }
        let hidden = if y < vh * 0.5 {
            false
        } else if dy > 3.0 {
            true
        } else if dy < -3.0 {
            false
        } else {
            self.nav_hidden
        };
        if hidden != self.nav_hidden {
            self.nav_hidden = hidden;
            set_root_class("nav-hidden", hidden);
        }
        self.last_y = y;

        self.track_section(vh);

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

        for t in &mut self.tracked {
            let (_, top, _, height) = rect(&t.el);
            let p = match t.span {
                Span::Through => (vh - top) / (vh + height).max(1.0),
                Span::Enter => (vh - top) / height.min(vh).max(1.0),
                Span::Read => (vh * 0.85 - top) / (height + vh * 0.1).max(1.0),
            }
            .clamp(0.0, 1.0);
            if (p - t.last).abs() > 0.0005 {
                t.last = p;
                set_var(&t.el, "--p", p);
            }
        }

        for (el, speed) in &self.parallax {
            let (_, top, _, height) = rect(el);
            set_px(el, "--py", (top + height / 2.0 - vh / 2.0) * speed);
        }
    }

    /// Section courante : rang sur `<html>` (`--spy`), et lien du menu
    /// correspondant marqué `data-current`.
    fn track_section(&mut self, vh: f64) {
        let mut current = None;
        for (i, s) in self.sections.iter().enumerate() {
            if s.get_bounding_client_rect().top() <= vh * 0.45 {
                current = Some(i);
            }
        }
        let inside = match (self.sections.first(), self.sections.last()) {
            (Some(first), Some(last)) => {
                first.get_bounding_client_rect().top() < vh * 0.6
                    && last.get_bounding_client_rect().bottom() > vh * 0.4
            }
            _ => false,
        };
        if inside != self.spy_on {
            self.spy_on = inside;
            set_root_class("spy-on", inside);
        }
        if current == self.spy {
            return;
        }
        self.spy = current;
        if let (Some(root), Some(i)) = (root(), current) {
            let _ = root.style().set_property("--spy", &i.to_string());
        }
        let id = current
            .and_then(|i| self.sections.get(i))
            .map(|s| format!("#{}", s.id()));
        for link in &self.nav_links {
            let here = id.is_some() && link.get_attribute("href") == id;
            let _ = if here {
                link.set_attribute("data-current", "")
            } else {
                link.remove_attribute("data-current")
            };
        }
    }

    /// Lisse la vitesse de défilement. Renvoie `true` tant qu'elle n'est pas
    /// retombée à zéro.
    fn ease_velocity(&mut self, scrolled_now: bool) -> bool {
        let target = if scrolled_now { self.sample } else { 0.0 };
        self.velocity += (target - self.velocity) * 0.14;
        if self.velocity.abs() < 0.002 {
            self.velocity = 0.0;
        }
        if let Some(root) = root() {
            set_var(&root, "--velocity", (self.velocity / 2.5).clamp(-1.0, 1.0));
        }
        self.velocity != 0.0
    }

    /// Bandeaux : vitesse de base, multipliée par la vitesse de défilement,
    /// et sens inversé quand on remonte la page.
    fn run_marquees(&mut self, dt: f64) -> bool {
        let (_, vh) = viewport();
        // Bornée : un saut de défilement (lien d'ancre, touche Fin) ne doit
        // pas emballer le bandeau. Au plus cinq fois la vitesse de base.
        let boost = 1.0 + (self.velocity.abs() / 2.5).min(1.0) * 4.0;
        let mut busy = false;
        for m in &mut self.marquees {
            let r = m.host.get_bounding_client_rect();
            if r.bottom() < 0.0 || r.top() > vh {
                continue;
            }
            busy = true;
            let width = f64::from(m.track.scroll_width()) / 2.0;
            if width <= 0.0 {
                continue;
            }
            m.offset = (m.offset + 0.065 * boost * m.dir * self.heading * dt).rem_euclid(width);
            let _ = m.track.style().set_property(
                "transform",
                &format!("translate3d({:.1}px, 0, 0)", -m.offset),
            );
        }
        busy
    }

    fn draw_hero(&mut self, now: f64) -> bool {
        let Some(hero) = self.hero.as_mut() else {
            return false;
        };
        if hero.dust.is_none() {
            return false;
        }

        let (_, top, _, height) = rect(&hero.pin);
        let (sx, sy, w, h) = rect(&hero.stage);
        // Hors de la vue : rien à dessiner, et surtout rien à animer.
        if top + height < 0.0 || sy > h {
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

        let (ax, ay, aw, _) = rect(&hero.art);
        if let Some(dust) = &hero.dust {
            dust.draw(&hero.ctx, (w, h), (ax - sx, ay - sy, aw), progress, now);
        }
        // Tant que des grains flottent, ils dérivent : il faut l'image suivante.
        !settled
    }

    /// Lance le déchiffrage du texte d'un élément, s'il n'est pas déjà en cours.
    fn scramble(&mut self, el: &Element, delay: f64) {
        if self.scrambles.iter().any(|j| j.is_on(el)) {
            return;
        }
        if let Some(job) = Job::new(el, delay) {
            self.scrambles.push(job);
        }
    }

    fn run_scrambles(&mut self, now: f64) -> bool {
        if self.scrambles.is_empty() {
            return false;
        }
        // Rien ne démarre sous le rideau d'ouverture : on attendrait un effet
        // que personne ne voit.
        let ready = document()
            .document_element()
            .is_some_and(|e| e.class_list().contains("ready"));
        if ready {
            self.scrambles.retain_mut(|j| j.step(now));
        }
        true
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
            let label = closest("[data-cursor]").and_then(|e| e.get_attribute("data-cursor"));
            if label != self.cursor_label {
                let _ = match &label {
                    Some(l) => c.set_attribute("data-label", l),
                    None => c.remove_attribute("data-label"),
                };
                self.cursor_label = label;
            }
        }

        // Texte déchiffré à nouveau au survol.
        let hovered = closest("[data-scramble]");
        let fresh = match (&hovered, &self.hovered) {
            (Some(a), Some(b)) => !a.is_same_node(Some(b)),
            (Some(_), None) => true,
            _ => false,
        };
        if fresh && let Some(el) = &hovered {
            self.scramble(el, 0.0);
        }
        self.hovered = hovered;

        // Projecteur des cartes : position du pointeur dans la carte.
        if let Some(card) = closest("[data-spot]").and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
            let (l, t, _, _) = rect(&card);
            set_px(&card, "--sx", x - l);
            set_px(&card, "--sy", y - t);
        }

        // Inclinaison 3D : la carte pivote vers le pointeur.
        let tilt = closest("[data-tilt]").and_then(|e| e.dyn_into::<HtmlElement>().ok());
        if let Some(el) = &tilt {
            let (l, t, w, h) = rect(el);
            let nx = ((x - l) / w.max(1.0) - 0.5).clamp(-0.5, 0.5);
            let ny = ((y - t) / h.max(1.0) - 0.5).clamp(-0.5, 0.5);
            let style = el.style();
            let _ = style.set_property("--ry", &format!("{:.2}deg", nx * 14.0));
            let _ = style.set_property("--rx", &format!("{:.2}deg", -ny * 12.0));
        }
        if !same(&self.tilt, &tilt) {
            self.release_tilt();
            self.tilt = tilt;
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
            set_px(m, "--mx", (x - cx) * 0.3);
            set_px(m, "--my", (y - cy) * 0.3);
        }
        if !same(&self.magnet, &magnet) {
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

    fn release_tilt(&mut self) {
        if let Some(m) = self.tilt.take() {
            let _ = m.style().remove_property("--rx");
            let _ = m.style().remove_property("--ry");
        }
    }

    /// L'anneau rattrape le pointeur, et la position lissée est publiée sur
    /// `<html>` pour les parallaxes. Renvoie `true` tant qu'il n'y est pas.
    fn follow_cursor(&mut self) -> bool {
        if !self.fine || !self.pointer.seen {
            return false;
        }
        let p = &mut self.pointer;
        p.rx += (p.x - p.rx) * 0.2;
        p.ry += (p.y - p.ry) * 0.2;
        if let Some(c) = &self.cursor {
            let _ = c.style().set_property(
                "transform",
                &format!("translate3d({:.1}px, {:.1}px, 0)", p.rx, p.ry),
            );
        }
        if let Some(root) = root() {
            let (vw, vh) = viewport();
            set_var(&root, "--pointer-x", (p.rx / vw.max(1.0)) * 2.0 - 1.0);
            set_var(&root, "--pointer-y", (p.ry / vh.max(1.0)) * 2.0 - 1.0);
        }
        (p.x - p.rx).abs() > 0.1 || (p.y - p.ry).abs() > 0.1
    }
}

fn same(a: &Option<HtmlElement>, b: &Option<HtmlElement>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.is_same_node(Some(b)),
        (None, None) => true,
        _ => false,
    }
}
