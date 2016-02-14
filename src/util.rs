use std::io::{self, Read};
use std::process::{Child, ChildStdout};
use std::ops::Drop;
use iron::response::{WriteBody, ResponseBody};

pub struct OwnedChildStdout {
    child: Child,
    stdout: ChildStdout
}

impl OwnedChildStdout {
    pub fn from_child(mut child: Child) -> Option<Self> {
        child.stdout.take().map(|stdout| OwnedChildStdout {
            child: child,
            stdout: stdout
        })
    }
}

impl Drop for OwnedChildStdout {
    fn drop(&mut self) {
        println!("Waiting for command to terminate");
        self.child.wait().expect("child wait");
        println!("Command terminated");
    }
}

impl Read for OwnedChildStdout {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdout.read(buf)
    }
}

pub struct BodyReader<R>(pub R);

impl <R: Read> WriteBody for BodyReader<R> {
    fn write_body(&mut self, res: &mut ResponseBody) -> io::Result<()> {
        try!(io::copy(&mut self.0, res));
        Ok(())
    }
}
