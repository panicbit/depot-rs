//! A crude cgi parser 

use std::io::{self, BufRead};
use std::collections::HashMap;
use regex::Regex;
use std::process::Command;
use std::ops::{Deref, DerefMut};

static field_re: Regex = regex!(r"([[:alpha:]-]+): *(.*)");

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

            let caps = field_re.captures(line).expect("header");
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

struct Cgi {
    cmd: Command
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