use leptos::prelude::*;

use crate::data::{Data, Item};

#[component]
pub fn Game(data: RwSignal<Data>) -> impl IntoView {
    let question = RwSignal::new(data.with_untracked(super::data::Data::pick_one));

    view! { {prompt(data, question)} }
}

fn prompt(data: RwSignal<Data>, question: RwSignal<(Item, String)>) -> impl IntoView {
    view! { <h1>{move || question.with(|q| q.0.prompt_string(&q.1))}</h1> }
}
