use leptos::{
    ev::SubmitEvent,
    html,
    logging::{debug_log, error},
    prelude::*,
};
use web_sys::console;

use crate::data::{
    Data,
    Outcome::{Almost, Correct, Wrong},
};

#[derive(Debug, Clone)]
enum AnswerStatus {
    Correct,
    Wrong,
    None,
}

#[component]
pub fn Game(data: RwSignal<Data>) -> impl IntoView {
    let question = RwSignal::new(data.with_untracked(super::data::Data::pick_one));
    let prev_question = RwSignal::new(None);

    let (status, set_status) = signal(AnswerStatus::None);

    let streak = RwSignal::new(0);

    let prev_and_current = move || [Some(question()), prev_question()].into_iter().flatten();

    let (given_answer, set_answer) = signal(Option::<String>::None);

    let (almost, set_almost) = signal(false);

    Effect::new(move |_| {
        if let Some(answer) = given_answer() {
            debug_log!("Got an answer: {answer}");
            let outcome = data.with_untracked(move |data| {
                let item = data.get_item(question.get_untracked());
                item.check(&answer)
            });
            if matches!(outcome, Almost) {
                set_almost(true);
                return;
            }

            data.update(move |data| {
                let item = data.get_mut_item(question.get_untracked());
                match outcome {
                    Correct => {
                        item.streak += 1;
                        streak.update(|s| *s += 1);
                        set_status(AnswerStatus::Correct);
                    }
                    Almost => unreachable!("Already checked"),
                    Wrong => {
                        item.streak = 0;
                        streak.set(0);
                        set_status(AnswerStatus::Wrong);
                    }
                }
                // Reset
                item.attempts += 1;
                item.last_attempt = 0;
                data.step_all();
                set_answer(None);
                // New question
                prev_question.set(Some(question()));
                question.set(data.pick_one());
            });
        } else {
            debug_log!("Got no answer");
        }
    });

    view! {
        // {preanswer}
        <ForEnumerate
            each=prev_and_current
            key=|i| *i
            //
            children={move |index: ReadSignal<usize>, id: usize| {
                debug_log!("Rendering question {id} with index {}", index());
                prompt(set_answer, id, data, index, almost)
            }}
        />
    }
}

fn prompt(
    set_answer: WriteSignal<Option<String>>,
    question: usize,
    data: RwSignal<Data>,
    index: ReadSignal<usize>,
    almost: ReadSignal<bool>,
) -> impl IntoView {
    let old = move || index() == 1;

    let input_element: NodeRef<html::Input> = NodeRef::new();

    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        // here, we'll extract the value from the input
        let element = input_element.get().expect("<input> should be mounted");

        let answer = element.value();

        element.set_value("");

        set_answer(Some(answer));
    };

    let prompt = move || data.with(move |d| d.get_prompt(question));

    Effect::new(move |_| {
        if let Some(node) = input_element.get()
            && let Err(err) = node.focus()
        {
            error!("Error focusing element:");
            console::log_1(&err);
        }
    });
    view! {
        <div class="card" class:old=old>
            <h1>{prompt}</h1>
            {move || {
                if old() {
                    None
                } else {
                    Some(
                        view! {
                            <form on:submit=on_submit>
                                <input node_ref=input_element type="text" />
                                <input type="submit" value="Enter" on:submit=on_submit />
                            </form>
                            {move || if almost() { Some(view! { <p>"Almost!"</p> }) } else { None }}
                        },
                    )
                }
            }}
        </div>
    }
}
