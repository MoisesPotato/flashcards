use leptos::leptos_dom::helpers::window;
use wasm_bindgen::prelude::*;

use crate::data::Data;

pub struct SaveResult {
    pub data: Data,
    pub errors: Vec<String>,
}

pub fn get_saved() -> Result<SaveResult, JsValue> {
    let storage = window()
        .local_storage()?
        .ok_or("No storage in this browser")?;

    let Some(string) = storage.get("questions")? else {
        return Ok(SaveResult {
            data: Data::default(),
            errors: vec!["Nothing is saved".into()],
        });
    };

    let data = Data::deserialize(&string).expect("TODO");

    Ok(SaveResult {
        data,
        errors: vec![],
    })
}

pub fn save(data: &Data) -> Result<(), JsValue> {
    let storage = window()
        .local_storage()?
        .ok_or("No storage in this browser")?;

    let string = data.serialize();

    storage.set("questions", &string)
}
