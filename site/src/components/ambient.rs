use active::Node;
use active::html::*;

/// Calques d'ambiance, communs aux deux pages : barre de progression du
/// défilement, grain de pellicule, curseur anneau. Purement décoratifs, donc
/// cachés aux technologies d'assistance, et sans effet sur les clics
/// (`pointer-events: none`). Le mouvement vient de `motion` et de la CSS.
pub fn ambient() -> Node {
    Node::from([
        div().class("progress").attr("aria-hidden", "true"),
        div().class("grain").attr("aria-hidden", "true"),
        div()
            .class("cursor")
            .attr("aria-hidden", "true")
            .child(span()),
    ])
}
