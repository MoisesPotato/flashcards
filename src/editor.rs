use leptos::{either::EitherOf5, prelude::*};
use web_sys::{HtmlDivElement, HtmlTextAreaElement};

use crate::{Data, data::serde::LineOutput};

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

    let errors = RwSignal::new(vec![]);

    let on_input = move |_ev| {
        data.set(text.with(|t| {
            let (data, new_errors) = Data::deserialize(t);
            errors.set(new_errors);
            data
        }));
    };

    let lines = view! {
        <For
            each=move || {
                text.with(|t| t.lines().map(|l| Oco::<'static, str>::Owned(l.to_owned())).collect::<Vec<_>>())
            }
            key={|x| x.clone()}
            children=child_fn
        />
    };

    view! {
        <div id="highlighted" node_ref=node_div>
            {lines}
        </div>
        <textarea
            bind:value=text
            id="q-input"
            node_ref=node_area
            on:scroll=on_scroll
            on:input=on_input
        ></textarea>
        {move || {
            errors
                .get()
                .iter()
                .map(move |error| {
                    view! { <p class="error-msg">{error.msg.clone()}</p> }
                })
                .collect::<Vec<_>>()
        }}
    }
}

#[allow(clippy::needless_pass_by_value)]
fn child_fn(input: Oco<'static, str>) -> impl IntoView {
    match LineOutput::deserialize(&input) {
        Ok(LineOutput::Comment) => {
            EitherOf5::A(view! {<span class="code comment">{input}</span>"\n"})
        }
        Ok(LineOutput::Item(_)) => EitherOf5::B(view! {<span class="code item">{input}</span>"\n"}),
        Ok(LineOutput::Prompt(_)) => {
            EitherOf5::C(view! {<span class="code prompt">{input}</span>"\n"})
        }
        Ok(LineOutput::Parameter(_)) => {
            EitherOf5::D(view! {<span class="code parameter">{input}</span>"\n"})
        }
        Err(_) => EitherOf5::E(view! {<span class="code error">{input}</span>"\n"}),
    }
}
