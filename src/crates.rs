use std::fs::{File, create_dir_all};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;
use iron::prelude::*;
use iron::{Handler, status};
use router::Router;
use hyper::{Client, Url};
use iron::response::{ResponseBody, WriteBody};
use iron::modifier::Modifier;
use ::util::OptionalTee;

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

        let body = CachedCrateBody::new(&**name, &**version);

        Ok(Response::with((status::Ok, body)))
    })
}

struct CachedCrateBody {
    name: String,
    version: String
}

impl CachedCrateBody {
    pub fn new<S1, S2>(name: S1, version: S2) -> Self where
        S1: Into<String>,
        S2: Into<String>
    {
        CachedCrateBody {
            name: name.into(),
            version: version.into()
        }
    }
}

impl Modifier<Response> for CachedCrateBody {
    fn modify(self, res: &mut Response) {
        // If crate is already cached, use File as response,
        // otherwise use CrateDownloadBody as response
        match get_cached_crate(&self.name, &self.version) {
            Ok(file) => {
                res.set_mut((status::Ok, file));
            },
            Err(_) => {
                println!("Cache miss");

                let body = CrateDownloadBody::new(&self.name, &self.version)
                    .expect("download failed");

                res.body = Some(Box::new(body));
            }
        };
    }
}

struct CrateDownloadBody {
    dl: ::hyper::client::Response,
    file: File
}

impl CrateDownloadBody {
    fn new(name: &str, version: &str) -> ::hyper::Result<CrateDownloadBody> {
        let download_path = construct_download_path(name, version);
        let url = try!(Url::parse(
            &format!("https://crates.io/api/v1/crates/{}/{}/download", name, version)
        ));

        let response = try!(Client::new().get(url).send());

        if let Some(dl_dir) = download_path.parent() {
            try!(create_dir_all(dl_dir));
        };

        let file = try!(File::create(download_path));

        Ok(CrateDownloadBody {
            dl: response,
            file: file
        })
    }
}

impl WriteBody for CrateDownloadBody {
    fn write_body(&mut self, res: &mut ResponseBody) -> io::Result<()> {
        let tee = OptionalTee::new(&mut self.file, res);
        let ref mut tee = BufWriter::new(tee);
        let ref mut dl = BufReader::new(&mut self.dl);
        io::copy(dl, tee).map(|_| ())
    }
}

fn construct_download_path(name: &str, version: &str) -> PathBuf {
    // TODO: Use dir layout like index [issue #6]
    let mut path = PathBuf::from(CACHE_DIR);
    path.push(name);
    path.push(version);
    path.push("download");
    path
}

fn get_cached_crate(name: &str, version: &str) -> io::Result<File> {
    let path = construct_download_path(name, version);
    File::open(path)
}
