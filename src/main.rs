#![feature(plugin)]
#![plugin(regex_macros)]
extern crate iron;
#[macro_use]
extern crate router;
extern crate mount;
extern crate crowbar;
// extern crate git2;
extern crate hyper;
extern crate regex;
#[macro_use]
extern crate mdo;

use std::path::PathBuf;
use std::process::Command;
use iron::prelude::*;
use iron::{Handler, Protocol};
use mount::Mount;
use crowbar::Index;
// use git2::Repository;

mod crates;
mod cgi;
mod util;

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
    index.set_config(&config).expect("config");

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
        let mut cgi = cgi::Cgi::from_request(r, "git");
        cgi
            .arg("http-backend")
            .env("GIT_PROJECT_ROOT", &self.path)
            .env("GIT_HTTP_EXPORT_ALL", "1")
        ;

        let cgi_resp = cgi.dispatch_with_request_body(r).expect("cgi response");

        println!("HEADERS: {:#?}", cgi_resp.header());

        Ok(Response::with(cgi_resp))
    }
}
