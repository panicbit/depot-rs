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
extern crate logger;
#[macro_use]
extern crate log;
extern crate flexi_logger;

use std::process::Command;
use iron::prelude::*;
use iron::Protocol;
use mount::Mount;
use crowbar::Index;
use logger::Logger;
// use git2::Repository;

mod crates;
mod cgi;
mod util;
mod git;

fn main() {
    configure_logging();
    let (logger_before, logger_after) = Logger::new(None);
    let index_path = "/tmp/index";
    let mut index = crowbar::Index::new(index_path).expect("index");

    let mut mount = Mount::new();
    mount
        .mount("/index", git::Server::new(index_path))
        .mount("/api/v1/crates", crates::crates());

    update_index_config(&mut index);

    let mut mount = Chain::new(mount);
    mount.link_before(logger_before);
    mount.link_after(logger_after);

    Iron::new(mount).listen_with("0.0.0.0:8080", 20, Protocol::Http, None).expect("iron");
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

fn configure_logging() {
    use flexi_logger::LogConfig;
    let config = LogConfig::new();
    let log_level = "DEBUG";
    flexi_logger::init(config, Some(log_level.into())).expect("logging");
}
