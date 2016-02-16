use std::path::PathBuf;
use iron::prelude::*;
use iron::Handler;
use ::cgi;

pub struct Server {
    path: PathBuf
}

impl Server {
    pub fn new<P: Into<PathBuf>>(path: P) -> Server {
        let mut path = path.into();
        path.push(".git");

        Server {
            path: path
        }
    }
}

impl Handler for Server {
    fn handle(&self, r: &mut Request) -> IronResult<Response> {
        let mut cgi = cgi::Cgi::from_request(r, "git");
        cgi
            .arg("http-backend")
            .env("GIT_PROJECT_ROOT", &self.path)
            .env("GIT_HTTP_EXPORT_ALL", "1")
        ;

        let cgi_resp = cgi.dispatch_with_request_body(r).expect("cgi response");

        debug!("HEADERS: {:#?}", cgi_resp.header());

        Ok(Response::with(cgi_resp))
    }
}
