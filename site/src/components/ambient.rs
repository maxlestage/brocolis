use yew::prelude::*;

/// Calques d'ambiance, communs aux deux pages : barre de progression du
/// défilement, grain de pellicule, curseur anneau. Purement décoratifs, donc
/// cachés aux technologies d'assistance, et sans effet sur les clics
/// (`pointer-events: none`). Le mouvement vient de `motion` et de la CSS.
#[function_component]
pub fn Ambient() -> Html {
    html! {
        <>
            <div class="progress" aria-hidden="true" />
            <div class="grain" aria-hidden="true" />
            <div class="cursor" aria-hidden="true"><span /></div>
        </>
    }
}
