#![no_std]

use core::fmt;
use core::sync::atomic::Ordering;
use std::ffi::{CStr, CString};
use std::{io,cmp};
use std::process;
use std::{thread, time};
use std::io::Write;
use libc;
use backtrace::Backtrace;

use crate::common::{PUUID, UUID, WISKFDS};
use crate::path;
use crate::TRACER;

pub const READ_LIMIT: usize = libc::ssize_t::MAX as usize;
pub const JSON: bool = true;
pub const SIMPLE: bool = false;

const fn max_iov() -> usize {
    libc::UIO_MAXIOV as usize
}

#[derive(Debug)]
pub struct File {
    pub fd: i32,
}

impl File {
    pub fn open(filename: &str, flags: i32, mode: i32, specificfd: i32) -> io::Result<File> {
        // eprintln!("PID: {}, internal_open FLAGS: {}, File: {}",
        //           process::id(), flags, filename);
        cevent!(Level::INFO, "open(filename={}, flags={}, mode={}, specificfd={})",
                filename, flags, mode, specificfd);
        let fd = if specificfd >= 0 {
            let eflags = unsafe { libc::syscall(libc::SYS_fcntl, specificfd, libc::F_GETFD) } as libc::c_int;
            if eflags >= 0 {
                let fname = path::fd_to_pathstr(specificfd).unwrap();
                cevent!(Level::INFO, "checkingfdtoname(filename={}, specificfd={})", fname, specificfd);
                WISKFDS.write().unwrap().push(specificfd);
                let f = File {
                    fd: specificfd,
                };
                return Ok(f)
            }
            specificfd
        } else {
            -1
        };
        let filename = CString::new(filename).unwrap();
        cevent!(Level::INFO, "opening(filename={:?})", filename);
        let tempfd = unsafe {
            if mode != 0 {
                libc::syscall(libc::SYS_openat, libc::AT_FDCWD, filename.as_ptr(), flags, mode)
            } else {
                libc::syscall(libc::SYS_openat, libc::AT_FDCWD, filename.as_ptr(), flags)
            }
        } as i32;
        if tempfd < 0 {
            return Err(io::Error::last_os_error());
        }
        cevent!(Level::INFO, "opened(filename={:?})={}", filename, tempfd);
        let retfd = if specificfd > 0 && specificfd != tempfd {
            cevent!(Level::INFO, "Duplicating FD: {}, relocating to {}", tempfd, specificfd);
            let retfd = unsafe { libc::syscall(libc::SYS_dup3, tempfd, specificfd, flags & libc::O_CLOEXEC) } as i32;
            if retfd < 0 {
                errorexit!("Cannot dup3 fd {} to {}, flags: {}\n{}",
                        tempfd, specificfd, flags & libc::O_CLOEXEC, io::Error::last_os_error());
            }
            WISKFDS.write().unwrap().push(specificfd);
            cevent!(Level::INFO, "File Descriptor(Relocated): {} -> {}, File: {}",
                tempfd, fd, filename.to_string_lossy());
            cevent!(Level::INFO, "Closing FD: {}", tempfd);
            unsafe { libc::syscall(libc::SYS_close, tempfd) };
            retfd
        } else {
            cevent!(Level::INFO, "File Descriptor(Original): {}, File: {}",
                tempfd, filename.to_string_lossy());
            tempfd
        };
        // let eflags = unsafe { libc::syscall(libc::SYS_fcntl, retfd, libc::F_GETFD) } as libc::c_int;
        // if eflags < 0 {
        //     errorexit!("Error Validating FD: {} returned eflags: {}, File: {}\n{}",
        //                retfd, eflags, filename.to_string_lossy(), io::Error::last_os_error());
        // }
        // cevent!(Level::INFO, "fcntlfdcheck FD: {}, EFLAGS: {}", retfd, eflags);
        // if specificfd >= 0 && (eflags & libc::O_CLOEXEC) != 0 {
        //     errorexit!("Error O_CLOEXEC FD: {} returned eflasgs: {}, File: {}", retfd, eflags, filename.to_string_lossy());
        // }
        // let eflags = unsafe { libc::syscall(libc::SYS_fcntl, retfd, libc::F_GETFL) } as libc::c_int;
        // cevent!(Level::INFO, "fcntlfdcheck FD: {}, EFLAGS: {}", retfd, eflags);
        let f = File {
            fd: retfd,
        };
        cevent!(Level::INFO, "Created File Buffer {:?}", f);
        Ok(f)
    }

    pub fn clone(fd: i32, bufsize: usize) -> File {
        let f = File {
            fd: fd,
        };
        f
    }

    pub fn sanity_check(&self) {
        cevent!(Level::INFO, "Sanity Checking File: {:?}", self);
        let eflags = unsafe { libc::syscall(libc::SYS_fcntl, self.fd, libc::F_GETFD) } as libc::c_int;
        if eflags < 0 {
            errorexit!("File Sanity check failed");
        }
        cevent!(Level::INFO, "Sanity Checked File: {:?} = {}", self, eflags);
    }

    pub fn as_raw_fd(&self) -> i32 {
        self.fd
    }

    pub fn sync_all(&self) -> io::Result<()> {
        cevent!(Level::INFO, "sync_all: File: {:?}", self);

        let rv = unsafe { libc::syscall(libc::SYS_fsync, self.fd) } as i32;
        if rv < 0 {
            return Err(io::Error::last_os_error());
        } else {
            Ok(())
        }
    }

    pub fn file_size(&self) -> io::Result<usize> {
        let size = unsafe { libc::syscall(libc::SYS_lseek, self.fd, 0, libc::SEEK_END) } as usize;
        if size < 0 {
            return Err(io::Error::last_os_error());
        }
        let rv = unsafe { libc::syscall(libc::SYS_lseek, self.fd, 0, libc::SEEK_SET) } as usize;
        if rv < 0 {
            return Err(io::Error::last_os_error());
        }
        return Ok(size)
    }

}

pub trait WriteStr: std::io::Write {
    fn write_cstrptr(&mut self, p: *const i8, json: bool) -> io::Result<usize> where Self: std::fmt::Debug;

    fn write_str(&mut self, s: &str) -> io::Result<usize> {
        self.write(s.as_bytes())
    }

}

impl WriteStr for File {
    fn write_cstrptr(&mut self, mut p: *const i8, json: bool) -> io::Result<usize> where Self: std::fmt::Debug {
        cevent!(Level::INFO, "write_cstrptr: File {:?}", self);
        // let mut d: *mut i8 = unsafe { self.buffer.as_ptr().add(self.index) } as *mut i8;

        // while unsafe { *p } != b'\0' as i8 {
        //     unsafe { *d = *p };
        //     p =  unsafe { p.add(1) };
        //     d = unsafe { d.add(1) };
        //     self.index = self.index + 1;
        //     if self.index >= libc::PIPE_BUF {
        //         cevent!(Level::INFO, "write_cstrptr: File {:?}: END", self);
        //         self.index=0;
        //     }
        // }
        cevent!(Level::INFO, "write_cstrptr: File {:?}", self);
        Ok(0 as usize)
    }
}

impl WriteStr for &File {
    fn write_cstrptr(&mut self, mut p: *const i8, json: bool) -> io::Result<usize> where Self: std::fmt::Debug {
        cevent!(Level::INFO, "write_cstrptr: File {:?}", self);
        // let mut d: *mut i8 = self.buffer.as_ptr().add(self.index) as *mut i8;

        // while unsafe { *p } != b'\0' as i8 {
        //     unsafe { *d = *p };
        //     p =  unsafe { p.add(1) };
        //     d = unsafe { d.add(1) };
        //     self.as_mut() = self.index + 1;
        //     if self.index >= libc::PIPE_BUF {
        //         cevent!(Level::INFO, "write_cstrptr: File {:?}: END", self);
        //         *(self.as_mut()) = 0;
        //     }
        // }
        cevent!(Level::INFO, "write_cstrptr: File {:?}", self);
        Ok(0 as usize)
    }
}

impl std::io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let x = unsafe { libc::syscall(
                             libc::SYS_read,
                             self.fd,
                             buf.as_ptr() as usize,
                             cmp::min(buf.len(),READ_LIMIT)) };
        if x < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(x as usize)
    }

}

impl Drop for File {
    fn drop(&mut self) {
        cevent!(Level::INFO, "Drop Closing({:?})", self);
        unsafe { libc::syscall(libc::SYS_close, self.fd) };
    }
}

impl std::io::Write for File {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        cevent!(Level::INFO, "File::Write: {:?}", String::from_utf8_lossy(buf));
        let x = unsafe { libc::syscall(
                            libc::SYS_write,
                            self.fd,
                            buf.as_ptr() as usize,
                            buf.len()) };
        if x < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(x as usize)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.sync_all()
    }

}

impl std::io::Write for &File {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        cevent!(Level::INFO, "&File::Write {:?}", String::from_utf8_lossy(buf));
        let x = unsafe { libc::syscall(
                            libc::SYS_write,
                            self.fd,
                            buf.as_ptr() as usize,
                            buf.len()) };
        if x < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(x as usize)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.sync_all()
    }

}

// impl fmt::Write for File {
//     fn write_str(&mut self, s: &str) -> fmt::Result {
//         let raw_s = s.as_bytes();
//         match self.write(raw_s) {
//             Ok(x) => Ok(()),
//             Err(e) => Err(fmt::Error),
//         }
//         // Ok(())
//     }

//     fn write_fmt(&mut self, args: fmt::Arguments) -> Result<(), fmt::Error> {
//         cevent!(Level::INFO, "Write_fmt data: {:?}", Backtrace::new());
//         fmt::write(self, args)?;
//         // w.as_str().ok_or(fmt::Error)
//         Ok(())
//     }
// }

// impl fmt::Write for &File {
//     fn write_str(&mut self, s: &str) -> fmt::Result {
//         let raw_s = s.as_bytes();
//         match self.write(raw_s) {
//             Ok(x) => Ok(()),
//             Err(e) => Err(fmt::Error),
//         }
//     }

//     fn write_fmt(&mut self, args: fmt::Arguments) -> Result<(), fmt::Error> {
//         cevent!(Level::INFO, "Write_fmt data &File {:?}", Backtrace::new());
//         fmt::write(self, args)?;
//         // w.as_str().ok_or(fmt::Error)
//         Ok(())
//     }
// }

#[cfg(test)]
mod tests_write {
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

    static MODULENAME: &str = "write";
    const WRITEFDBASE:i32 = 100;

    pub struct TestTracer {
        pub file: File,
    }

    impl TestTracer {
        pub fn new() -> TestTracer {
            let f: File = File::open("/tmp/tests_write_dataglobal",
                                     (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                     (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                     WRITEFDBASE+5).unwrap();
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
                                    WRITEFDBASE+0)?;
        let message = format!("Hello World: {}{}\n", MODULENAME,TESTID);
        rfile.write(message.as_bytes()).unwrap();
        rfile.sync_all();
        assert_eq!(WRITEFDBASE+0, rfile.as_raw_fd());
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World: {}{}\n", MODULENAME, TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_001() -> io::Result<()> {
        static TESTID: &str = "001";
        setup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?;
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                                   (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                    (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                    WRITEFDBASE+1)?;
        let message = format!("Hello World: {}\n", TESTID);
        rfile.write(message.as_bytes()).unwrap();
        rfile.sync_all();
        assert_eq!(WRITEFDBASE+1, rfile.as_raw_fd());
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
        let message = format!("Hello World: {}\n", TESTID);
        // assert_eq!(3, rfile.as_raw_fd());
        rfile.write(message.as_bytes()).unwrap();
        rfile.sync_all();
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World: {}\n", TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_003() -> io::Result<()> {
        static TESTID: &str = "003";
        setup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?;
        let ofile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                               (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                               (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                               WRITEFDBASE+3)?;
        ofile.sync_all();
        let mut rfile = File::open(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID),
                               (O_CREAT|O_WRONLY|O_APPEND|O_LARGEFILE) as i32,
                               (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                               WRITEFDBASE+3)?;
        assert_eq!(rfile.as_raw_fd(), WRITEFDBASE+3);
        let message = format!("Hello World: {}\n", TESTID);
        rfile.write(message.as_bytes()).unwrap();
        rfile.sync_all();
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
                                    WRITEFDBASE+4)?;
        assert_eq!(rfile.as_raw_fd(), WRITEFDBASE+4);
        rfile.sync_all();
        rfile.write_fmt(format_args!("Hello World: {}\n", TESTID)).unwrap();
        rfile.sync_all();
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World: {}\n", TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_005() -> io::Result<()> {
        static TESTID: &str = "005";
        assert_eq!(TRACER.file.as_raw_fd(), WRITEFDBASE+5);
        // (&(*TRACER).file).write_fmt(format_args!("Hello World: {}\n", 1)).unwrap();
        write!(&(*TRACER).file, "Hello World: {}\n", TESTID);
        TRACER.file.sync_all();
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_dataglobal", MODULENAME)).unwrap(), format!("Hello World: {}\n", TESTID));
        Ok(())
    }

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
        rfile.sync_all();
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


#[cfg(test)]
mod tests_write_fmt {
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

    static MODULENAME: &str = "write_fmt";
    const WRITEFMTFDBASE:i32 = 200;

    pub struct TestTracer {
        pub file: File,
    }

    impl TestTracer {
        pub fn new() -> TestTracer {
            let f: File = File::open("/tmp/tests_write_fmt_dataglobal",
                                     (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                     (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                     WRITEFMTFDBASE+5).unwrap();
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
                                    WRITEFMTFDBASE+0)?;
        write!(rfile, "Hello World fmt: {}\n", TESTID);
        // rfile.write_fmt(format_args!("Hello World: {}\n", 1)).unwrap();
        rfile.sync_all();
        assert_eq!(WRITEFMTFDBASE+0, rfile.as_raw_fd());
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), format!("Hello World fmt: {}\n", TESTID));
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_001() -> io::Result<()> {
        static TESTID: &str = "001";
        write!(&(*TRACER).file, "Hello World fmt: {}\n", TESTID);
        // rfile.write_fmt(format_args!("Hello World: {}\n", 1)).unwrap();
        TRACER.file.sync_all();
        assert_eq!(WRITEFMTFDBASE+5, TRACER.file.as_raw_fd());
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_dataglobal", MODULENAME)).unwrap(), format!("Hello World fmt: {}\n", TESTID));
        Ok(())
    }

}

#[cfg(test)]
mod tests_write_cstrptr {
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

    static MODULENAME: &str = "writercstrptr";
    const WRITERCSTRPTRFDBASE:i32 = 300;

    pub struct TestTracer {
        pub file: File,
    }

    impl TestTracer {
        pub fn new() -> TestTracer {
            let f: File = File::open(&format!("/tmp/tests_{}_dataglobal", MODULENAME),
                                     (O_CREAT|O_WRONLY|O_TRUNC|O_LARGEFILE) as i32,
                                     (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32,
                                     WRITERCSTRPTRFDBASE+5).unwrap();
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
                                    WRITERCSTRPTRFDBASE+0)?;
        let cstr = CStr::from_bytes_with_nul(b"Hello World cstrptr: 0\0").unwrap();
        rfile.write_cstrptr(cstr.as_ptr(), JSON);
        // rfile.write_fmt(format_args!("Hello World: {}\n", 1)).unwrap();
        rfile.sync_all();
        assert_eq!(WRITERCSTRPTRFDBASE+0, rfile.as_raw_fd());
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID)).unwrap(), "Hello World cstrptr: 0\n");
        Ok(cleanup(&format!("/tmp/tests_{}_data{}", MODULENAME, TESTID))?)
    }

    #[test]
    fn report_test_001() -> io::Result<()> {
        static TESTID: &str = "001";
        let cstr = CStr::from_bytes_with_nul(b"Hello World cstrptr: 1\0").unwrap();
        (&(*TRACER).file).write_cstrptr(cstr.as_ptr(), JSON);
        // rfile.write_fmt(format_args!("Hello World: {}\n", 1)).unwrap();
        TRACER.file.sync_all();
        assert_eq!(WRITERCSTRPTRFDBASE+5, TRACER.file.as_raw_fd());
        assert_eq!(fs::read_to_string(&format!("/tmp/tests_{}_dataglobal", MODULENAME)).unwrap(), "Hello World cstrptr: 1\n");
        Ok(())
    }

}