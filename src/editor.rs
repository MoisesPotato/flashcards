use leptos::{
    either::{EitherOf3, EitherOf5},
    prelude::*,
};
use web_sys::{HtmlDivElement, HtmlTextAreaElement};

use crate::{
    Data,
    data::serde::{LineOutput, PromptPiece, parse_prompt},
};

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
                text.with(|t| {
                    t.lines().map(|l| Oco::<'static, str>::Owned(l.to_owned())).collect::<Vec<_>>()
                })
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
        Ok(LineOutput::Comment) => EitherOf5::A(view! {
            <span class="code comment">{input}</span>
            "\n"
        }),
        Ok(LineOutput::Item(_)) => EitherOf5::B(view! {
            <span class="code item">{input}</span>
            "\n"
        }),
        Ok(LineOutput::Prompt { start, .. }) => EitherOf5::C(display_prompt(&input, start)),
        Ok(LineOutput::Parameter {
            key, value_bytes, ..
        }) => EitherOf5::D(display_param(&input, key, value_bytes)),
        Err(_) => EitherOf5::E(view! {
            <span class="code error">{input}</span>
            "\n"
        }),
    }
}

fn display_param(
    line: &str,
    key_span: (usize, usize),
    value_span: (usize, usize),
) -> impl IntoView + 'static {
    let before_key = line[..key_span.0].to_owned();
    let key = line[key_span.0..key_span.1].to_owned();
    let semicolon = line[key_span.1..value_span.0].to_owned();
    let value = line[value_span.0..value_span.1].to_owned();
    let end = line[value_span.1..].to_owned();
    view! {
        <span class="code parameter">
            {before_key} <span class="key">{key}</span> {semicolon}
            <span class="value">{value}</span> {end}"\n"
        </span>
    }
    .into_view()
}

fn display_prompt(line: &str, start: usize) -> impl IntoView + 'static {
    let prompt = &line[start..];
    let pieces = parse_prompt(prompt);
    let mut inside = vec![EitherOf3::A(line[0..start].to_owned())];

    for piece in pieces {
        match piece {
            PromptPiece::Normal(start, end) => inside.push(EitherOf3::B(
                view! { <span class="text">{prompt[start..end].to_string()}</span> },
            )),
            PromptPiece::Dollar => {
                inside.push(EitherOf3::C(view! { <span class="dollar">"$"</span> }));
            }
        }
    }

    view! {
        <span class="code prompt">{inside}</span>
        "\n"
    }
}
