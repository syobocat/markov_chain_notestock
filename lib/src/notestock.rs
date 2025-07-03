use std::{
    io::{Cursor, Read},
    path::Path,
};

use anyhow::Context;
use regex::Regex;
use serde::Deserialize;
use tar::Archive;
use zip::ZipArchive;

#[derive(Deserialize)]
struct Post {
    content: String,
}

pub fn parse_file<P: AsRef<Path>>(tar_zip: P) -> anyhow::Result<Vec<String>> {
    let contents = std::fs::read(tar_zip).context("Failed to read zip file")?;
    parse(&contents)
}

pub fn parse(data: &[u8]) -> anyhow::Result<Vec<String>> {
    let json = extract(data)?;
    let posts = json_to_posts(&json)?;
    let texts = posts_to_texts(posts);
    Ok(texts)
}

fn extract(tar_zip: &[u8]) -> anyhow::Result<String> {
    let tar_zip = Cursor::new(tar_zip);
    let mut tar_zip = ZipArchive::new(tar_zip).context("Failed to read zip file")?;
    if tar_zip.len() != 1 {
        anyhow::bail!("Zip downloaded from Notestock should have only 1 file");
    }
    let tar = tar_zip.by_index(0).context("Failed to extract zip file")?;
    let mut tar = Archive::new(tar);

    let mut json = String::new();
    for entry in tar.entries().context("Failed to read tar file")? {
        let mut entry = entry.context("Failed to read tar file")?;
        let mut buf = String::new();
        entry
            .read_to_string(&mut buf)
            .context("Failed to read tar file")?;
        json += &buf[0..buf.len() - 1];
    }
    json.push(']');
    Ok(json)
}

fn json_to_posts(json: &str) -> anyhow::Result<Vec<Post>> {
    serde_json::from_str(json).context("Failed to parse json")
}

fn posts_to_texts(posts: Vec<Post>) -> Vec<String> {
    posts
        .into_iter()
        .filter_map(|p| html_to_text(&p.content))
        .collect()
}

fn html_to_text(html: &str) -> Option<String> {
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

    let remove_trailing_spaces = Regex::new(r"[[:space:]]+$").unwrap();
    let text = remove_trailing_spaces.replace_all(&text, "");

    if text.is_empty() {
        None
    } else {
        Some(text.into_owned())
    }
}
