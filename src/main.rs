#![recursion_limit = "1024"]

extern crate failure;
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
use std::path::PathBuf;

use failure::*;
use rayon::prelude::*;
use reqwest::Url;
use select::document::Document;
use select::predicate::{Class, Name, Predicate};

fn main() {
    let query = env::args().nth(1).unwrap();
    Workflow::new().run(&query);
}

struct Workflow {
    current_dir: PathBuf,
    cache_path_buf: PathBuf,
}

impl Workflow {
    fn new() -> Self {
        let current_dir = env::current_dir().unwrap();

        let cache_dir = env::var("alfred_workflow_cache").unwrap_or_else(|_| ".cache".into());
        let cache_path = Path::new(&cache_dir);
        if !cache_path.exists() {
            fs::create_dir_all(cache_path).unwrap();
        }
        let cache_path_buf = cache_path.to_path_buf();

        Workflow {
            current_dir,
            cache_path_buf,
        }
    }

    fn run(&self, query: &str) {
        let items = match self.search_results(query)
            .and_then(|results| self.items(&results))
        {
            Ok(ref items) if items.is_empty() => {
                let icon_path = self.current_dir.join("broken_heart.png");
                vec![
                    Item {
                        uid: None,
                        title: "No results found".into(),
                        arg: None,
                        icon: Some(Icon {
                            path: icon_path.to_str().unwrap().into(),
                        }),
                    },
                ]
            }
            Ok(items) => items,
            Err(e) => {
                let icon_path = self.current_dir.join("exclamation_point.png");
                vec![
                    Item {
                        uid: None,
                        title: format!("{}", e),
                        arg: None,
                        icon: Some(Icon {
                            path: icon_path.to_str().unwrap().into(),
                        }),
                    },
                ]
            }
        };

        let script_filter = ScriptFilter { items };
        let json = serde_json::to_string(&script_filter).unwrap();
        println!("{}", json);
    }

    fn search_results(&self, query: &str) -> Result<Vec<SearchResult>, Error> {
        let url =
            Url::parse_with_params("https://emojipedia.org/search/", &[("q", query)]).unwrap();
        let res = reqwest::get(url).context("Unable to get search results")?;
        let doc = Document::from_read(res).context("Unable to parse search results")?;

        doc.find(Class("search-results").descendant(Name("h2").descendant(Name("a"))))
            .flat_map(|node| {
                node.find(Class("emoji"))
                    .next()
                    .map(|elem| (node, elem.text()))
            })
            .map(|(node, emoji)| {
                let href = node.attr("href")
                    .ok_or_else(|| err_msg("Unable to get href"))?
                    .to_string();
                let mut children = node.children();
                let text = children
                    .nth(1)
                    .ok_or_else(|| err_msg("Unable to get text"))?
                    .text();
                Ok(SearchResult { href, emoji, text })
            })
            .collect()
    }

    fn items(&self, results: &[SearchResult]) -> Result<Vec<Item>, Error> {
        let mut items = vec![];
        results
            .par_iter()
            .map(|search_result| {
                let href = &search_result.href;
                let emoji = &search_result.emoji;

                let file_name = format!("{}.png", href.trim_matches('/'));
                let cache_path = self.cache(&file_name, || self.download_emoji_image(href))?;

                let uid = Some(emoji.clone());
                let title = search_result.text.clone();
                let arg = Some(emoji.clone());
                let icon_path = cache_path.to_str().unwrap();
                let icon = Some(Icon {
                    path: icon_path.into(),
                });
                Ok(Item {
                    uid,
                    title,
                    arg,
                    icon,
                })
            })
            .collect_into(&mut items);

        items.into_iter().collect()
    }

    fn cache<F>(&self, file_name: &str, f: F) -> Result<PathBuf, Error>
    where
        F: Fn() -> Result<Vec<u8>, Error>,
    {
        let file_path = self.cache_path_buf.join(file_name);
        if !file_path.exists() {
            let mut file =
                fs::File::create(file_path.clone()).context("Unable to create cache file")?;
            let image = f()?;
            file.write_all(&image)
                .context("Unable to write cache file")?;
        }
        Ok(file_path)
    }

    fn download_emoji_image(&self, href: &str) -> Result<Vec<u8>, Error> {
        let base_url = Url::parse("https://emojipedia.org").unwrap();
        let url = base_url.join(href).unwrap();
        let res = reqwest::get(url).context("Unable to fetch emoji")?;

        let doc = Document::from_read(res).context("Unable to parse emoji")?;
        let vendor_image = doc.find(Class("vendor-image"))
            .next()
            .ok_or_else(|| err_msg("Unable to find emoji image"))?;
        let img = vendor_image
            .find(Name("img"))
            .next()
            .ok_or_else(|| err_msg("Unable to find emoji image"))?;
        let src = img.attr("src")
            .ok_or_else(|| err_msg("Unable to find emoji image"))?;

        let url = Url::parse(src).context("Unable to find emoji image")?;
        let mut res = reqwest::get(url).context("Unable to download emoji image")?;
        let mut image = vec![];
        res.read_to_end(&mut image)
            .context("Unable to save emoji image")?;
        Ok(image)
    }
}

struct SearchResult {
    href: String,
    emoji: String,
    text: String,
}

#[derive(Serialize)]
struct ScriptFilter {
    items: Vec<Item>,
}

#[derive(Serialize)]
struct Item {
    uid: Option<String>,
    title: String,
    arg: Option<String>,
    icon: Option<Icon>,
}

#[derive(Serialize)]
struct Icon {
    path: String,
}
