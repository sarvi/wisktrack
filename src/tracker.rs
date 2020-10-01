
use std::mem;
use std::{env, ptr};
use std::ffi::{CStr, OsString};
use std::os::unix::io::{FromRawFd,AsRawFd,IntoRawFd};
// use std::sync::Mutex;
use std::io::prelude::*;
use std::io::{Error, Read, Result, Write};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions, metadata, create_dir_all};
use std::string::String;
use std::env::var;
use std::process;
use std::collections::HashMap;
use libc::{c_char,c_int, O_CREAT, O_APPEND, O_LARGEFILE, O_CLOEXEC, AT_FDCWD, SYS_open};
use nix::unistd::dup2;
use uuid::Uuid;
// use serde::{Serialize, Deserialize};
use base_62;
use filepath::FilePath;
use tracing::dispatcher::{with_default, Dispatch};
use tracing_appender::non_blocking::WorkerGuard;
use tracing::{Level, event, };
// use redhook::ld_preload::make_dispatch;
use redhook::debug;
use backtrace::Backtrace;
use crate::utils;

pub const DEBUGMODE:bool = true;

#[macro_export]
macro_rules! setdebugmode {
    ($operation:expr) => {
            if DEBUGMODE {
                if !std::env::var("RUST_BACKTRACE").is_ok() {
                    std::env::set_var("RUST_BACKTRACE", "1");
                    std::env::set_var("RUST_DEBUG", $operation);
                }
                // assert_eq!(std::env::var("RUST_BACKTRACE").is_ok(), true, "Command:  {} : {} : {}\n{:?}",
                //            $operation, TRACKER.uuid, TRACKER.cmdline.join(" "),Backtrace::new());
            }
    };
}

const TRACKERFD: c_int = 800;

const SENDLIMIT: usize = 4094;
// const SENDLIMIT: usize = 100;

const WISK_FIELDS: &'static [&'static str] = &["WISK_TRACE", "WISK_TRACK", "WISK_PUUID", "WISK_WSROOT",
                                               "WISK_CONFIG", "LD_PRELOAD", "RUST_BACKTRACE", "LD_DEBUG"];

pub struct Tracker {
    pub file: File,
    pub fd: i32,
}


lazy_static! {
    pub static ref CWD : String = {
        let cwdostr = env::current_dir().unwrap().into_os_string();
        let mut rv = cwdostr.into_string().unwrap();
        // debug(format_args!("CWD: {}\n", rv.as_str()));
        rv
    };
    pub static ref WSROOT : String = {
        let rv = match env::var("WISK_WSROOT") {
            Ok(mut wsroot) => {
                if wsroot.is_empty() {
                    wsroot.push_str(CWD.as_str());
                }
                if !Path::new(&wsroot).exists() {
                    create_dir_all(&wsroot).unwrap();
                }
                wsroot
            },
            Err(_) => CWD.to_owned(),
        };
        // debug(format_args!("WSROOT: {}\n", rv.as_str()));
        rv
    };
    pub static ref PUUID:String = {
        match env::var("WISK_PUUID") {
            Ok(uuid) => uuid,
            Err(_) => String::from("XXXXXXXXXXXXXXXXXXXXXX")
        }
    };
    pub static ref UUID : String = format!("{}", base_62::encode(Uuid::new_v4().as_bytes()));
    pub static ref PID : String = process::id().to_string();

    pub static ref WISKTRACK:String = {
        // debug(format_args!("Here\n"));        
        let mut fname:String = match var("WISK_TRACK") {
            Ok(v) =>  {
                if v.is_empty() {
                    String::from(format!("{}/track.file", WSROOT.as_str()))
                } else {
                    v
                }
            },
            Err(_) => {
                // debug(format_args!("WISK_TRACK is missing\n"));
                String::from(format!("{}/track.file", WSROOT.as_str()))
            },
        };
        if !fname.ends_with(".file")  {
            let t = format!("/wisktrack.{}", &UUID.as_str());
            fname.push_str(&t);
            // debug(format_args!("Updated Trackfile: {}\n", &fname));
        }
        if !fname.starts_with("/") {
            fname.insert_str(0, "/");
            fname.insert_str(0, WSROOT.as_str());
        }
        let p = Path::new(&fname);
        if !p.parent().unwrap().exists() {
            debug(format_args!("parent: {:?}", p.parent().unwrap()));
            create_dir_all(p.parent().unwrap()).unwrap();
        }
        // debug(format_args!("WISKTRACK: {}\n", fname.as_str()));
        fname
    };
    pub static ref WISKMAP : Vec<(String, String)> = {
        // debug(format_args!("WISKMAP\n"));
        let mut wiskmap: Vec<(String, String)> = utils::envextractwisk(WISK_FIELDS.to_vec());
        wiskmap.push(("WISK_PUUID".to_string(), UUID.to_string()));
        // debug(format_args!("WISKMAP: {:?}", &wiskmap));
        wiskmap
    };
    pub static ref ENV : HashMap<String, String> = {
        let mut map = HashMap::new();
        for (key, val) in env::vars_os() {
            // Use pattern bindings instead of testing .is_some() followed by .unwrap()
            if let Ok(k) = key.into_string() {
                map.insert(k, val.into_string().unwrap());
            }
        }
        map
    };
    pub static ref CMDLINE: Vec<String> = std::env::args().map(|x| x).collect();
    pub static ref TRACKER : Tracker = Tracker::new();
    // pub static ref WISKFDS : Vec<i32> = {
    //     let x= vec!();
    //     x.push(TRACKER.)
    // }
}

pub fn initialize_statics() {
    lazy_static::initialize(&WSROOT);
    lazy_static::initialize(&PUUID);
    lazy_static::initialize(&UUID);
    lazy_static::initialize(&PID);
    lazy_static::initialize(&CWD);
    lazy_static::initialize(&WISKTRACK);
    lazy_static::initialize(&WISKMAP);
    lazy_static::initialize(&ENV);
    lazy_static::initialize(&CMDLINE);
    lazy_static::initialize(&TRACKER);
}


fn fd2path (fd : c_int ) -> PathBuf {
    // let f = unsafe { File::from_raw_fd(fd) };
    // let fp = f.path().unwrap();
    // // println!("{}",fp.as_path().to_str().unwrap());
    // f.into_raw_fd();
    // fp
    let mut rv = PathBuf::new();
    rv.push("some looooooooooooooooooooooooong dummy path");
    rv
}

fn path2str (path: PathBuf) -> String {
    let pathostr = path.into_os_string();
    pathostr.into_string().unwrap()
}

unsafe fn cstr2str<'a>(ptr: *const c_char) -> &'a str {
    CStr::from_ptr(ptr).to_str().unwrap()
}

unsafe fn pathget(ipath: *const libc::c_char) -> String {
    String::from(cstr2str(ipath))
}

unsafe fn pathgetabs(ipath: *const libc::c_char, fd: c_int) -> String {
    let ipath = pathget(ipath);
    let ipath = if ipath.starts_with("/") {
        ipath
    } else {
        let mut dirpath: PathBuf;
        if fd >= 0 {
            dirpath = PathBuf::from(&CWD.as_str());
            // dirpath = fd2path(fd);
        } else if fd == AT_FDCWD {
            dirpath = PathBuf::from(&CWD.as_str());
        } else {
            dirpath = PathBuf::from(&CWD.as_str());
        }
        dirpath.push(ipath);
        path2str(dirpath)
    };
    if ipath.starts_with(&WSROOT.as_str()) {
        ipath.replacen(&WSROOT.as_str(),"",1)
    } else {
        ipath
    }
}

// this is a macro just so it can handle any integer type easily.
macro_rules! check_err {
    ( $e:expr ) => {
        match $e {
            -1 => Err(Error::last_os_error()),
            other => Ok(other)
        }
    }
}

impl Tracker {
    pub fn new() -> Tracker {
        // debug(format_args!("Tracker Initializer\n"));
        let f = OpenOptions::new().create(true).append(true).open(&*WISKTRACK).unwrap();
        let tempfd = f.into_raw_fd();
        let fd = dup2(tempfd, TRACKERFD).unwrap();
        let tracker = Tracker {
            file :  unsafe { FromRawFd::from_raw_fd(fd) },
            fd : fd,
        };
        // let tracker = Tracker { file : utils::internal_open(&*WISKTRACK, O_CREAT|O_APPEND|O_LARGEFILE|O_CLOEXEC)};
        // debug(format_args!("Tracker File: {:?}\n", tracker.file));
        // debug(format_args!("Tracker Initializer: Done\n"));
        tracker
    }

    pub fn initialize(&self) {
        // debug(format_args!("Tracker Initializer\n"));
        event!(Level::INFO, "Tracker Initialization:\n");
        setdebugmode!("program_start");
        // debug(format_args!("Tracker File: {:?}\n{}\n", self.file, serde_json::to_string(&CMDLINE.to_vec()).unwrap()));

        (&self.file).write_all(format!("{} CALLS {}\n", &PUUID.as_str(), serde_json::to_string(&UUID.as_str()).unwrap()).as_bytes()).unwrap();
        (&self.file).write_all(format!("{} CMDLINE {}\n", &UUID.as_str(), serde_json::to_string(&CMDLINE.to_vec()).unwrap()).as_bytes()).unwrap();
        (&self.file).write_all(format!("{} WISKENV {}\n", &UUID.as_str(), serde_json::to_string(&ENV.to_owned()).unwrap()).as_bytes()).unwrap();
        (&self.file).write_all(format!("{} PID {}\n", UUID.as_str(), serde_json::to_string(&PID.to_owned()).unwrap()).as_bytes()).unwrap();
        (&self.file).write_all(format!("{} CWD {}\n", UUID.as_str(), serde_json::to_string(&CWD.to_owned()).unwrap()).as_bytes()).unwrap();
        (&self.file).write_all(format!("{} WSROOT {}\n", UUID.as_str(), serde_json::to_string(&WSROOT.to_owned()).unwrap()).as_bytes()).unwrap();
        // event!(Level::INFO, "Tracker Initialization Complete: {} CALLS {}, WISKENV: {}, CMD: {}",
        //         tracker.puuid, serde_json::to_string(&tracker.uuid).unwrap(), serde_json::to_string(&tracker.wiskfields).unwrap(),
        //         &tracker.cmdline.join(" "));
        // debug(format_args!("Tracker Initializer Complete\n"));
    }
    
    pub fn report(self: &Self, op : &str, value: &str) {
        let mut minlen: usize = &UUID.as_str().len() + op.len() + 2;
        let mut availen: usize = SENDLIMIT - minlen;
        let mut lenleft = value.len();
        let mut ind = 0;
        let mut contin = "";
        static mut REOPENCOUNT:i32 =0;

        // println!("{} op={} value={}", self.uuid, op, value);
        while lenleft != 0 {
            let max = if lenleft > availen {lenleft = lenleft - availen; ind + availen } 
                    else { let x=lenleft; lenleft = 0; ind + x };
            // println!("minlen={} valeft={} ind={} max={}\n{} {} {}", minlen, lenleft, ind, max,
            //         self.uuid, op, contin);
            // debug(format_args!("Tracker Write : {:?}", &self.file));
            if let Err(e) = (&self.file).write_all(format!("{} {} {}{}\n", &UUID.as_str(), op, contin, &value[ind..max]).as_bytes()) {
                // debug(format_args!("Tracker Write Error(reopening): {}", e));
                unsafe {
                    REOPENCOUNT = REOPENCOUNT + 1;
                    if REOPENCOUNT > 1 {
                        debug(format_args!("Tracker Write Count:{} File: {:?} Error(reopening): {}\nPID: {}\nCMDLIE: {:?}\n, Traceback: {:?}",
                        REOPENCOUNT, self.file, e, std::process::id(),
                        serde_json::to_string(&CMDLINE.to_vec()).unwrap(),
                        Backtrace::new()));
                    }
                }
                let tracker = Tracker::new();
                tracker.report(op, value);
            }
            contin = "*";
            ind = max ;
            minlen = &UUID.as_str().len() + op.len() + 2 + 1;
            availen = SENDLIMIT - minlen;
        };
        (&self.file).flush().unwrap();
    }
    
    // pub fn reportenv(self: &Self,: env====) {
    //     // if let Ops::ENV(ref mut map) = op {
    //     //     for (key, val) in env::vars_os() {
    //     //         if let (Ok(k), Ok(v)) = (key.into_string(), val.into_string()) {
    //     //             map.append(vec!(k, v));
    //     //         }
    //     //     }
    //     // }
    //     let serialized = serde_json::to_string(&op).unwrap();
    //     println!("serialized = {:?}", serialized);
    //     match op {
    //         LINK(values) => self.report("LINKS", serde_json::to_string(&values).unwrap()),
    //         CHMOD(values) => self.report("CHMODS", serde_json::to_string(&values).unwrap()),
    //         _(values) => self.report("UNKNOWN", serde_json::to_string(&values).unwrap())
    //     }
    //     // self.report("ENV", &serialized);
    // }

    pub unsafe fn reportreadlink(self: &Self, path: *const libc::c_char) {
        let args = (&pathgetabs(path,-1), );
        self.report("READLINK", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportsymlink(self: &Self, target: *const libc::c_char, linkpath: *const libc::c_char) {
        let args = (cstr2str(target), pathgetabs(linkpath,-1));
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportsymlinkat(self: &Self, target: *const libc::c_char, newdirfd: libc::c_int, linkpath: *const libc::c_char) {
        let args = (cstr2str(target), pathgetabs(linkpath, newdirfd));
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportlink(self: &Self, oldpath: *const c_char, newpath: *const c_char) {
        let args = (&pathgetabs(oldpath,-1), &pathgetabs(newpath, -1));
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportlinkat(self: &Self, olddirfd: c_int, oldpath: *const c_char, newdirfd: c_int, newpath: *const c_char, flags: c_int) {
        let args = (&pathgetabs(oldpath, olddirfd), &pathgetabs(newpath, newdirfd), flags);
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportunlink(self: &Self, pathname: *const libc::c_char) {
        let args = (pathgetabs(pathname,-1),);
        self.report("UNLINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportunlinkat(self: &Self, dirfd: libc::c_int, pathname: *const libc::c_char, flags: libc::c_int) {
        let args = (pathgetabs(pathname,dirfd),flags);
        self.report("UNLINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportchmod(self: &Self, pathname: *const libc::c_char, mode: libc::mode_t) {
        let args = (pathgetabs(pathname,-1), mode);
        self.report("CHMODS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportfchmod(self: &Self, fd: libc::c_int, mode: libc::mode_t) {
        let args = (path2str(fd2path(fd)),mode);
        self.report("CHMODS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportfchmodat(self: &Self, dirfd: libc::c_int, pathname: *const libc::c_char, mode: libc::mode_t, flags: libc::c_int) {
        let args = (pathgetabs(pathname,dirfd),mode,flags);
        self.report("CHMODS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportcreat(self: &Self, pathname: *const libc::c_char, mode: libc::mode_t) {
        let args = (pathgetabs(pathname,-1), mode);
        self.report("WRITES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportfopen(self: &Self, name: *const libc::c_char, mode: *const libc::c_char) {
        let args = (pathgetabs(name,-1), cstr2str(mode));
        if args.1.contains("w") || args.1.contains("a") {
            self.report("WRITES", &serde_json::to_string(&args).unwrap());
            if args.1.contains("+") {
                self.report("READS", &serde_json::to_string(&args).unwrap());
            }
        } else {
            self.report("READS", &serde_json::to_string(&args).unwrap());
            if args.1.contains("+") {
                self.report("WRITES", &serde_json::to_string(&args).unwrap());
            }
        }
    }

    pub unsafe fn reportopen(self: &Self, pathname: *const libc::c_char, flags: libc::c_int, mode: libc::c_int) {
        if (flags | O_CREAT) == O_CREAT {
            let args = (pathgetabs(pathname,-1), flags, mode);
            self.report("OPEN", &serde_json::to_string(&args).unwrap());
        } else {
            let args = (pathgetabs(pathname,-1), flags);
            self.report("OPEN", &serde_json::to_string(&args).unwrap());
        }
    }

    pub unsafe fn reportexecv(self: &Self, path: *const libc::c_char, argv: *const *const libc::c_char) {
        let mut vargv: Vec<&str> = vec![];
        for i in 0 .. {
            let argptr: *const c_char = *(argv.offset(i));
            if argptr != ptr::null() {
                vargv.push(cstr2str(argptr))
            } else {
                break;
            }
        }
        let args = (pathgetabs(path,-1), vargv);
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportexecvpe(self: &Self, path: *const libc::c_char, argv: *const *const libc::c_char,
                                env: *const *const libc::c_char) {
        let mut vargv: Vec<&str> = vec![];
        let mut venv: Vec<Vec<&str>> = vec![];
        for i in 0 .. {
            let argptr: *const c_char = *(argv.offset(i));
            if argptr != ptr::null() {
                vargv.push(cstr2str(argptr))
            } else {
                break;
            }
        }
        for i in 0 .. {
            let argptr: *const c_char = *(env.offset(i));
            if argptr != ptr::null() {
                venv.push(cstr2str(argptr).splitn(2,"=").collect());
            } else {
                break;
            }
        }
        let args = (pathgetabs(path,-1), vargv, venv);
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportexecvp(self: &Self, path: *const libc::c_char, argv: *const *const libc::c_char) {
        let mut vargv: Vec<&str> = vec![];
        for i in 0 .. {
            let argptr: *const c_char = *(argv.offset(i));
            if argptr != ptr::null() {
                vargv.push(cstr2str(argptr))
            } else {
                break;
            }
        }
        let args = (pathgetabs(path,-1), vargv);
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportpopen(self: &Self, command: *const libc::c_char, ctype: *const libc::c_char) {
        let args = ("/bin/sh", cstr2str(command), cstr2str(ctype));
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportsystem(self: &Self, command: *const libc::c_char) {
        let args = ("/bin/sh", cstr2str(command));
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

}

// thread_local! {
//     #[allow(nonstandard_style)]
//     pub static MY_DISPATCH_initialized: ::core::cell::Cell<bool> = false.into();
// }

// lazy_static! {
//     pub static ref TRACKER : Tracker = Tracker::init();
// }

// thread_local! {
//     pub static MY_DISPATCH: (bool, Dispatch, WorkerGuard) = {
//         // debug(format_args!("Trace Initialize\n"));
//         let ret = make_dispatch("WISK_TRACE");
//         if ret.0 {
//             with_default(&ret.1, || {
//                 // lazy_static::initialize(&TRACKER);
//                 event!(Level::INFO, "Tracker Initialization Complete: {} CALLS {}, WISKENV: {}, CMD: {}",
//                 &TRACKER.puuid, serde_json::to_string(&TRACKER.uuid).unwrap(), serde_json::to_string(&TRACKER.wiskfields).unwrap(),
//                 &TRACKER.cmdline.join(" "));
//             });
//         } else {
//             debug(format_args!("something is wrooooooooooooooooooooooooooooooooooooooong!!!!!"));
//             // lazy_static::initialize(&TRACKER);
//             event!(Level::INFO, "Tracker Initialization Complete: {} CALLS {}, WISKENV: {}, CMD: {}",
//             &TRACKER.puuid, serde_json::to_string(&TRACKER.uuid).unwrap(), serde_json::to_string(&TRACKER.wiskfields).unwrap(),
//             &TRACKER.cmdline.join(" "));
//     }
//         MY_DISPATCH_initialized.with(|it| it.set(true));
//         // debug(format_args!("Trace Initialize: Complete\n"));
//         ret
//     };
// }



// #[cfg(test)]
// mod report_tests {
//     use std::io;
//     use super::*;

//     #[test]
//     fn report_test_000() -> io::Result<()> {
//         TRACKER.report("test_000", "");
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(!buffer.contains(&format!("\n\n")));
//         assert!(!buffer.contains(&format!("{} test_000\n", TRACKER.uuid)));
//         Ok(())
//     }

//     #[test]
//     fn report_test_001() -> io::Result<()> {
//         TRACKER.report("test_001", "D");
//         println!("FileName: {}", TRACKER.filename);
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} test_001 D\n", TRACKER.uuid)));
//         Ok(())
//     }

//     #[test]
//     fn report_test_002() -> io::Result<()> {
//         TRACKER.report("test_002", &"D".repeat(SENDLIMIT-32));
//         println!("FileName: {}", TRACKER.filename);
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} test_002 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
//         Ok(())
//     }

//     #[test]
//     fn report_tests_003() -> io::Result<()> {
//         TRACKER.report("test_003", &"D".repeat(SENDLIMIT-31));
//         println!("FileName: {}", TRACKER.filename);
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} test_003 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
//         assert!(buffer.contains(&format!("{} test_003 *{}\n", TRACKER.uuid, &"D".repeat(1))));
//         Ok(())
//     }

//     #[test]
//     fn report_test_004() -> io::Result<()> {
//         TRACKER.report("test_004", &"D".repeat(SENDLIMIT*2-9));
//         println!("FileName: {}", TRACKER.filename);
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} test_004 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
//         assert!(buffer.contains(&format!("{} test_004 *{}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-33))));
//         Ok(())
//     }

//     #[test]
//     fn report_test_005() -> io::Result<()> {
//         TRACKER.report("test_005", &"D".repeat(SENDLIMIT*2-(32*2)));
//         println!("FileName: {}", TRACKER.filename);
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} test_005 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
//         assert!(buffer.contains(&format!("{} test_005 *{}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-33))));
//         assert!(buffer.contains(&format!("{} test_005 *{}\n", TRACKER.uuid, &"D".repeat(1))));
//         Ok(())
//     }
// }

// #[cfg(test)]
// mod reportop_tests {
//     use std::io;
//     use std::ffi::{CString};
//     use libc::{opendir, dirfd};
//     use super::*;

//     #[test]
//     fn test_link() -> io::Result<()> {
//         unsafe {
//             TRACKER.reportlink(CString::new("/a/b/c").unwrap().as_ptr(),CString::new("/x/y/link").unwrap().as_ptr());
//             TRACKER.reportlink(CString::new("/a/b/c").unwrap().as_ptr(),CString::new("x/y/link").unwrap().as_ptr());
//         }
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"/x/y/link\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"{}/x/y/link\"]\n", TRACKER.uuid, TRACKER.cwd)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_linkat() -> io::Result<()> {
//         unsafe {
//             let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
//             TRACKER.reportlinkat(fd, CString::new("/a/b/c").unwrap().as_ptr(),fd, CString::new("/x/y/linkat").unwrap().as_ptr(), 300);
//             TRACKER.reportlinkat(fd, CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/linkat").unwrap().as_ptr(), 300);
//         };
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"/x/y/linkat\",300]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} LINKS [\"/tmp/a/b/c\",\"/tmp/x/y/linkat\",300]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_symlink() -> io::Result<()> {
//         unsafe {
//             TRACKER.reportsymlink(CString::new("/a/b/c").unwrap().as_ptr(),CString::new("/x/y/symlink").unwrap().as_ptr());
//             TRACKER.reportsymlink(CString::new("/a/b/c").unwrap().as_ptr(),CString::new("x/y/symlink").unwrap().as_ptr());
//         }
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"/x/y/symlink\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"{}/x/y/symlink\"]\n", TRACKER.uuid, TRACKER.cwd)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_symlinkat() -> io::Result<()> {
//         unsafe {
//             let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
//             TRACKER.reportsymlinkat(CString::new("/a/b/c").unwrap().as_ptr(),fd, CString::new("/x/y/symlinkat").unwrap().as_ptr());
//             TRACKER.reportsymlinkat(CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/symlinkat").unwrap().as_ptr());
//         };
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"/x/y/symlinkat\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} LINKS [\"a/b/c\",\"/tmp/x/y/symlinkat\"]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_unlink() -> io::Result<()> {
//         unsafe {
//             TRACKER.reportunlink(CString::new("/a/b/unlink").unwrap().as_ptr());
//         }
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} UNLINKS [\"/a/b/unlink\"]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_unlinkat() -> io::Result<()> {
//         unsafe {
//             let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
//             TRACKER.reportunlinkat(fd, CString::new("/a/b/unlinkat").unwrap().as_ptr(),300);
//             TRACKER.reportunlinkat(fd, CString::new("a/b/unlinkat").unwrap().as_ptr(),300);
//         };
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} UNLINKS [\"/a/b/unlinkat\",300]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} UNLINKS [\"/tmp/a/b/unlinkat\",300]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_chmod() -> io::Result<()> {
//         unsafe {
//             TRACKER.reportchmod(CString::new("/a/b/chmod").unwrap().as_ptr(), 0);
//         }
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} CHMODS [\"/a/b/chmod\",0]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_fchmod() -> io::Result<()> {
//         unsafe {
//             let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
//             TRACKER.reportfchmod(fd,0);
//         };
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} CHMODS [\"/tmp\",0]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_fchmodat() -> io::Result<()> {
//         unsafe {
//             let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
//             TRACKER.reportfchmodat(fd, CString::new("/a/b/fchmodat").unwrap().as_ptr(),0,0);
//             TRACKER.reportfchmodat(fd, CString::new("a/b/fchmodat").unwrap().as_ptr(),0,0);
//         };
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} CHMODS [\"/a/b/fchmodat\",0,0]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} CHMODS [\"/tmp/a/b/fchmodat\",0,0]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_creat() -> io::Result<()> {
//         unsafe {
//             TRACKER.reportcreat(CString::new("/a/b/creat").unwrap().as_ptr(),0);
//             TRACKER.reportcreat(CString::new("a/b/creat").unwrap().as_ptr(),0);
//         };
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} WRITES [\"/a/b/creat\",0]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} WRITES [\"{}/a/b/creat\",0]\n", TRACKER.uuid, TRACKER.cwd)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_fopen() -> io::Result<()> {
//         unsafe {
//             TRACKER.reportfopen(CString::new("/a/b/reads").unwrap().as_ptr(),CString::new("r").unwrap().as_ptr());
//             TRACKER.reportfopen(CString::new("/a/b/readsplus").unwrap().as_ptr(),CString::new("r+").unwrap().as_ptr());
//             TRACKER.reportfopen(CString::new("/a/b/writes").unwrap().as_ptr(),CString::new("w").unwrap().as_ptr());
//             TRACKER.reportfopen(CString::new("/a/b/writesplus").unwrap().as_ptr(),CString::new("w+").unwrap().as_ptr());
//             TRACKER.reportfopen(CString::new("/a/b/appends").unwrap().as_ptr(),CString::new("a").unwrap().as_ptr());
//             TRACKER.reportfopen(CString::new("/a/b/appendsplus").unwrap().as_ptr(),CString::new("a+").unwrap().as_ptr());
//         }
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} READS [\"/a/b/reads\",\"r\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} READS [\"/a/b/readsplus\",\"r+\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} WRITES [\"/a/b/readsplus\",\"r+\"]\n", TRACKER.uuid)));

//         assert!(buffer.contains(&format!("{} WRITES [\"/a/b/writes\",\"w\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} WRITES [\"/a/b/writesplus\",\"w+\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} READS [\"/a/b/writesplus\",\"w+\"]\n", TRACKER.uuid)));


//         assert!(buffer.contains(&format!("{} WRITES [\"/a/b/appends\",\"a\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} WRITES [\"/a/b/appendsplus\",\"a+\"]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} READS [\"/a/b/appendsplus\",\"a+\"]\n", TRACKER.uuid)));

//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_execv() -> io::Result<()> {
//         let argv = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];
//         let cstr_argv: Vec<_> = argv.iter()
//                                     .map(|arg| CString::new(arg.as_str()).unwrap())
//                                     .collect();
//         let mut p_argv: Vec<_> = cstr_argv.iter() // do NOT into_iter()
//                                           .map(|arg| arg.as_ptr())
//                                           .collect();
//         p_argv.push(std::ptr::null());
//         unsafe {
//             TRACKER.reportexecv(CString::new("/a/b/execv").unwrap().as_ptr(), p_argv.as_ptr());
//             TRACKER.reportexecv(CString::new("a/b/execv").unwrap().as_ptr(), p_argv.as_ptr());
//         }
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} EXECUTES [\"/a/b/execv\",[\"arg1\",\"arg2\",\"arg3\"]]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} EXECUTES [\"{}/a/b/execv\",[\"arg1\",\"arg2\",\"arg3\"]]\n", TRACKER.uuid, TRACKER.cwd)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_execvpe() -> io::Result<()> {
//         let args = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];
//         let env = vec!["A=B".to_string(), "C=D".to_string(), "E=F".to_string()];
//         let cstr_args: Vec<_> = args.iter()
//                                     .map(|arg| CString::new(arg.as_str()).unwrap())
//                                     .collect();
//         let mut p_args: Vec<_> = cstr_args.iter() // do NOT into_iter()
//                                           .map(|arg| arg.as_ptr())
//                                           .collect();
//         p_args.push(std::ptr::null());
//         let cstr_env: Vec<_> = env.iter()
//                                     .map(|arg| CString::new(arg.as_str()).unwrap())
//                                     .collect();
//         let mut p_env: Vec<_> = cstr_env.iter() // do NOT into_iter()
//                                           .map(|arg| arg.as_ptr())
//                                           .collect();
//         p_env.push(std::ptr::null());
//         unsafe {
//             TRACKER.reportexecvpe(CString::new("/a/b/execvpe").unwrap().as_ptr(), p_args.as_ptr(), p_env.as_ptr());
//             TRACKER.reportexecvpe(CString::new("a/b/execvpe").unwrap().as_ptr(), p_args.as_ptr(), p_env.as_ptr());
//         }
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} EXECUTES [\"/a/b/execvpe\",[\"arg1\",\"arg2\",\"arg3\"],[[\"A\",\"B\"],[\"C\",\"D\"],[\"E\",\"F\"]]]\n", TRACKER.uuid)));
//         assert!(buffer.contains(&format!("{} EXECUTES [\"{}/a/b/execvpe\",[\"arg1\",\"arg2\",\"arg3\"],[[\"A\",\"B\"],[\"C\",\"D\"],[\"E\",\"F\"]]]\n", TRACKER.uuid, TRACKER.cwd)));
//         assert!(true);
//         Ok(())
//     }

//     #[test]
//     fn test_popen() -> io::Result<()> {
//         unsafe {
//             TRACKER.reportpopen(CString::new("echo \"something\"").unwrap().as_ptr(),
//                                 CString::new("ctype").unwrap().as_ptr());
//         };
//         let mut rfile = File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} EXECUTES [\"/bin/sh\",\"echo \\\"something\\\"\",\"ctype\"]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

// }