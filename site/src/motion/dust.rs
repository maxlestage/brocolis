//! La mascotte qui se forme dans la poussière.
//!
//! L'image du héros est réduite à une grille de `GRID × GRID` cellules ; chaque
//! cellule du disque devient un grain de poussière qui porte la couleur de son
//! pixel. Au départ les grains flottent dans toute la scène ; à mesure que la
//! scène épinglée défile, ils rejoignent leur case et la mosaïque reconstitue
//! l'image — que la vraie photo vient ensuite recouvrir en fondu (CSS).
//!
//! Tout est piloté par la progression du défilement : revenir en arrière
//! redisperse la poussière. Il n'y a donc aucun état d'animation à garder, la
//! même progression donne toujours la même image (au dérivé près).

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

use crate::dom::document;

/// Résolution de la mosaïque. 56 cases de large donnent ~2 400 grains dans le
/// disque : assez pour qu'on reconnaisse la mascotte avant le fondu, assez peu
/// pour rester à quelques dixièmes de milliseconde par image.
const GRID: u32 = 56;

/// Fraction de la progression pendant laquelle un grain voyage. Chaque grain
/// part avec un retard propre (jusqu'à `1 - SPAN - MARGIN`), d'où l'effet de
/// nuée plutôt que de bloc.
const SPAN: f64 = 0.5;
const MARGIN: f64 = 0.12;

/// Avancement à partir duquel tous les grains sont à leur place.
pub const SETTLED: f64 = 1.0 - MARGIN;

struct Grain {
    /// Position cible dans l'image, en fraction de son côté.
    u: f64,
    v: f64,
    /// Position de départ dans la scène, en fraction de sa taille.
    sx: f64,
    sy: f64,
    phase: f64,
    drift: f64,
    delay: f64,
}

pub struct Dust {
    /// Grains triés par couleur : on ne change de `fillStyle` qu'une fois par
    /// teinte, pas une fois par grain.
    grains: Vec<Grain>,
    /// Couleur et indice de fin (exclu) de chaque groupe dans `grains`.
    groups: Vec<(String, usize)>,
}

/// Contexte 2D d'un canevas.
pub fn context_2d(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas.get_context("2d").ok()??.dyn_into().ok()
}

fn ease_in_out_cubic(x: f64) -> f64 {
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
    }
}

impl Dust {
    /// Échantillonne l'image. `None` si elle n'est pas encore décodée ou si le
    /// canevas est indisponible — l'appelant réessaiera au chargement.
    pub fn sample(img: &HtmlImageElement) -> Option<Self> {
        if !img.complete() || img.natural_width() == 0 {
            return None;
        }
        let canvas: HtmlCanvasElement =
            document().create_element("canvas").ok()?.dyn_into().ok()?;
        canvas.set_width(GRID);
        canvas.set_height(GRID);
        let ctx = context_2d(&canvas)?;
        // Réduire l'image à la grille fait la moyenne des pixels de chaque
        // case : c'est exactement la couleur qu'on veut donner au grain.
        ctx.draw_image_with_html_image_element_and_dw_and_dh(
            img,
            0.0,
            0.0,
            GRID as f64,
            GRID as f64,
        )
        .ok()?;
        let pixels = ctx
            .get_image_data(0.0, 0.0, GRID as f64, GRID as f64)
            .ok()?
            .data();

        let half = GRID as f64 / 2.0;
        let mut keyed: Vec<(u16, Grain)> = Vec::new();
        for gy in 0..GRID {
            for gx in 0..GRID {
                let (cx, cy) = (gx as f64 + 0.5 - half, gy as f64 + 0.5 - half);
                if cx * cx + cy * cy > half * half {
                    continue;
                }
                let i = ((gy * GRID + gx) * 4) as usize;
                // Quatre bits par canal : quelques centaines de teintes au
                // plus, à l'œil indiscernable une fois en mouvement.
                let key = (u16::from(pixels[i] >> 4) << 8)
                    | (u16::from(pixels[i + 1] >> 4) << 4)
                    | u16::from(pixels[i + 2] >> 4);
                let r = js_sys::Math::random;
                keyed.push((
                    key,
                    Grain {
                        u: gx as f64 / GRID as f64,
                        v: gy as f64 / GRID as f64,
                        sx: r() * 1.2 - 0.1,
                        sy: r() * 1.2 - 0.1,
                        phase: r() * std::f64::consts::TAU,
                        drift: 8.0 + r() * 28.0,
                        // Le centre arrive avant les bords : la forme se
                        // dessine du cœur vers l'extérieur.
                        delay: ((cx * cx + cy * cy).sqrt() / half) * 0.22
                            + r() * (1.0 - SPAN - MARGIN - 0.22),
                    },
                ));
            }
        }
        keyed.sort_by_key(|(k, _)| *k);

        let mut groups: Vec<(String, usize)> = Vec::new();
        let mut grains = Vec::with_capacity(keyed.len());
        let mut last = None;
        for (key, grain) in keyed {
            if last != Some(key) {
                if let Some(prev) = last {
                    groups.push((color(prev), grains.len()));
                }
                last = Some(key);
            }
            grains.push(grain);
        }
        if let Some(prev) = last {
            groups.push((color(prev), grains.len()));
        }
        Some(Self { grains, groups })
    }

    /// Dessine la poussière. `stage` est la taille de la scène, `art` le carré
    /// (x, y, côté) où l'image se reforme, en pixels CSS ; `progress` va de 0
    /// (dispersé) à 1 (formé) ; `now` anime la dérive des grains libres.
    pub fn draw(
        &self,
        ctx: &CanvasRenderingContext2d,
        stage: (f64, f64),
        art: (f64, f64, f64),
        progress: f64,
        now: f64,
    ) {
        let (w, h) = stage;
        let (ax, ay, side) = art;
        let cell = side / GRID as f64;
        ctx.clear_rect(0.0, 0.0, w, h);

        let mut start = 0;
        for (fill, end) in &self.groups {
            ctx.set_fill_style_str(fill);
            for g in &self.grains[start..*end] {
                let e = ease_in_out_cubic(((progress - g.delay) / SPAN).clamp(0.0, 1.0));
                let free = 1.0 - e;
                let dx = (now * 0.00042 + g.phase).sin() * g.drift * free;
                let dy = (now * 0.00031 + g.phase * 1.7).cos() * g.drift * free;
                let x = (g.sx * w + dx) * free + (ax + g.u * side) * e;
                let y = (g.sy * h + dy) * free + (ay + g.v * side) * e;
                // Un grain libre est un point ; posé, il couvre sa case (un
                // demi-pixel de plus pour que la mosaïque ne montre pas de
                // joints).
                let size = 1.6 * free + (cell + 0.5) * e;
                ctx.set_global_alpha(0.3 + 0.7 * e);
                ctx.fill_rect(x, y, size, size);
            }
            start = *end;
        }
        ctx.set_global_alpha(1.0);
    }
}

fn color(key: u16) -> String {
    // Milieu de l'intervalle quantifié : 0x0 → 8, 0xF → 248.
    let c = |n: u16| ((n & 0xF) * 16 + 8) as u8;
    format!("rgb({},{},{})", c(key >> 8), c(key >> 4), c(key))
}
