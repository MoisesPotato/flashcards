#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::must_use_candidate,
    clippy::cast_precision_loss,
    clippy::empty_enums,
    clippy::suboptimal_flops
)]

use leptos::{either::Either, logging::error, prelude::*};

use Screen::{MainMenu, Playing};
use web_sys::console;

use crate::{
    game::Game,
    storage::{get_saved, save},
};

pub use data::{Data, Item};

mod data;
mod game;
mod storage;
mod strings;

#[derive(Clone, Copy)]
enum Screen {
    MainMenu,
    Playing,
}

#[component]
pub fn App() -> impl IntoView {
    let (screen, set_screen) = signal(Screen::MainMenu);

    let data = get_saved().map_or_else(
        |err| {
            error!("Failed to get data, error:");
            console::log_1(&err);
            Data::capitals()
        },
        |result| {
            if !result.errors.is_empty() {
                error!("Had these errores getting data:");
                for err in &result.errors {
                    error!("{err}");
                }
            }
            result.data
        },
    );

    let data = RwSignal::new(data);

    Effect::new(move |_| match data.with(save) {
        Ok(()) => {} //debug_log!("Saved data to local storage"),
        Err(err) => {
            error!("Error saving data");
            console::log_1(&err);
        }
    });

    move || match screen() {
        MainMenu => {
            Either::Left(view! { <button on:click=move |_| set_screen(Playing)>"Play"</button> })
        }
        Playing => Either::Right(view! { <Game data /> }),
    }
}
