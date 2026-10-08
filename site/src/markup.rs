//! Insertion de balisage littéral venu du dictionnaire.

use yew::{AttrValue, Html};

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
