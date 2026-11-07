use leptos::{either::EitherOf4, ev::SubmitEvent, html, prelude::*};

use crate::data::{
    Data,
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
    let prev_question = RwSignal::new(None);

    let (status, set_status) = signal(AnswerStatus::None);

    let preanswer = move || match status() {
        AnswerStatus::None => EitherOf4::A(()),
        AnswerStatus::Almost => EitherOf4::B(view! { <p>"Almost! Try again"</p> }),
        AnswerStatus::Correct => EitherOf4::C(view! { <p>"Correct!"</p> }),
        AnswerStatus::Wrong { correct } => {
            EitherOf4::D(view! { <p>"Wrong! The correct answer was " {correct}</p> })
        }
    };

    let streak = RwSignal::new(0);

    let prev_and_current = move || [Some(question()), prev_question()].into_iter().flatten();

    view! {
        // {preanswer}
        <ForEnumerate
            each=prev_and_current
            key=|i| *i
            children={//
                move |index: ReadSignal<usize>, id: usize| {
                prompt(set_status, id, data, streak, index)
            }}
        />
    }
}

fn prompt(
    set_answer: WriteSignal<AnswerStatus>,
    question: usize,
    data: RwSignal<Data>,
    streak: RwSignal<usize>,
    index: ReadSignal<usize>,
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

        data.update(move |data| {
            let item = data.get_mut_item(question);
            match item.check(&answer) {
                Correct => {
                    item.streak += 1;
                    streak.update(|s| *s += 1);
                    set_answer(AnswerStatus::Correct);
                }
                Almost => {
                    set_answer(AnswerStatus::Almost);
                    // Try again
                    return;
                }
                Wrong => {
                    item.streak = 0;
                    streak.set(0);
                    set_answer(AnswerStatus::Wrong {
                        correct: item.answers.clone(),
                    });
                }
            }
            // Reset
            item.attempts += 1;
            item.last_attempt = 0;
            data.step_all();
            // question.set(data.pick_one());
        });
    };

    let prompt = move || data.with(move |d| d.get_prompt(question));

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
                        },
                    )
                }
            }}
        </div>
    }
}
