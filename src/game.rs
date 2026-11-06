use leptos::{ev::SubmitEvent, html, prelude::*};

use crate::data::{
    Data, Item,
    Outcome::{Almost, Correct, Wrong},
};

#[derive(Debug, Clone)]
enum AnswerStatus {
    Correct,
    Almost,
    Wrong { correct: String },
    None,
}

#[component]
pub fn Game(data: RwSignal<Data>) -> impl IntoView {
    let question = RwSignal::new(data.with_untracked(super::data::Data::pick_one));

    let (answer, set_answer) = signal(AnswerStatus::None);

    move || match answer() {
        AnswerStatus::None => view! { {prompt(set_answer, question)} },
        x => todo!("{x:?}"),
    }
}

fn prompt(
    set_answer: WriteSignal<AnswerStatus>,
    question: RwSignal<(Item, String)>,
) -> impl IntoView {
    let input_element: NodeRef<html::Input> = NodeRef::new();

    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        // here, we'll extract the value from the input
        let answer = input_element
            .get()
            .expect("<input> should be mounted")
            .value();

        match question.with(|(item, _)| item.check(&answer)) {
            Correct => log::error!("Todo correct"),
            Almost => log::error!("Todo almost"),
            Wrong => log::error!("Todo wrong"),
        }
    };

    view! {
        <div class="card">
            <h1>{move || question.with(|q| q.0.prompt_string(&q.1))}</h1>
            <form on:submit=on_submit>
                <input node_ref=input_element type="text" />
                <input type="submit" on:submit=on_submit />
            </form>
        </div>
    }
}
