use std::fmt;
use std::io::{
    self, Error, ErrorKind, IntoInnerError, IoSlice, Seek, SeekFrom, Write,
};
use std::convert::TryInto;
use std::process;
use backtrace::Backtrace;
use libc;
use crate::common::{PUUID, UUID, WISKFDS};
use crate::TRACER;


pub struct BufWriter<W: Write> {
    inner: Option<W>,
    // #30888: If the inner writer panics in a call to write, we don't want to
    // write the buffered data a second time in BufWriter's destructor. This
    // flag tells the Drop impl if it should skip the flush.
    panicked: bool,
    #[thread_local]
    buf: [i32; libc::PIPE_BUF],
    #[thread_local]
    ind: usize,
}

impl<W: Write> BufWriter<W> {
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter {
            inner: Some(inner),
            panicked: false,
            buf: [0; libc::PIPE_BUF],
            ind: 0,
        }
    }

    pub(super) fn flush_buf(&mut self) -> io::Result<()> {
        /// Helper struct to ensure the buffer is updated after all the writes
        /// are complete. It tracks the number of written bytes and drains them
        /// all from the front of the buffer when dropped.
        // struct BufGuard<'a> {
        //     buffer: &'a mut Vec<u8>,
        //     written: usize,
        // }

        // impl<'a> BufGuard<'a> {
        //     fn new(buffer: &'a mut Vec<u8>) -> Self {
        //         Self { buffer, written: 0 }
        //     }

        //     /// The unwritten part of the buffer
        //     fn remaining(&self) -> &[u8] {
        //         &self.buffer[self.written..]
        //     }

        //     /// Flag some bytes as removed from the front of the buffer
        //     fn consume(&mut self, amt: usize) {
        //         self.written += amt;
        //     }

        //     /// true if all of the bytes have been written
        //     fn done(&self) -> bool {
        //         self.written >= self.buffer.len()
        //     }
        // }

        // impl Drop for BufGuard<'_> {
        //     fn drop(&mut self) {
        //         if self.written > 0 {
        //             self.buffer.drain(..self.written);
        //         }
        //     }
        // }

        // let mut guard = BufGuard::new(&mut self.buf);
        // let inner = self.inner.as_mut().unwrap();
        // while !guard.done() {
        //     self.panicked = true;
        //     let r = inner.write(guard.remaining());
        //     self.panicked = false;

        //     match r {
        //         Ok(0) => {
        //             return Err(Error::new(
        //                 ErrorKind::WriteZero,
        //                 "failed to write the buffered data",
        //             ));
        //         }
        //         Ok(n) => guard.consume(n),
        //         Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
        //         Err(e) => return Err(e),
        //     }
        // }
        self.inner.as_mut().unwrap().flush();
        Ok(())
    }

    /// Buffer some data without flushing it, regardless of the size of the
    /// data. Writes as much as possible without exceeding capacity. Returns
    /// the number of bytes written.
    pub(super) fn write_to_buf(&mut self, buf: &[u8]) -> usize {
        // let available = self.buf.capacity() - self.buf.len();
        // let amt_to_buffer = available.min(buf.len());
        // self.buf.extend_from_slice(&buf[..amt_to_buffer]);
        // amt_to_buffer
        buf.len()
    }

    pub fn get_ref(&self) -> &W {
        self.inner.as_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.inner.as_mut().unwrap()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buf().and_then(|()| self.get_mut().flush())
    }
}


impl<W: Write> Write for BufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        cevent!(Level::INFO, "BufWriter::write(buf={:?})", String::from_utf8_lossy(buf));
        // if self.buf.len() + buf.len() > self.buf.capacity() {
        //     self.flush_buf()?;
        // }
        // // FIXME: Why no len > capacity? Why not buffer len == capacity? #72919
        // if buf.len() >= self.buf.capacity() {
        //     self.panicked = true;
        //     let r = self.get_mut().write(buf);
        //     self.panicked = false;
        //     r
        // } else {
        //     self.buf.extend_from_slice(buf);
        //     Ok(buf.len())
        // }
        let r = self.get_mut().write(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        cevent!(Level::INFO, "BufWriter::write_all(buf={:?})", String::from_utf8_lossy(buf));
        // // Normally, `write_all` just calls `write` in a loop. We can do better
        // // by calling `self.get_mut().write_all()` directly, which avoids
        // // round trips through the buffer in the event of a series of partial
        // // writes in some circumstances.
        // if self.buf.len() + buf.len() > self.buf.capacity() {
        //     self.flush_buf()?;
        // }
        // // FIXME: Why no len > capacity? Why not buffer len == capacity? #72919
        // if buf.len() >= self.buf.capacity() {
        //     self.panicked = true;
        //     let r = self.get_mut().write_all(buf);
        //     self.panicked = false;
        //     r
        // } else {
        //     self.buf.extend_from_slice(buf);
        //     Ok(())
        // }
        let r = self.get_mut().write_all(buf);
        Ok(())
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let total_len = bufs.iter().map(|b| b.len()).sum::<usize>();
        // if self.buf.len() + total_len > self.buf.capacity() {
        //     self.flush_buf()?;
        // }
        // // FIXME: Why no len > capacity? Why not buffer len == capacity? #72919
        // if total_len >= self.buf.capacity() {
        //     self.panicked = true;
        //     let r = self.get_mut().write_vectored(bufs);
        //     self.panicked = false;
        //     r
        // } else {
        //     bufs.iter().for_each(|b| self.buf.extend_from_slice(b));
        //     Ok(total_len)
        // }
        Ok(total_len)
    }

    // fn is_write_vectored(&self) -> bool {
    //     self.get_ref().is_write_vectored()
    // }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buf().and_then(|()| self.get_mut().flush())
    }
}

impl<W: Write> fmt::Debug for BufWriter<W>
where
    W: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BufWriter")
            .field("writer", &self.inner.as_ref().unwrap())
            .field("buffer", &format_args!("{}/{}", self.buf.len(), libc::PIPE_BUF))
            .finish()
    }
}

impl<W: Write + Seek> Seek for BufWriter<W> {
    /// Seek to the offset, in bytes, in the underlying writer.
    ///
    /// Seeking always writes out the internal buffer before seeking.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        errorexit!("Not implemented");
        // self.flush_buf()?;
        // self.get_mut().seek(pos)
    }
}

impl<W: Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        if self.inner.is_some() && !self.panicked {
            // dtors should not panic, so we ignore a failed flush
            let _r = self.flush_buf();
        }
    }
}


#[cfg(test)]
mod tests_bufwriter {
    use std::io;
    use libc::{O_CLOEXEC,O_RDONLY,O_CREAT,O_WRONLY,O_TRUNC,O_APPEND,O_LARGEFILE,S_IRUSR,S_IWUSR,S_IRGRP,S_IWGRP};
    use std::os::unix::io::{FromRawFd};
    use std::fs;
    use std::io::{Read, Write};
    // use std::fmt::Write as FmtWrite;
    use std::path::Path;
    use std::{thread, time};
    use std::str;
    use super::*;
    use crate::fs::File;

    static MODULENAME: &str = "bufwriter";
    const BUFWRITERFDBASE:i32 = 400;

    pub struct TestTracer {
        pub file: BufWriter<File>,
    }

    impl TestTracer {
        pub fn new() -> TestTracer {
            let f: File = File::open(&format!("/tmp/tests_{}_dataglobal", MODULENAME),
                                     (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                     (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                     BUFWRITERFDBASE+5).unwrap();
            let f = BufWriter::new(f);
            let t = TestTracer {
                file: f,
            };
            t
        }
    }

    lazy_static! {
        pub static ref TRACER: TestTracer = TestTracer::new();
    }

    pub fn setup(tfile: &str) -> io::Result<()> {
        if Path::new(tfile).exists() {
            fs::remove_file(tfile)?;
        }
        Ok(())
    }
    pub fn cleanup(tfile: &str) -> io::Result<()> {
        if Path::new(tfile).exists() {
            fs::remove_file(tfile)?;
        }
        Ok(())
    }

    #[test]
    fn report_test_000() -> io::Result<()> {
        static TESTID: &str = "000";
        setup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?;
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                                   (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                    (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                    BUFWRITERFDBASE+0)?;
        let mut rfile = BufWriter::new(rfile);

        let message = format!("Hello World: {}\n", TESTID);
        rfile.write(message.as_bytes()).unwrap();
        rfile.flush();
        assert_eq!(BUFWRITERFDBASE+0, rfile.get_ref().as_raw_fd());
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World: {}\n", TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_001() -> io::Result<()> {
        static TESTID: &str = "001";
        setup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?;
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                                   (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                    (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                    BUFWRITERFDBASE+1)?;
        let mut rfile = BufWriter::new(rfile);
        let message = format!("Hello World: {}\n", TESTID);
        rfile.write(message.as_bytes()).unwrap();
        rfile.flush();
        assert_eq!(BUFWRITERFDBASE+1, rfile.get_ref().as_raw_fd());
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World: {}\n", TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_002() -> io::Result<()> {
        static TESTID: &str = "002";
        setup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?;
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                                   (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                    (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                    -1)?;
        let mut rfile = BufWriter::new(rfile);
        let message = format!("Hello World: {}\n", TESTID);
        // assert_eq!(3, rfile.as_raw_fd());
        rfile.write(message.as_bytes()).unwrap();
        rfile.flush();
        assert_eq!(fs::read_to_string(format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World: {}\n", TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_003() -> io::Result<()> {
        static TESTID: &str = "003";
        setup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?;
        let ofile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                               (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                               (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                               BUFWRITERFDBASE+3)?;
        ofile.sync_all();
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                               (O_CREAT|O_WRONLY|O_APPEND|O_LARGEFILE) as i32,
                               (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                               BUFWRITERFDBASE+3)?;
        let mut rfile = BufWriter::new(rfile);
        assert_eq!(rfile.get_ref().as_raw_fd(), BUFWRITERFDBASE+3);
        let message = format!("Hello World: {}\n", TESTID);
        rfile.write(message.as_bytes()).unwrap();
        rfile.flush();
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World: {}\n", TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_004() -> io::Result<()> {
        static TESTID: &str = "004";
        setup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?;
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                                   (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                    (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                    BUFWRITERFDBASE+4)?;
        let mut rfile = BufWriter::new(rfile);
        assert_eq!(rfile.get_ref().as_raw_fd(), BUFWRITERFDBASE+4);
        rfile.flush();
        rfile.write_fmt(format_args!("Hello World: {}\n", TESTID)).unwrap();
        rfile.flush();
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World: {}\n", TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    // #[test]
    // fn report_test_005() -> io::Result<()> {
    //     static TESTID: &str = "global";
    //     assert_eq!(TRACER.file.get_ref().as_raw_fd(), BUFWRITERFDBASE+5);
    //     // (&(*TRACER).file).write_fmt(format_args!("Hello World: {}\n", 1)).unwrap();
    //     write!(&(*TRACER).file, "Hello World: global\n");
    //     TRACER.file.flush();
    //     assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), "Hello World: global\n");
    //     Ok(())
    // }

    #[test]
    fn report_test_006() -> io::Result<()> {
        static TESTID: &str = "006";
        setup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?;
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                                   (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                    (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                    -1)?;
        let message = format!("Hello World: {}\n", TESTID);
        rfile.write(message.as_bytes()).unwrap();
        rfile.flush();
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                                   (O_RDONLY|O_CLOEXEC) as i32,
                                    (0) as i32,
                                    -1)?;
        let mut buf = [0; 17];
        rfile.read(&mut buf);
        assert_eq!(format!("Hello World: {}\n", TESTID), str::from_utf8(&buf).unwrap());
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }
}