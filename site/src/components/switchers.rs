use active::Node;
use active::html::*;

use crate::i18n::LANGS;
use crate::markup::Translated;
use crate::settings::{lang, set_lang, set_theme, theme};
use crate::theme::THEMES;

/// Les deux sélecteurs (langue, thème), présents dans les deux barres.
///
/// Chaque groupe est un `role="group"` nommé, et chaque bouton porte
/// `aria-pressed` : un lecteur d'écran annonce donc « Langue, groupe » puis
/// « FR, bouton, enfoncé ». Le thème a en plus un libellé textuel accessible,
/// parce qu'un pictogramme seul (🥦 ☀ 🌙) ne s'annonce pas.
pub fn switchers() -> Node {
    let langs = LANGS.map(|l| {
        let active = move || lang().get() == l;
        button()
            .attr("type", "button")
            .class("pill")
            .class_if("active", active)
            .attr_dyn("aria-pressed", move || active().to_string())
            .on_click(move |_| set_lang(l))
            .text(l.label())
    });

    let themes = THEMES.map(|th| {
        let active = move || theme().get() == th;
        let label = move |t: &'static crate::i18n::Strings| t.theme_label(th);
        button()
            .attr("type", "button")
            .class("pill")
            .class_if("active", active)
            .attr_dyn("aria-pressed", move || active().to_string())
            .attr_tr("title", label)
            .on_click(move |_| set_theme(th))
            .child(span().attr("aria-hidden", "true").text(th.icon()))
            .child(span().class("sr-only").tr(label))
    });

    div()
        .class("switchers")
        .child(
            div()
                .class("switcher")
                .attr("role", "group")
                .attr_tr("aria-label", |t| t.switch_lang)
                .children(langs),
        )
        .child(
            div()
                .class("switcher")
                .attr("role", "group")
                .attr_tr("aria-label", |t| t.switch_theme)
                .children(themes),
        )
        .into()
}
