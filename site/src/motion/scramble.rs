//! Texte qui se « déchiffre » : les caractères défilent en glyphes aléatoires
//! puis se fixent un à un, de gauche à droite — le thème même du site.
//!
//! Le texte appartient à Yew. On ne remplace donc jamais le nœud texte (Yew
//! garde une référence dessus et ne mettrait plus à jour qu'un nœud détaché) :
//! on réécrit seulement son contenu, et on le rend intact à la fin. Si Yew
//! change le texte en cours de route — changement de langue —, le contenu lu
//! ne correspond plus à ce qu'on a écrit : on adopte alors le nouveau texte
//! comme cible.

use wasm_bindgen::JsCast;
use web_sys::{Element, Text};

const GLYPHS: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'K', 'M', 'N', 'P', 'R', 'S', 'T', 'X', 'Z', '0', '1',
    '2', '3', '4', '5', '7', '8', '9', '#', '%', '&', '*', '+', '=', '<', '>', '/',
];

/// Glyphe pseudo-aléatoire.
pub fn glyph() -> char {
    GLYPHS[(js_sys::Math::random() * GLYPHS.len() as f64) as usize % GLYPHS.len()]
}

pub struct Job {
    el: Element,
    node: Text,
    target: String,
    /// Dernière valeur écrite : sert à détecter une réécriture par Yew.
    last: String,
    start: Option<f64>,
    delay: f64,
}

impl Job {
    /// Prépare l'animation du premier nœud texte non vide de `el`.
    pub fn new(el: &Element, delay: f64) -> Option<Self> {
        let children = el.child_nodes();
        let node = (0..children.length())
            .filter_map(|i| children.get(i))
            .filter_map(|n| n.dyn_into::<Text>().ok())
            .find(|t| !t.data().trim().is_empty())?;
        let target = node.data();
        Some(Self {
            el: el.clone(),
            node,
            last: target.clone(),
            target,
            start: None,
            delay,
        })
    }

    pub fn is_on(&self, el: &Element) -> bool {
        self.el.is_same_node(Some(el))
    }

    /// Avance d'une image. Renvoie `false` une fois le texte rétabli.
    pub fn step(&mut self, now: f64) -> bool {
        let current = self.node.data();
        if current != self.last {
            self.target = current;
        }
        let start = *self.start.get_or_insert(now + self.delay);
        let elapsed = now - start;
        if elapsed < 0.0 {
            self.last = self.node.data();
            return true;
        }
        let chars: Vec<char> = self.target.chars().collect();
        let n = chars.len().max(1) as f64;
        let duration = (380.0 + n * 32.0).min(1100.0);
        if elapsed >= duration {
            self.node.set_data(&self.target);
            self.last = self.target.clone();
            return false;
        }
        let out: String = chars
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let fixed_at = duration * (0.25 + 0.75 * i as f64 / n);
                if elapsed >= fixed_at || !c.is_alphanumeric() {
                    c
                } else {
                    glyph()
                }
            })
            .collect();
        self.node.set_data(&out);
        self.last = out;
        true
    }
}
