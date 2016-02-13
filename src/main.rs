#![feature(plugin)]
#![plugin(regex_macros)]
extern crate iron;
#[macro_use]
extern crate router;
extern crate mount;
extern crate staticfile;
extern crate crowbar;
// extern crate git2;
extern crate hyper;
extern crate regex;
extern crate owning_ref;


use std::path::{Path, PathBuf};
use std::fs::File;
use std::process::{Command, Stdio};
use std::io::{BufReader, BufRead, Read};
use std::cell::RefCell;
use iron::prelude::*;
use iron::{Handler, Protocol, status};
use iron::response::WriteBody;
use iron::mime::Mime;
use iron::method::Method;
use iron::headers::ContentType;
use mount::Mount;
use staticfile::Static;
use crowbar::Index;
// use git2::Repository;
use owning_ref::BoxRef;

mod crates;
mod cgi;

fn main() {
    let index_path = "/tmp/index";
    let mut index = crowbar::Index::new(index_path).expect("index");

    let mut mount = Mount::new();
    mount
        .mount("/index", GitServer::new(index_path))
        .mount("/api/v1/crates", crates::crates());

    update_index_config(&mut index);

    let mut mount = Chain::new(mount);
    mount.link_before(LogAccess);


    Iron::new(mount).listen_with("0.0.0.0:8080", 100, Protocol::Http, None).expect("iron");
}

struct LogAccess;
impl iron::BeforeMiddleware for LogAccess {
    fn before(&self, r: &mut Request) -> IronResult<()> {
        println!("url: {}", r.url);
        Ok(())
    }
}

fn update_index_config(index: &mut Index) {
    let mut config = index.config().expect("config.json");
    config.set_dl("http://localhost:8080/api/v1/crates");
    config.set_api("http://localhost:8080/");
    index.set_config(&config);

    // libgit is too frickin low level

    // let repo = Repository::open(index.path()).expect("index repo");
    // let mut index = repo.index().expect("git index");
    // index.add_path(Path::new("config.json")).expect("add config.json");
    // index.write().expect("index write");

    // let signature = repo.signature().expect("signature");

    // let master = repo.refname_to_id("HEAD").expect("master branch");
    // let tree = repo.find_tree(master).expect("tree");

    // repo.commit(
    //     None,
    //     &signature,
    //     &signature,
    //     "Update config.json",
    //     &tree,
    //     &[]
    // ).expect("commit");

    Command::new("git")
        .args(&["add", "config.json"])
        .current_dir(index.path())
        .spawn()
        .expect("exec git")
        .wait()
        .expect("wait");

    Command::new("git")
        .args(&["commit", "-m", "Update config.json"])
        .current_dir(index.path())
        .spawn()
        .expect("exec git")
        .wait()
        .expect("wait");
    
    Command::new("git")
        .arg("update-server-info")
        .current_dir(index.path())
        .spawn()
        .expect("exec git")
        .wait()
        .expect("wait");
    
}

struct GitServer {
    path: PathBuf
}

impl GitServer {
    fn new<P: Into<PathBuf>>(path: P) -> GitServer {
        let mut path = path.into();
        path.push(".git");

        GitServer {
            path: path
        }
    }
}

impl Handler for GitServer {
    fn handle(&self, r: &mut Request) -> IronResult<Response> {
        println!("METHOD: {}", r.method);

        if r.method == Method::Post {
            let mut post_data = String::new();
            r.body.read_to_string(&mut post_data).expect("post data");
            println!("POST DATA: {:#?}", post_data);
        }

        let mut response = Response::new();
        let mut path = String::new();
        for segment in &r.url.path {
            if segment == "" {
                continue
            }
            path.push('/');
            path.push_str(segment);
        }

        let query = r.url.query.as_ref().map(|s| &s[..]).unwrap_or("");
        let content_type = r.headers.get::<ContentType>()
            .map(|ct| ct.to_string())
            .unwrap_or_else(|| String::new());

        let git_backend = Command::new("git")
            .arg("http-backend")
            .env("REQUEST_METHOD", r.method.to_string())
            .env("GIT_PROJECT_ROOT", &self.path)
            .env("PATH_INFO", path)
            .env("QUERY_STRING", query)
            .env("CONTENT_TYPE", content_type)
            .env("GIT_HTTP_EXPORT_ALL", "1")
            .stdout(Stdio::piped())
            .spawn().expect("git backend");

        let stdout = git_backend.stdout.expect("stdout");
        let stdout = BufReader::new(stdout);

        let mut git_resp = cgi::parse(stdout).expect("cgi response");

        println!("HEADER: {:#?}", git_resp.header());

        // Get content type from git
        if let Some(content_type) = git_resp.header().get("content-type") {
            println!("Setting content-type");
            let mime: Mime = content_type.parse().expect("content type mime");
            println!("Parsed: {}", mime);
            response = response.set(mime);
        }


        // FIXME: Inefficient. How to avoid?
        // Read body
        let content_length: Option<u64> = git_resp
            .header().get("content_length")
            .and_then(|n|n.parse().ok());

        let mut data = Vec::new();

        if let Some(content_length) = content_length {
            git_resp.body().take(content_length).read_to_end(&mut data).expect("body");
        }
        else {
            git_resp.body().read_to_end(&mut data).expect("body");
        }
        println!("GIT SAID {:?}", data);
        response = response.set(data);

        // TODO: Get status from git
        response = response.set(status::Ok);

        Ok(response)
    }
}