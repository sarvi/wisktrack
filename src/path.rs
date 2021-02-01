use std::ffi::{CStr, CString};use nix::fcntl::OFlag;
use std::io;
use std::{fs, fmt, error};
use std::path::{Path, PathBuf};
use std::cell::RefCell;
use libc::{c_char,c_int, PATH_MAX, SYS_readlink};
use tracing::{Level};
use redhook::debug;
use regex::{RegexSet};


pub fn readlink(link: &str) -> io::Result<String> {
    thread_local! {
        pub static BUFFER: RefCell<Vec<i8>> = RefCell::new(vec![0; PATH_MAX as usize]);
    }
    let pathname = CString::new(link).expect("Not a valid path string");
    BUFFER.with(|f| {
        let mut b = f.borrow_mut();
        unsafe {
            let size = libc::syscall(SYS_readlink, pathname.as_ptr(), (*b).as_ptr(), PATH_MAX as usize);
            if size <0 {
                return Err(io::Error::last_os_error());
            }
            b[size as usize] = 0;
            let x = CStr::from_ptr((*b).as_ptr()).to_owned();
            Ok(x.into_string().expect("WISK_ERROR: Invalid path returned by readlink"))
        }
    })
}

pub fn fd_to_pathstr(fd: c_int) -> io::Result<String> {
    readlink(format!("/proc/self/fd/{}", fd).as_str())
}

pub fn join(p1: &str, p2: &str) -> String {
    if p2.starts_with("/") {
        p2.to_owned()
    } else {
        let mut rv = String::from(p1);
        if !rv.ends_with("/") {
            rv.push_str("/");
        }
        rv.push_str(p2);
        rv
    }
}

pub fn normalize(p: &str) -> String {
    let mut v:Vec<&str> = vec!();
    let mut rv = String::new();
    for i in p.split("/") {
        if i == "." {
            continue;
        } else if i == ".." {
            if v.is_empty() || *v.last().unwrap() == ".." {
                v.push(i);
            } else {
                v.pop();
            }
        } else {
            if !i.is_empty() {
                v.push(i);
            }
        }
    }
    for i in v.iter() {
        if !rv.is_empty() || p.starts_with("/") {
            rv.push_str("/");
        }
        rv.push_str(i);
    }
    if rv.is_empty() {
        rv.push_str(".");
    }
    rv
}

pub fn is_match(file: &str, patterns: &RegexSet, cwd: &str) -> bool {
    // debug(format_args!("Matching Patters: {} with {:#?}\n", file, patterns));
    if !file.starts_with("/") && !cwd.is_empty()  {
        let absfile = join(cwd,file);
        let file = normalize(absfile.as_str());
        let rv = patterns.is_match(file.as_str());
        // debug(format_args!("Match: {} {}\n", file, rv));
        rv
    } else {
        let file = normalize(file);
        let rv = patterns.is_match(file.as_str());
        // debug(format_args!("Match: {} {}\n", file, rv));
        rv
    }
}


#[cfg(test)]
mod path_tests {
    use std::io;
    use super::*;

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("."), ".");
        assert_eq!(normalize("./"), ".");
        assert_eq!(normalize("./."), ".");
        assert_eq!(normalize("././"), ".");
        assert_eq!(normalize("../"), "..");
        assert_eq!(normalize("../.."), "../..");
        assert_eq!(normalize("../../"), "../..");
        assert_eq!(normalize("something/new.so"), "something/new.so");
        assert_eq!(normalize("./something/new.so"), "something/new.so");
        assert_eq!(normalize("../something/new.so"), "../something/new.so");
        assert_eq!(normalize("../../something/new.so"), "../../something/new.so");
        assert_eq!(normalize("something/./new.so"), "something/new.so");
        assert_eq!(normalize("something/././new.so"), "something/new.so");
        assert_eq!(normalize("something/extra/../new.so"), "something/new.so");
        assert_eq!(normalize("./something/extra/../new.so"), "something/new.so");
        assert_eq!(normalize("../something/extra/../new.so"), "../something/new.so");
        assert_eq!(normalize("../something//extra/../new.so"), "../something/new.so");
        assert_eq!(normalize("../../something/..//extra/../new.so"), "../../new.so");
        assert_eq!(normalize("../something//extra/../../new.so"), "../new.so");
        assert_eq!(normalize("../something//extra/../../../new.so"), "../../new.so");
        assert_eq!(normalize("something//last/"), "something/last");
        assert_eq!(normalize("something//last/."), "something/last");
        assert_eq!(normalize("something//last/../../.."), "..");
        assert_eq!(normalize("/something/new.so"), "/something/new.so");
        assert_eq!(normalize("/something////new.so"), "/something/new.so");
    }

    #[test]
    fn test_join() {
        assert_eq!(join(".", "something.so"), "./something.so");
        assert_eq!(join("./", "something.so"), "./something.so");
        assert_eq!(join(".//", "something.so"), ".//something.so");
        assert_eq!(join("./.", "something.so"), "././something.so");
        assert_eq!(join("././", "something.so"), "././something.so");
    }

    #[test]
    fn test_readlink() {
        assert!(!readlink("/proc/self").unwrap().is_empty());
    }

}