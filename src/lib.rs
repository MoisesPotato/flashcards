#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::must_use_candidate,
    clippy::cast_precision_loss,
    clippy::empty_enums
)]

use leptos::{either::Either, prelude::*};

use Screen::{MainMenu, Playing};

use crate::{data::Data, game::Game};

mod data;
mod game;

#[derive(Clone, Copy)]
enum Screen {
    MainMenu,
    Playing,
}

#[component]
pub fn App() -> impl IntoView {
    let (screen, set_screen) = signal(Screen::MainMenu);

    let data = RwSignal::new(Data::default());

    move || match screen() {
        MainMenu => {
            Either::Left(view! { <button on:click=move |_| set_screen(Playing)>"Play"</button> })
        }
        Playing => Either::Right(view! { <Game data /> }),
    }
}
