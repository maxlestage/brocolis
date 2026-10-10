//! Petites briques de vue partagées : textes traduits, balisage contenu dans
//! les traductions, titres découpés en mots ou en lettres.
//!
//! Tout texte traduit est une liaison (`text_dyn`, `html_dyn`, `attr_dyn`) :
//! la fermeture relit la langue courante, et un changement de langue réécrit
//! le texte dans le nœud existant. Le DOM, et ce que `motion` y a posé
//! (`data-in`, variables CSS, déchiffrage en cours), reste en place.

use active::html::span;
use active::{Element, Node, fragment_dyn, text_dyn};

use crate::i18n::Strings;
use crate::settings::t;

/// Choisit une chaîne dans les traductions.
pub trait Pick: Fn(&'static Strings) -> &'static str + 'static {}
impl<F: Fn(&'static Strings) -> &'static str + 'static> Pick for F {}

/// Texte traduit, tenu à jour.
pub fn tr(pick: impl Pick) -> Node {
    text_dyn(move || pick(t()).to_owned())
}

/// Liaisons traduites sur un élément.
pub trait Translated {
    /// Ajoute un texte traduit.
    fn tr(self, pick: impl Pick) -> Self;
    /// Un attribut traduit.
    fn attr_tr(self, name: &'static str, pick: impl Pick) -> Self;
    /// Remplace le contenu par une chaîne du dictionnaire rendue en HTML.
    ///
    /// Une bonne partie des traductions contiennent `<code>`, `<strong>` ou
    /// `<em>` au milieu de la phrase, et les découper en fragments rendrait
    /// les fichiers de traduction illisibles pour un traducteur. Sûr ici parce
    /// que la chaîne est un `&'static str` : seuls les littéraux compilés dans
    /// le binaire peuvent l'atteindre. Le site n'a par ailleurs aucune entrée
    /// utilisateur, aucun paramètre d'URL lu, aucune donnée distante.
    fn raw(self, pick: impl Pick) -> Self;
}

impl Translated for Element {
    fn tr(self, pick: impl Pick) -> Self {
        self.child(tr(pick))
    }

    fn attr_tr(self, name: &'static str, pick: impl Pick) -> Self {
        self.attr_dyn(name, move || pick(t()).to_owned())
    }

    fn raw(self, pick: impl Pick) -> Self {
        self.html_dyn(move || pick(t()).to_owned())
    }
}

/// Titre découpé en mots, chacun dans son masque : ils montent l'un après
/// l'autre quand le titre apparaît (`data-reveal="words"`). Le contenu
/// textuel reste identique au texte d'origine — mêmes mots, mêmes espaces —
/// pour les lecteurs d'écran comme pour la recherche dans la page.
///
/// Le nombre de mots dépend de la langue : les mots sont reconstruits quand
/// elle change, le titre qui les porte reste.
pub fn words(pick: impl Pick) -> Node {
    fragment_dyn(move || {
        let parts: Vec<&str> = pick(t()).split(' ').collect();
        let last = parts.len().saturating_sub(1);
        let mut nodes = Vec::with_capacity(parts.len() * 2);
        for (i, w) in parts.into_iter().enumerate() {
            nodes.push(
                span()
                    .class("w")
                    .child(span().style(format!("--i: {i}")).text(w))
                    .into(),
            );
            if i < last {
                nodes.push(" ".into());
            }
        }
        nodes
    })
}

/// Libellé de lien ou de bouton qui « roule » au survol : une copie du texte,
/// dessinée en CSS à partir de `data-text`, remonte prendre sa place. La copie
/// n'existe pas dans le DOM : rien n'est lu deux fois.
pub fn rolling(pick: impl Pick + Copy) -> Node {
    span()
        .class("roll")
        .child(span().attr_tr("data-text", pick).tr(pick))
        .into()
}

/// Décalages verticaux des lettres d'un mot géant quand il se disperse : une
/// suite fixe plutôt qu'un tirage, pour que le dessin soit le même à chaque
/// visite.
pub const SCATTER: [f64; 8] = [0.7, 1.6, 1.0, 2.1, 0.5, 1.4, 2.4, 1.1];

/// Mot géant découpé en lettres, chacune portant son rang (`--i`) et son
/// amplitude de dispersion (`--d`).
pub fn letters(word: &str) -> Vec<Node> {
    word.chars()
        .enumerate()
        .map(|(i, c)| {
            let d = SCATTER[i % SCATTER.len()];
            span()
                .class("char")
                .style(format!("--i: {i}; --d: {d}"))
                .text(c.to_string())
                .into()
        })
        .collect()
}

/// Garde `value` en vie aussi longtemps que la vue en cours de construction
/// (ou de montage) : elle est libérée — et son `Drop` exécuté — quand la
/// région dynamique qui la contient est reconstruite, par exemple au
/// changement de page. Hors de toute région, elle vit autant que la page.
///
/// Un état `active` appartient à la vue qui l'a créé et meurt avec elle :
/// c'est ce qui tient lieu de fonction de nettoyage.
pub fn keep<T: 'static>(value: T) {
    let _ = active::use_state(value);
}

#[cfg(test)]
mod tests {
    use super::letters;

    #[test]
    fn un_mot_geant_garde_son_texte() {
        let html: String = letters("Brocolis").iter().map(|n| n.render()).collect();
        assert!(html.starts_with(r#"<span class="char" style="--i: 0; --d: 0.7">B</span>"#));
        let text: String = html
            .split('>')
            .filter_map(|s| s.split('<').next())
            .collect();
        assert_eq!(text, "Brocolis");
    }
}
