//! A crude cgi parser 

use std::io::{self, BufReader, BufRead, Read, Write};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::ffi::OsStr;
use std::ops::{Deref, DerefMut};
use regex::Regex;
use iron::Request;
use iron::mime::Mime;
use iron::modifier::{Modifier, Set};
use iron::response::WriteBody;
use iron::headers::{
    ContentLength,
    ContentType
};
use util::{OwnedChildStdout, BodyReader};

static FIELD_RE: Regex = regex!(r"([[:alpha:]-]+): *(.*)");

pub type Header = HashMap<String, String>;

pub struct Response<R> {
    header: Header,
    body: R
}

impl <R: BufRead> Response<R> {
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn body(&mut self) -> &mut R {
        &mut self.body
    }
}

pub fn parse<R: BufRead>(mut reader: R) -> io::Result<Response<R>> {
    let mut header = HashMap::new();
    let mut line = String::new();
    loop {
        {
            try!(reader.read_line(&mut line));
            let line = line.trim();
            // Detect end of header
            if line == "" {
                break
            }

            let caps = FIELD_RE.captures(line).expect("header");
            let key = caps.at(1).expect("field key");
            let value = caps.at(2).expect("field value");

            header.insert(key.to_lowercase(), value.to_owned());
        }

        line.clear();
    }

    Ok(Response {
        header: header,
        body: reader
    })
}

pub struct Cgi {
    cmd: Command
}

impl Cgi {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        let mut cgi = Cgi {
            cmd: Command::new(program)
        };

        cgi.env("GATEWAY_INTERFACE", "CGI/1.1");
        cgi.set_method("GET");

        cgi
    }

    pub fn from_request<S: AsRef<OsStr>>(r: &Request, program: S) -> Self {
        let mut cgi = Cgi::new(program);

        // Inherit method
        cgi.set_method(r.method.to_string());

        // Inherit path
        {
            let mut path = String::new();
            for segment in &r.url.path {
                if segment == "" {
                    continue
                }
                path.push('/');
                path.push_str(segment);
            }
            cgi.set_path(path);
        }

        // Inherit query
        if let Some(ref query) = r.url.query {
            cgi.set_query(query);
        }

        // Inherit content type
        if let Some(content_type) = r.headers.get::<ContentType>() {
            cgi.set_content_type(content_type.to_string());
        }

        cgi
    }

    pub fn dispatch_with_request_body(&mut self, r: &mut Request) -> io::Result<Response<BufReader<OwnedChildStdout>>> {
        // Configure stdio
        self
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
        ;

        // Inherit Content-Length
        if let Some(content_length) = r.headers.get::<ContentLength>() {
            self.env("CONTENT_LENGTH", content_length.to_string());
        };

        // Inherit Content-Type
        if let Some(content_type) = r.headers.get::<ContentType>() {
            self.env("CONTENT_TYPE", content_type.to_string());
        };

        // Spawn script
        println!("Spawning: {:#?}", self.cmd);
        let mut child = self.spawn().expect("spawn child");

        // Inherit request body
        {
            let mut data = Vec::new();
            r.body.read_to_end(&mut data).expect("post data");
            // Dropping ChildStdin closes it, so move it out of child
            let mut stdin = child.stdin.take().expect("stdin");
            stdin.write(&data).expect("stdin write");
        }

        let stdout = OwnedChildStdout::from_child(child).expect("stdout");
        let stdout = BufReader::new(stdout);

        parse(stdout)
    }

    pub fn set_method<S: AsRef<OsStr>>(&mut self, method: S) -> &mut Self {
        self.env("REQUEST_METHOD", method);
        self
    }

    pub fn set_path<S: AsRef<OsStr>>(&mut self, path: S) -> &mut Self {
        self.env("PATH_INFO", path);
        self.env_remove("PATH_TRANSLATED");
        self
    }

    pub fn set_path_translared<S: AsRef<OsStr>>(&mut self, path: S) -> &mut Self {
        self.env("PATH_TRANSLATED", path);
        self.env_remove("PATH_INFO");
        self
    }

    pub fn set_query<S: AsRef<OsStr>>(&mut self, query: S) -> &mut Self {
        self.env("QUERY_STRING", query);
        self
    }

    pub fn set_content_type<S: AsRef<OsStr>>(&mut self, content_type: S) -> &mut Self {
        self.env("CONTENT_TYPE", content_type);
        self
    }
}

impl <R: BufRead + Read + Send + 'static> Modifier<::iron::Response> for Response<R> {
    fn modify(self, r: &mut ::iron::Response) {
        use iron::modifiers::Header;
        use iron::status::Status;
        use mdo::option::*;

        // Inherit Content-Length
        if let Some(content_length) = self.header().get("content-length") {
            let content_length = content_length.parse().expect("invalid content length");
            r.set_mut(Header(ContentLength(content_length)));
        }

        // Inherit Content-Type
        if let Some(content_type) = self.header().get("content-type") {
            let mime: Mime = content_type.parse().expect("content type mime");
            println!("RESPONSE: ContentType: {}", mime);
            r.set_mut(mime);
        }

        // Inherit status code
        {
            let code = mdo! {
                status_line =<< self.header().get("status");
                code =<< status_line.split(' ').next();
                code =<< code.parse().ok();
                ret ret(Status::from_u16(code))
            };

            if let Some(code) = code {
                r.set_mut(code);
            } else {
                r.set_mut(Status::Ok);
            }
        }

        // Inherit request body
        let body = BodyReader(self.body);
        let body = Box::new(body) as Box<WriteBody + Send>;
        r.set_mut(body);
    }
}

impl Deref for Cgi {
    type Target = Command;
    fn deref(&self) -> &Self::Target {
        &self.cmd
    }
}

impl DerefMut for Cgi {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cmd
    }
}
