use yew::prelude::*;

use crate::i18n::LANGS;
use crate::settings::use_settings;
use crate::theme::THEMES;

/// Les deux sélecteurs (langue, thème), présents dans les deux barres.
///
/// Chaque groupe est un `role="group"` nommé, et chaque bouton porte
/// `aria-pressed` : un lecteur d'écran annonce donc « Langue, groupe » puis
/// « FR, bouton, enfoncé ». Le thème a en plus un libellé textuel accessible,
/// parce qu'un pictogramme seul (🥦 ☀ 🌙) ne s'annonce pas.
#[function_component]
pub fn Switchers() -> Html {
    let settings = use_settings();
    let t = settings.lang.strings();

    let langs = LANGS.iter().map(|&l| {
        let on_pick = {
            let set_lang = settings.set_lang.clone();
            Callback::from(move |_: MouseEvent| set_lang.emit(l))
        };
        let active = settings.lang == l;
        html! {
            <button
                type="button"
                class={classes!("pill", active.then_some("active"))}
                aria-pressed={active.to_string()}
                onclick={on_pick}
            >
                { l.label() }
            </button>
        }
    });

    let themes = THEMES.iter().map(|&th| {
        let on_pick = {
            let set_theme = settings.set_theme.clone();
            Callback::from(move |_: MouseEvent| set_theme.emit(th))
        };
        let active = settings.theme == th;
        let label = t.theme_label(th);
        html! {
            <button
                type="button"
                class={classes!("pill", active.then_some("active"))}
                aria-pressed={active.to_string()}
                title={label}
                onclick={on_pick}
            >
                <span aria-hidden="true">{ th.icon() }</span>
                <span class="sr-only">{ label }</span>
            </button>
        }
    });

    html! {
        <div class="switchers">
            <div class="switcher" role="group" aria-label={t.switch_lang}>
                { for langs }
            </div>
            <div class="switcher" role="group" aria-label={t.switch_theme}>
                { for themes }
            </div>
        </div>
    }
}
