extern crate hyper;
extern crate hyper_native_tls;
extern crate rayon;
extern crate select;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::{env, fs};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use hyper::{Client, Url};
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use rayon::prelude::*;
use select::document::Document;
use select::predicate::{Class, Name, Predicate};

fn main() {
    let workflow = Workflow::new();

    let query = env::args().nth(1).unwrap();
    let results = workflow.search(&query);

    let mut items = vec![];
    results
        .par_iter()
        .map(|search_result| {
            let uid = search_result.emoji.clone();
            let title = search_result.text.clone();
            let arg = search_result.emoji.clone();
            let path = workflow
                .download_image(&search_result.href)
                .to_str()
                .unwrap()
                .into();
            let icon = Icon { path };
            Item {
                uid,
                title,
                arg,
                icon,
            }
        })
        .collect_into(&mut items);

    let script_filter = ScriptFilter { items };
    let json = serde_json::to_string(&script_filter).unwrap();
    println!("{}", json);
}

struct Workflow {
    cache_path_buf: PathBuf,
    client: Client,
    base_url: Url,
}

impl Workflow {
    fn new() -> Self {
        let cache_dir = env::var("alfred_workflow_cache").unwrap_or_else(|_| ".cache".into());
        let cache_path = Path::new(&cache_dir);
        if !cache_path.exists() {
            fs::create_dir_all(cache_path).unwrap();
        }
        let cache_path_buf = cache_path.to_path_buf();

        let ssl = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        let client = Client::with_connector(connector);

        let base_url = Url::parse("https://emojipedia.org").unwrap();

        Workflow {
            cache_path_buf,
            client,
            base_url,
        }
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let url = Url::parse_with_params("https://emojipedia.org/search/", &[("q", query)])
            .unwrap();
        let res = self.client.get(url).send().unwrap();

        let doc = Document::from_read(res).unwrap();
        doc.find(Class("search-results").descendant(Name("h2").descendant(Name("a"))))
            .map(|node| {
                     let href = node.attr("href").unwrap().to_string();
                     let mut children = node.children();
                     let emoji = children.next().unwrap().text();
                     let text = children.next().unwrap().text();
                     SearchResult { href, emoji, text }
                 })
            .collect()
    }

    fn download_image(&self, href: &str) -> PathBuf {
        let url = self.base_url.join(href).unwrap();
        let res = self.client.get(url).send().unwrap();

        let doc = Document::from_read(res).unwrap();
        let vendor_image = doc.find(Class("vendor-image")).next().unwrap();
        let img = vendor_image.find(Name("img")).next().unwrap();
        let src = img.attr("src").unwrap();

        let url = Url::parse(src).unwrap();
        let mut res = self.client.get(url).send().unwrap();

        let file_name = href.trim_matches('/');
        let mut file_path = self.cache_path_buf.clone();
        file_path.push(file_name);
        file_path.set_extension("png");

        if file_path.exists() {
            return file_path;
        }

        let mut file = fs::File::create(file_path.clone()).unwrap();

        let mut buf = vec![];
        res.read_to_end(&mut buf).unwrap();
        file.write_all(&buf).unwrap();

        file_path
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
    uid: String,
    title: String,
    arg: String,
    icon: Icon,
}

#[derive(Serialize)]
struct Icon {
    path: String,
}
