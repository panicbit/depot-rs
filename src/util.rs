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
        debug!("Waiting for command to terminate");
        self.child.wait().expect("child wait");
        debug!("Command terminated");
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

pub struct OptionalTee<R1, R2> {
    important: R1,
    optional: Option<R2>
}

impl <R1: io::Write, R2: io::Write> OptionalTee<R1, R2> {
    pub fn new(important: R1, optional: R2) -> Self {
        OptionalTee {
            important: important,
            optional: Some(optional)
        }
    }
}

impl <R1: io::Write, R2: io::Write> io::Write for OptionalTee<R1, R2> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = self.important.write_all(buf);

        self.optional = self.optional.take().and_then(|mut optional| {
            optional.write_all(buf).ok().and(Some(optional))
        });

        result.and(Ok(buf.len()))
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = self.important.flush();

        self.optional = self.optional.take().and_then(|mut optional| {
            optional.flush().ok().and(Some(optional))
        });

        result
    }
}
