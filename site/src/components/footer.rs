use yew::prelude::*;

use crate::settings::use_t;

#[function_component]
pub fn Footer() -> Html {
    let t = use_t();

    html! {
        <footer>
            <div class="container">
                <p>{ t.footer }</p>
            </div>
        </footer>
    }
}
