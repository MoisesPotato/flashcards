use leptos::{either::EitherOf3, ev::SubmitEvent, html, logging::error, prelude::*};
use web_sys::{HtmlElement, console};

use crate::data::{
    Data,
    Outcome::{Almost, Correct, Wrong},
};

#[derive(Debug, Clone)]
enum AnswerStatus {
    Correct(usize),
    Wrong(usize),
    None,
}

#[component]
pub fn Game(data: RwSignal<Data>) -> impl IntoView {
    let question = RwSignal::new(data.with_untracked(super::data::Data::pick_one));
    let prev_question = RwSignal::new(AnswerStatus::None);
    let prev_prev_question = RwSignal::new(AnswerStatus::None);

    // Probably unnecessary
    // Effect::new(move |_| {
    //     if !matches!(prev_prev_question.get(), AnswerStatus::None) {
    //         set_timeout(
    //             move || {
    //                 prev_prev_question.set(AnswerStatus::None);
    //             },
    //             std::time::Duration::from_millis(1_000),
    //         );
    //     }
    // });

    let streak = RwSignal::new(0);

    let prev_and_current = move || {
        [
            Some(question()),
            match prev_question.get() {
                AnswerStatus::Correct(i) | AnswerStatus::Wrong(i) => Some(i),
                AnswerStatus::None => None,
            },
            match prev_prev_question.get() {
                AnswerStatus::Correct(i) | AnswerStatus::Wrong(i) => Some(i),
                AnswerStatus::None => None,
            },
        ]
        .into_iter()
        .flatten()
    };

    let (given_answer, set_answer) = signal(Option::<String>::None);

    let (almost, set_almost) = signal(false);

    Effect::new(move |_| {
        if let Some(answer) = given_answer() {
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
                        prev_prev_question.set(prev_question.get());
                        prev_question.set(AnswerStatus::Correct(question()));
                    }
                    Almost => unreachable!("Already checked"),
                    Wrong => {
                        item.streak = 0;
                        streak.set(0);
                        prev_prev_question.set(prev_question.get());
                        prev_question.set(AnswerStatus::Wrong(question()));
                    }
                }
                // Reset
                item.attempts += 1;
                item.last_attempt = 0;
                data.step_all();

                set_answer(None);
                set_almost(false);
                // New question
                question.set(data.pick_one());
            });
        }
    });

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

    Effect::new(move |_| {
        if let Some(node) = input_element.get()
            && let Err(err) = node.focus()
        {
            error!("Error focusing element:");
            console::log_1(&err);
        }
    });

    view! {
        // {preanswer}
        <p class="streak">"Streak: "{streak}</p>
        <ForEnumerate
            each=prev_and_current
            key=|i| *i
            children={move |index: ReadSignal<usize>, id: usize| {
                card(id, data, index, prev_question)
            }}
        />
        <form on:submit=on_submit class="answer-box">
            <input
                node_ref=input_element
                type="text"
                placeholder={move || if almost() { "Almost!" } else { "" }}
            />
            <input type="submit" value="Enter" on:submit=on_submit />
        </form>
    }
}

fn card(
    question: usize,
    data: RwSignal<Data>,
    index: ReadSignal<usize>,
    prev_question: RwSignal<AnswerStatus>,
) -> impl IntoView {
    let old = move || index() > 0;

    let correct = move || old() && matches!(prev_question.get(), AnswerStatus::Correct(_));
    let wrong = move || old() && matches!(prev_question.get(), AnswerStatus::Wrong(_));

    let prompt = move || data.with(move |d| d.get_prompt(question));

    let question_elt: NodeRef<html::H1> = NodeRef::new();

    Effect::new(move |_| {
        if let Some(node) = question_elt.get() {
            //  if (!el.parentElement) return;
            if let Err(err) = HtmlElement::style(&node).set_property("--font-size", "1em") {
                error!("Error setting font size:");
                console::log_1(&err);
                return;
            }

            let rect = node.get_bounding_client_rect();

            let width = rect.width();

            let target_width = 600.;

            let font_size = (target_width / width).clamp(0., 2.);

            if let Err(err) =
                HtmlElement::style(&node).set_property("--font-size", &format!("{font_size}em"))
            {
                error!("Error setting font size:");
                console::log_1(&err);
            }
        }
    });

    let summary = move || {
        if old() {
            let item = data.with(move |d| d.get_item(question).clone());
            let correct = item.attempts - item.errors;
            let total = item.attempts;
            let streak = item.streak;
            Some(
                view! { <div class="summary">"Overall: "{correct}"/"{total}", Streak "{streak}</div> },
            )
        } else {
            None
        }
    };

    view! {
        <div
            class="card"
            class:old=old
            class:new=move || !old()
            class:correct=correct
            class:wrong=wrong
            class:leaving={move || index() > 1}
        >
            {move || {
                if correct() {
                    EitherOf3::A(view! { <p class="correct_wrong">"Correct!"</p> })
                } else if wrong() {
                    let correct_answer = data.with(move |d| d.get_item(question).answers.clone());
                    EitherOf3::B(
                        view! {
                            <p class="correct_wrong">
                                "Wrong! The correct answer was "{correct_answer}
                            </p>
                        },
                    )
                } else {
                    EitherOf3::C(())
                }
            }}

            <h1 node_ref=question_elt>{prompt}</h1>
            {summary}
        </div>
    }
}
