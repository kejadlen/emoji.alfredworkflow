use std::env;
use std::io::prelude::*;

use alphred::{cached, Item, Workflow};
use anyhow::{Context, Result};
use rayon::prelude::*;
use reqwest::Url;
use select::document::Document;
use select::predicate::{Class, Name, Predicate};

fn main() {
    let query = env::args().nth(1).unwrap();
    run(&query);
}

fn run(query: &str) {
    let current_dir = env::current_dir().unwrap();

    let workflow = Workflow::new(|| {
        let items = search_results(query).and_then(|results| items(&results))?;

        if items.is_empty() {
            let icon_path = current_dir.join("broken_heart.png");
            return Ok(vec![Item::new("No results found").icon(icon_path.as_path())]);
        }

        Ok(items)
    });
    println!("{}", workflow);
}

struct SearchResult {
    href: String,
    emoji: String,
    text: String,
}

fn search_results(query: &str) -> Result<Vec<SearchResult>> {
    let url = Url::parse_with_params("https://emojipedia.org/search/", &[("q", query)]).unwrap();
    let res = reqwest::blocking::get(url)?;
    let doc = Document::from_read(res)?;

    doc.find(Class("search-results").descendant(Name("h2").descendant(Name("a"))))
        .flat_map(|node| {
            node.find(Class("emoji"))
                .next()
                .map(|elem| (node, elem.text()))
        })
        .map(|(node, emoji)| {
            let href = node.attr("href").context("Unable to get href")?.to_string();
            let mut children = node.children();
            let text = children.nth(1).context("Unable to get text")?.text();
            Ok(SearchResult { href, emoji, text })
        })
        .collect()
}

fn items(results: &[SearchResult]) -> Result<Vec<Item>> {
    let mut items = vec![];
    results
        .par_iter()
        .map(|search_result| {
            let href = &search_result.href;
            let emoji = &search_result.emoji;

            let file_name = format!("{}.png", href.trim_matches('/'));

            let uid = emoji.clone();
            let title = search_result.text.clone();
            let arg = emoji.clone();
            let icon_path = cached(&file_name, || download_emoji_image(href))?;

            Ok(Item::new(title).uid(&uid).arg(&arg).icon(icon_path.as_path()))
        })
        .collect_into_vec(&mut items);

    items.into_iter().collect()
}

fn download_emoji_image(href: &str) -> Result<Vec<u8>> {
    let base_url = Url::parse("https://emojipedia.org").unwrap();
    let url = base_url.join(href).unwrap();
    let res = reqwest::blocking::get(url)?;

    let doc = Document::from_read(res)?;
    let vendor_image = doc
        .find(Class("vendor-image"))
        .next()
        .context("Unable to find emoji image")?;
    let img = vendor_image
        .find(Name("img"))
        .next()
        .context("Unable to find emoji image")?;
    let src = img
        .attr("data-cfsrc")
        .context("Unable to find emoji image")?;

    let url = Url::parse(src)?;
    let mut res = reqwest::blocking::get(url)?;
    let mut image = vec![];
    res.read_to_end(&mut image)?;
    Ok(image)
}
