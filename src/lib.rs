#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::must_use_candidate)]

use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {<p>"Ola"</p>}
}
