/*
 * SPDX-FileCopyrightText: 2025 SyoBoN <syobon@syobon.net>
 *
 * SPDX-License-Identifier: UPL-1.0
 */

use std::{collections::HashMap, fs::File, path::Path};

use anyhow::Context;
use bincode::{Decode, Encode};
use litsea::{adaboost::AdaBoost, segmenter::Segmenter};
use rand::seq::IndexedRandom;

const MODEL: &[u8] = include_bytes!("../assets/JEITA_Genpaku_ChaSen_IPAdic.model");

pub struct MarkovBuilder {
    segmenter: Segmenter,
    model: MarkovModel,
}

pub struct MarkovGenerator {
    current: Token,
    model: Option<MarkovModel>,
}

pub type MarkovModel = HashMap<Token, HashMap<Token, u32>>;

#[derive(Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub enum Token {
    Bos,
    Word(String),
    Eos,
}

impl Default for MarkovGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkovGenerator {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current: Token::Bos,
            model: None,
        }
    }

    #[must_use]
    pub const fn from_model(model: MarkovModel) -> Self {
        Self {
            current: Token::Bos,
            model: Some(model),
        }
    }

    pub fn from_bincode(mut data: &[u8]) -> anyhow::Result<Self> {
        let data = bincode::decode_from_std_read(&mut data, bincode::config::standard())
            .context("Failed to decode the file")?;
        Ok(Self {
            current: Token::Bos,
            model: Some(data),
        })
    }

    pub fn from_file<P: AsRef<Path>>(f: P) -> anyhow::Result<Self> {
        let mut f = File::open(f).context("Failed to open the file")?;
        let data = bincode::decode_from_std_read(&mut f, bincode::config::standard())
            .context("Failed to decode the file")?;
        Ok(Self {
            current: Token::Bos,
            model: Some(data),
        })
    }

    pub fn set_model(&mut self, model: MarkovModel) {
        self.model = Some(model);
    }

    #[must_use]
    pub fn get_data(&self) -> Option<MarkovModel> {
        self.model.clone()
    }

    pub fn set_start(&mut self, token: Option<String>) -> anyhow::Result<()> {
        let Some(token) = token else {
            self.current = Token::Bos;
            return Ok(());
        };

        let token = Token::Word(token);
        if !self
            .model
            .as_ref()
            .is_some_and(|data| data.contains_key(&token))
        {
            anyhow::bail!("The model does not have such key");
        }
        self.current = token;
        Ok(())
    }

    fn generate_next(&mut self) -> anyhow::Result<Token> {
        let Some(data) = self.model.as_ref() else {
            anyhow::bail!("The model is not initialized yet");
        };
        let Some(next_candidate) = data.get(&self.current) else {
            return Ok(Token::Eos);
        };

        let next = next_candidate
            .iter()
            .collect::<Vec<(&Token, &u32)>>()
            .choose_weighted(&mut rand::rng(), |item| item.1)
            .unwrap()
            .0
            .clone();
        self.current = next.clone();
        Ok(next)
    }

    pub fn generate(&mut self) -> anyhow::Result<Vec<String>> {
        let mut text = Vec::new();
        if let Token::Word(starting_word) = self.current.clone() {
            text.push(starting_word);
        }
        while let Token::Word(word) = self.generate_next()? {
            text.push(word);
        }
        Ok(text)
    }
}

impl MarkovBuilder {
    pub fn new() -> anyhow::Result<Self> {
        let mut leaner = AdaBoost::new(0.01, 100, 1);
        leaner.load_model_from_reader(MODEL).unwrap();
        Ok(Self::from_segmenter(Segmenter::new(Some(leaner))))
    }

    #[must_use]
    pub fn from_segmenter(segmenter: Segmenter) -> Self {
        Self {
            segmenter,
            model: HashMap::new(),
        }
    }

    fn tokenize(&self, text: &str) -> Vec<Token> {
        let tokenized = self.segmenter.segment(text);
        std::iter::once(Token::Bos)
            .chain(tokenized.into_iter().map(Token::Word))
            .chain(std::iter::once(Token::Eos))
            .collect()
    }

    pub fn learn_one(&mut self, text: &str) {
        let tokens = self.tokenize(text);
        for pair in tokens.windows(2) {
            self.model
                .entry(pair[0].clone())
                .and_modify(|c| {
                    c.entry(pair[1].clone())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                })
                .or_insert_with(|| {
                    let mut new = HashMap::new();
                    new.insert(pair[1].clone(), 1);
                    new
                });
        }
    }

    pub fn learn_many(&mut self, texts: &[String]) {
        for text in texts {
            self.learn_one(text);
        }
    }

    #[must_use]
    pub fn build(self) -> MarkovModel {
        self.model
    }
}

pub fn encode_model(model: MarkovModel) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    bincode::encode_into_std_write(model, &mut buf, bincode::config::standard())
        .context("Failed to serialize the model")?;
    Ok(buf)
}

pub fn save_model<P: AsRef<Path>>(model: MarkovModel, path: P) -> anyhow::Result<()> {
    let mut f = File::create(path).context("Failed to write into the file")?;
    bincode::encode_into_std_write(model, &mut f, bincode::config::standard())
        .context("Failed to write into the file")?;
    Ok(())
}
