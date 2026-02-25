/*
 * SPDX-FileCopyrightText: 2025 SyoBoN <syobon@syobon.net>
 *
 * SPDX-License-Identifier: UPL-1.0
 */

use std::{
    io::{Cursor, Read},
    path::Path,
};

use anyhow::Context;
use regex::Regex;
use serde::Deserialize;
use zip::ZipArchive;

#[derive(Deserialize)]
struct Post {
    content: Option<String>,
}

pub fn parse_file<P: AsRef<Path>>(tar_zip: P) -> anyhow::Result<Vec<String>> {
    let contents = std::fs::read(tar_zip).context("Failed to read zip file")?;
    parse(&contents)
}

pub fn parse(data: &[u8]) -> anyhow::Result<Vec<String>> {
    let json = extract(data)?;
    let posts = unwrap_htmls_from_json(&json)?;
    let texts = htmls_to_texts(&posts);
    Ok(texts)
}

fn extract(zip: &[u8]) -> anyhow::Result<Vec<String>> {
    let zip = Cursor::new(zip);
    let mut zip = ZipArchive::new(zip).context("Failed to read zip file")?;

    let mut jsons: Vec<String> = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).context("Failed to extract zip file")?;
        let mut buf = String::new();
        entry
            .read_to_string(&mut buf)
            .context("Failed to read from zip file")?;
        jsons.push(buf);
    }

    Ok(jsons)
}

fn unwrap_htmls_from_json(jsons: &[String]) -> anyhow::Result<Vec<String>> {
    let mut posts: Vec<String> = Vec::new();
    for json in jsons {
        let p: Vec<Post> = serde_json::from_str(json).context("Failed to parse json")?;
        posts.extend(p.into_iter().filter_map(|p| p.content));
    }
    Ok(posts)
}

fn htmls_to_texts(posts: &[String]) -> Vec<String> {
    posts
        .iter()
        .filter(|p| filter(p))
        .flat_map(|p| html_to_text(p))
        .collect()
}

// 確定ゴミデータを消す
// なんかもっといいかんじにしたい
fn filter(html: &str) -> bool {
    // Mondo
    if html.contains("#クイズMondo") {
        return false;
    }

    // puzzlega.me
    if html.contains("https://puzzlega.me/") {
        return false;
    }

    // Daily Akari
    if html.contains("https://dailyakari.com/") {
        return false;
    }

    // Daily Alpaca Hack
    if html.contains("https://alpacahack.com/daily") {
        return false;
    }

    // 二重学習禁止
    if html.contains("#markov-generator-fedi") {
        return false;
    }
    if html.contains("#fedi_markov_chain_wasm") {
        return false;
    }

    true
}

fn html_to_text(html: &str) -> Vec<String> {
    let remove_a_tag = Regex::new(r"<a[ >].*?</a>").unwrap();
    let html = remove_a_tag.replace_all(html, "");

    let remove_pre_tag = Regex::new(r"(?s)<pre[ >].*?</pre>").unwrap();
    let html = remove_pre_tag.replace_all(&html, "");

    let remove_code_tag = Regex::new(r"<code[ >].*?</code>").unwrap();
    let html = remove_code_tag.replace_all(&html, "");

    let remove_blockquote_tag = Regex::new(r"<blockquote[ >].*?</blockquote>").unwrap();
    let html = remove_blockquote_tag.replace_all(&html, "");

    let text = nanohtml2text::html2text(&html);

    let sanitize_newline = Regex::new(r"[\r\n]+").unwrap();
    let text = sanitize_newline.replace_all(&text, "\n");

    let sanitize_spaces = Regex::new(r"[ 　]+").unwrap();
    let text = sanitize_spaces.replace_all(&text, " ");

    text.trim()
        .lines()
        .filter(|l| !l.starts_with("RE:")) // インライン引用を削除 (TODO: `QT: `とかの場合にも対応したい)
        .map(std::borrow::ToOwned::to_owned)
        .collect()
}
