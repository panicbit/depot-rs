use std::fs::{File, create_dir_all};
use std::io;
use std::path::{Path, PathBuf};
use iron::prelude::*;
use iron::{Handler, status};
use router::Router;
use hyper::{Client, Url};

const CACHE_DIR: &'static str = "/tmp/crate_cache";

pub fn crates() -> Router {
    router! {
        get "/:name/:version/download" => download(),
    }
}

fn download() -> Box<Handler> {
    Box::new(move |r: &mut Request| {
        let params = r.extensions.get::<Router>().expect("download router params");
        let name = &params["name"];
        let version = &params["version"];

        let file = get_cached_file(name, version).expect("cached file");

        Ok(Response::with((status::Ok, file)))
    })
}

fn get_cached_file(name: &str, version: &str) -> io::Result<File> {
    let mut path = PathBuf::from(CACHE_DIR);
    path.push(name);
    path.push(version);
    path.push("download");

    if !path.exists() {
        println!("Cache miss");
        download_crate(&path, name, version);
    }

    File::open(path)
}

fn download_crate<P: AsRef<Path>>(download_path: P, name: &str, version: &str) {
    let download_path = download_path.as_ref();
    download_path.parent().and_then(|parent|
        Some(create_dir_all(parent).expect("create dir"))
    );

    let client = Client::new();
    let url = Url::parse(
        &format!("https://crates.io/api/v1/crates/{}/{}/download", name, version)
    ).expect("download url");

    println!("Downloading {}", url);

    let ref mut response = client.get(url).send().expect("get request");
    let ref mut file = File::create(download_path).expect("download target");

    io::copy(response, file).expect("download");
}