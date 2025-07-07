/*
 * SPDX-FileCopyrightText: 2025 SyoBoN <syobon@syobon.net>
 *
 * SPDX-License-Identifier: UPL-1.0
 */

use std::sync::{LazyLock, Mutex};

use markov_chain_notestock::markov::{MarkovBuilder, MarkovGenerator};
use wasm_bindgen::prelude::wasm_bindgen;

static MODEL: LazyLock<Mutex<MarkovGenerator>> =
    LazyLock::new(|| Mutex::new(MarkovGenerator::new()));

#[wasm_bindgen]
pub struct MarkovWasm(MarkovBuilder);

#[wasm_bindgen]
impl MarkovWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(MarkovBuilder::new().unwrap())
    }

    pub fn learn(&mut self, tar_zip: &[u8]) -> bool {
        let Ok(data) = markov_chain_notestock::notestock::parse(tar_zip) else {
            return false;
        };
        self.0.learn_many(&data);
        true
    }

    pub fn build(self) {
        MODEL.lock().unwrap().set_model(self.0.build());
    }
}

#[wasm_bindgen]
pub fn set_starting_word(starting_word: String) -> bool {
    let token = if starting_word.is_empty() {
        None
    } else {
        Some(starting_word)
    };
    if MODEL.lock().unwrap().set_start(token).is_err() {
        return false;
    }
    true
}

#[wasm_bindgen]
pub fn generate() -> Vec<String> {
    MODEL.lock().unwrap().generate().unwrap()
}

#[wasm_bindgen]
pub fn download() -> Vec<u8> {
    let Some(model) = MODEL.lock().unwrap().get_data() else {
        return Vec::new();
    };
    markov_chain_notestock::markov::encode_model(model).unwrap_or_default()
}

#[wasm_bindgen]
pub fn upload(data: &[u8]) -> bool {
    let Ok(model) = MarkovGenerator::from_bincode(data) else {
        return false;
    };
    *MODEL.lock().unwrap() = model;
    true
}
