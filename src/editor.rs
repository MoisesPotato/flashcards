use leptos::prelude::*;
use web_sys::{HtmlDivElement, HtmlTextAreaElement};

use crate::Data;

#[component]
pub fn editor(data: RwSignal<Data>) -> impl IntoView {
    let text = RwSignal::new(data.with_untracked(Data::serialize));

    let node_div = NodeRef::new();
    let node_area = NodeRef::new();

    let on_scroll = move |_ev| {
        let div: HtmlDivElement = node_div.get().expect("Should be mounted");
        let area: HtmlTextAreaElement = node_area.get().expect("Should be mounted");

        div.set_scroll_top(area.scroll_top());
        div.set_scroll_left(area.scroll_left());
    };

    view! {
    <div id="highlighted" node_ref=node_div>{text}</div>
    <textarea bind:value=text id="q-input" node_ref=node_area on:scroll=on_scroll></textarea> }
}
