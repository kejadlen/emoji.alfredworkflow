extern crate rayon;
extern crate reqwest;
extern crate select;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::{env, fs};
use std::io::prelude::*;
use std::path::Path;

use rayon::prelude::*;
use reqwest::Url;
use select::document::Document;
use select::predicate::{Class, Name, Predicate};

fn main() {
    let cache_dir = env::var("alfred_workflow_cache").unwrap_or_else(|_| ".cache".into());
    let cache_path = Path::new(&cache_dir);
    if !cache_path.exists() {
        fs::create_dir_all(cache_path).unwrap();
    }
    let cache_path_buf = cache_path.to_path_buf();

    let query = env::args().nth(1).unwrap();
    let url = Url::parse_with_params("https://emojipedia.org/search/", &[("q", query)]).unwrap();
    let res = reqwest::get(url).unwrap();

    let doc = Document::from_read(res).unwrap();
    let results: Vec<_> = doc.find(Class("search-results").descendant(Name("h2")
                                                                          .descendant(Name("a"))))
        .map(|node| {
                 let href = node.attr("href").unwrap().to_string();
                 let mut children = node.children();
                 let emoji = children.next().unwrap().text();
                 let text = children.next().unwrap().text();
                 (href, emoji, text)
             })
        .collect();

    let mut items = vec![];
    let base_url = Url::parse("https://emojipedia.org").unwrap();
    results
        .par_iter()
        .map(|&(ref href, ref emoji, ref text)| {
            let slug = href.trim_matches('/');
            let mut file_path = cache_path_buf.clone();
            file_path.push(slug);
            file_path.set_extension("png");

            if !file_path.exists() {
                let url = base_url.join(&href).unwrap();
                let res = reqwest::get(url).unwrap();

                let doc = Document::from_read(res).unwrap();
                let vendor_image = doc.find(Class("vendor-image")).next().unwrap();
                let img = vendor_image.find(Name("img")).next().unwrap();
                let src = img.attr("src").unwrap();

                let url = Url::parse(src).unwrap();
                let mut res = reqwest::get(url).unwrap();
                let mut image = vec![];
                res.read_to_end(&mut image).unwrap();

                let mut file = fs::File::create(file_path.clone()).unwrap();
                file.write_all(&image).unwrap();
            }

            Item {
                uid: emoji.clone(),
                title: text.clone(),
                arg: emoji.clone(),
                icon: Icon { path: file_path.to_str().unwrap().into() },
            }
        })
        .collect_into(&mut items);

    let script_filter = ScriptFilter { items };
    let json = serde_json::to_string(&script_filter).unwrap();
    println!("{}", json);
}

#[derive(Serialize)]
struct ScriptFilter {
    items: Vec<Item>,
}

#[derive(Serialize)]
struct Item {
    uid: String,
    title: String,
    arg: String,
    icon: Icon,
}

#[derive(Serialize)]
struct Icon {
    path: String,
}
