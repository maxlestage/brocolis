//! Insertion de balisage littéral venu du dictionnaire.

use yew::{AttrValue, Html, html};

/// Rend une chaîne du dictionnaire en tant que HTML.
///
/// Équivalent du `dangerouslySetInnerHTML` de l'ancienne version React : une
/// bonne partie des traductions contiennent `<code>`, `<strong>` ou `<em>` au
/// milieu de la phrase, et les découper en fragments rendrait les fichiers de
/// traduction illisibles pour un traducteur.
///
/// Sûr ici parce que l'argument est un `&'static str` : seuls les littéraux
/// compilés dans le binaire peuvent l'atteindre. Le site n'a par ailleurs
/// aucune entrée utilisateur, aucun paramètre d'URL lu, aucune donnée distante.
pub fn raw(markup: &'static str) -> Html {
    Html::from_html_unchecked(AttrValue::from(markup))
}

/// Titre découpé en mots, chacun dans son masque : ils montent l'un après
/// l'autre quand le titre apparaît (`data-reveal="words"`). Le contenu
/// textuel reste identique au texte d'origine — mêmes mots, mêmes espaces —
/// pour les lecteurs d'écran comme pour la recherche dans la page.
pub fn words(text: &str) -> Html {
    let parts: Vec<&str> = text.split(' ').collect();
    let last = parts.len().saturating_sub(1);
    parts
        .into_iter()
        .enumerate()
        .map(|(i, w)| {
            html! {
                <>
                    <span class="w"><span style={format!("--i: {i}")}>{ w.to_owned() }</span></span>
                    if i < last { { " " } }
                </>
            }
        })
        .collect()
}

/// Libellé de lien ou de bouton qui « roule » au survol : une copie du texte,
/// dessinée en CSS à partir de `data-text`, remonte prendre sa place. La copie
/// n'existe pas dans le DOM : rien n'est lu deux fois.
pub fn rolling(text: &str) -> Html {
    html! {
        <span class="roll"><span data-text={text.to_owned()}>{ text.to_owned() }</span></span>
    }
}

/// Décalages verticaux des lettres d'un mot géant quand il se disperse : une
/// suite fixe plutôt qu'un tirage, pour que le dessin soit le même à chaque
/// visite.
pub const SCATTER: [f64; 8] = [0.7, 1.6, 1.0, 2.1, 0.5, 1.4, 2.4, 1.1];

/// Mot géant découpé en lettres, chacune portant son rang (`--i`) et son
/// amplitude de dispersion (`--d`).
pub fn letters(word: &str) -> Html {
    word.chars()
        .enumerate()
        .map(|(i, c)| {
            let d = SCATTER[i % SCATTER.len()];
            html! { <span class="char" style={format!("--i: {i}; --d: {d}")}>{ c }</span> }
        })
        .collect()
}
