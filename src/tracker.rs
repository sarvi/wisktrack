
use std::mem;
use std::sync::{Once, RwLock};
use std::{env, ptr, process, fs};
use std::ffi::{CStr, CString, OsString};
use std::os;
use std::os::unix::io::{FromRawFd,AsRawFd,IntoRawFd};
use std::io::prelude::*;
use std::io::{Error, Read, Write};
use std::path::{Path, PathBuf};
use std::string::String;
use std::collections::HashMap;
use libc::{c_char,c_int, O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_APPEND, O_LARGEFILE, O_CLOEXEC,
           AT_FDCWD, SYS_open, S_IRUSR, S_IWUSR, S_IRGRP, S_IWGRP};
use nix::unistd::dup2;
// use serde::{Serialize, Deserialize};
use base_62;
use filepath::FilePath;
use tracing::dispatcher::{with_default, Dispatch};
use tracing_appender::non_blocking::WorkerGuard;
use tracing::{Level};
// use redhook::ld_preload::make_dispatch;
use redhook::{debug, initialized};
use backtrace::Backtrace;
use string_template::Template;
use regex::{RegexSet, escape};
use crate::utils;
use crate::{errorexit, event, wiskassert};
use crate::path;
use crate::common::{WISKFDS, WISKTRACKFD, WISKTRACEFD, WISKTRACE, PUUID, UUID, PID};
use crate::tracer::{TRACER};
use crate::fs::{File, JSON, SIMPLE};
use crate::fs::WriteStr;
use crate::bufwriter;

pub const DEBUGMODE:bool = false;

#[macro_export]
macro_rules! setdebugmode {
    ($operation:expr) => {
            if DEBUGMODE {
                if !env::var("RUST_BACKTRACE").is_ok() {
                    env::set_var("RUST_BACKTRACE", "1");
                    env::set_var("RUST_DEBUG", $operation);
                }
                // assert_eq!(env::var("RUST_BACKTRACE").is_ok(), true, "Command:  {} : {} : {}\n{:?}",
                //            $operation, TRACKER.uuid, TRACKER.cmdline.join(" "),Backtrace::new());
            }
    };
}


#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Deserialize)]
pub struct Config {
    #[serde(default)]
    app64bitonly_patterns: Vec<String>,
}


const CONFIG_DEFAULTS: &str = "
---
x: 1
";

const SENDLIMIT: usize = 4094;
// const SENDLIMIT: usize = 100;

const WISK_FIELDS: &'static [&'static str] = &[
    "WISK_TRACE", "WISK_TRACK", "WISK_PUUID", "WISK_WSROOT", "WISK_CONFIG",
    "WISK_CONFIG", "LD_PRELOAD", "RUST_BACKTRACE", "LD_DEBUG"];

pub struct Tracker {
    pub file: File,
}


lazy_static! {
    pub static ref ORIGINAL_ENV: HashMap<String,String> = env::vars().collect();

    pub static ref LD_PRELOAD:String = {
        match ORIGINAL_ENV.get("LD_PRELOAD") {
            Some(ld_preload) => {
                let ld_preload = ld_preload.to_owned();
                let updated_ld_preload = ld_preload.replace("lib/libwisktrack.so",
                                                            "${LIB}/libwisktrack.so")
                                                   .replace("lib32/libwisktrack.so",
                                                            "${LIB}/libwisktrack.so")
                                                   .replace("lib64/libwisktrack.so",
                                                            "${LIB}/libwisktrack.so");
                if ld_preload != updated_ld_preload {
                    // eprintln!("before updating: {}", ld_preload);
                    env::set_var("LD_PRELOAD", updated_ld_preload.as_str());
                    // eprintln!("after updating: {}", env::var("LD_PRELOAD").unwrap());
                }
                // eprintln!("after updating: {}", env::var("LD_PRELOAD").unwrap());
                updated_ld_preload
            },
            None => errorexit!("Environmet variable LD_PRELOAD not found"),
        }
    };

    pub static ref CWD : String = {
        let rv = if let Ok(cwd) = env::current_dir() {
            let cwdostr = cwd.into_os_string();
            let rv = cwdostr.into_string().unwrap();
            rv
        } else {
            String::new()
        };
        // debug(format_args!("CWD: {}\n", rv.as_str()));
        rv
    };

    pub static ref WSROOT : String = {
        let rv = match ORIGINAL_ENV.get("WISK_WSROOT") {
            Some(wsroot) => {
                let mut wsroot = wsroot.to_owned();
                if wsroot.is_empty() {
                    errorexit!("WISK_WSROOT is empty. MUST be set to point to the root of the build workspace. Curret Value: {}",
                              wsroot);
                }
                if !Path::new(&wsroot).exists() {
                    fs::create_dir_all(&wsroot).unwrap();
                }
                wsroot
            },
            None => {
                errorexit!("WISK_ERROR: WISK_WSROOT missig. MUST be set point to the root of the build workspace.");
            },
        };
        // debug(format_args!("WSROOT: {}\n", rv.as_str()));
        rv
    };

    pub static ref WSROOT_BASE : String = {
        let mut x = WSROOT.to_owned();
        x.push_str("/");
        x
    };

    pub static ref CONFIG : Config = {
        cevent!(Level::INFO, "Config Reading....");
        let rv : Result<Config,String> = utils::read_config(WSROOT.as_str(), "wisktrack.ini");
        let x = match rv {
            Ok(config) => { config }
            Err(e) => {
                errorexit!("WISK_ERROR: Cannnot find wisktrack.ini under project. Reading Default.\nError: {}", e);
                let rv : Result<Config, serde_yaml::Error> = serde_yaml::from_str(CONFIG_DEFAULTS);
                match rv {
                    Ok(config) =>  config,
                    Err(e) => {
                        errorexit!("WISK_ERROR: {}", e);
                    }
                }
            }
        };
        cevent!(Level::INFO, "CONFIG Reading....Done");
        x
    };

    pub static ref WISKTRACK:String = {
        // debug(format_args!("Here\n"));        
        let mut fname:String = match env::var("WISK_TRACK") {
            Ok(v) =>  {
                if v.is_empty() {
                    String::from(format!("{}/wisktrack.file", WSROOT.as_str()))
                } else {
                    v
                }
            },
            Err(_) => {
                // debug(format_args!("WISK_TRACK is missing\n"));
                String::from(format!("{}/wisktrack.file", WSROOT.as_str()))
            },
        };
        if !fname.ends_with(".file")  {
            let t = format!("/track.{}", &UUID.as_str());
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
            fs::create_dir_all(p.parent().unwrap()).unwrap();
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

    pub static ref MAPFIELDS: Vec<(String,String)> = {
        let mut v: Vec<(String,String)> = [
            ("LD_PRELOAD", LD_PRELOAD.as_str()),
            ("CWD", CWD.as_str()),
            ("WSROOT", WSROOT.as_str()),
            ("PUUID", PUUID.as_str()),
            ("UUID", UUID.as_str()),
            ("PID", PID.as_str()),
            ("WISKTRACK", WISKTRACK.as_str()),
        ].iter().map(|i| (i.0.to_owned(), i.1.to_owned())).collect();
        for n in 0..v.len() {
            let mut x=v[n].0.to_owned();
            x.insert_str(0, "RE_");
            v.push((x,escape(v[n].1.as_str())))
        }
        // eprintln!("MAPFIELDS: {:?}", v);
        v
    };

    pub static ref TEMPLATEMAP: HashMap<&'static str,&'static str> = {
        let mut v: Vec<(&str, &str)> = vec!();
        for i in MAPFIELDS.iter() {
            v.push((&i.0,i.1.as_str()));
        }
        let templmap:HashMap<&'static str,&'static str> = v.iter().cloned().collect();
        // eprintln!("TEMPLATEMAP: {:#?}", templmap);

        templmap
    };

    pub static ref  APP64BITONLY_PATTERNS: RegexSet = {
        event!(Level::INFO, "APP64BITONLY_PATTERNS Reading....");
        let p: Vec<String> = if !CONFIG.app64bitonly_patterns.is_empty() {
            CONFIG.app64bitonly_patterns.iter().map(|v| {
                if v.starts_with("^") {
                    render(v,&TEMPLATEMAP)
                } else {
                    let mut x = "^".to_owned();
                    x.push_str("{{RE_WSROOT}}");
                    x.push_str("/");
                    x.push_str(v);
                    // eprintln!("REGEX: {}", x.as_str());
                    render(x.as_str(),&TEMPLATEMAP)
                }
            }).collect()
        } else {
            vec!("^NOMATCH/.*$".to_owned())
        };
        // event!(Level::INFO,"p: {:?}", p);
        let x = RegexSet::new(&p).unwrap_or_else(|e| {
            errorexit!("WISK_ERROR: Error compiling list of regex in app_64bitonly_match: {:?}", e);
        });
        event!(Level::INFO, "APP64BITONLY_PATTERNS Reading....Done");
        x
    };
}

pub fn initialize_constructor_statics() {
    lazy_static::initialize(&WISKFDS);
    lazy_static::initialize(&ORIGINAL_ENV);
    lazy_static::initialize(&PUUID);
    lazy_static::initialize(&PID);
    lazy_static::initialize(&LD_PRELOAD);
    lazy_static::initialize(&CWD);
    lazy_static::initialize(&WSROOT);
    lazy_static::initialize(&WSROOT_BASE);
    lazy_static::initialize(&UUID);
    lazy_static::initialize(&WISKTRACE);
    lazy_static::initialize(&WISKTRACK);
    lazy_static::initialize(&WISKMAP);
    lazy_static::initialize(&ENV);
    lazy_static::initialize(&CMDLINE);
    lazy_static::initialize(&MAPFIELDS);
    // The following are to be initialized in the main program and will happen
    // one of the intercepted API gets called from the main program.
    // This is to avoid doing complex operations inside the library constructor
    // and keep the initialization limited to essentials.
    lazy_static::initialize(&TEMPLATEMAP);
    lazy_static::initialize(&TRACER);
    lazy_static::initialize(&TRACKER);
    TRACKER.initialize();
    // lazy_static::initialize(&CONFIG);
    // lazy_static::initialize(&APP64BITONLY_PATTERNS);
}

// pub fn initialize_main_statics() -> bool {
//     if !initialized() {
//         return false;
//     }
//     TRACKER_INIT_ONCE.call_once(|| {
//         unsafe { TRACKER_INITIALIZING = true; }
//         // lazy_static::initialize(&TRACKERFDS);
//         // lazy_static::initialize(&ORIGINAL_ENV);
//         // lazy_static::initialize(&PUUID);
//         // lazy_static::initialize(&PID);
//         // lazy_static::initialize(&LD_PRELOAD);
//         // lazy_static::initialize(&CWD);
//         // lazy_static::initialize(&WSROOT);
//         // lazy_static::initialize(&WSROOT_BASE);
//         // lazy_static::initialize(&UUID);
//         // lazy_static::initialize(&WISKTRACE);
//         // lazy_static::initialize(&WISKTRACK);
//         // lazy_static::initialize(&WISKMAP);
//         // lazy_static::initialize(&ENV);
//         // lazy_static::initialize(&CMDLINE);
//         // lazy_static::initialize(&MAPFIELDS);
//         // The following are to be initialized in the main program and will happen
//         // one of the intercepted API gets called from the main program.
//         // This is to avoid doing complex operations inside the library constructor
//         // and keep the initialization limited to essentials.
//         // lazy_static::initialize(&TRACER);
//         // lazy_static::initialize(&TEMPLATEMAP);
//         // lazy_static::initialize(&CONFIG);
//         // lazy_static::initialize(&APP64BITONLY_PATTERNS);
//         // lazy_static::initialize(&TRACKER);
//         // TRACKER.initialize();
//         unsafe { TRACKER_INITIALIZING = false; }
//     });
//     return true
// }

pub fn render(field: &str, vals: &HashMap<&str, &str>) -> String {
    Template::new(field).render(vals)
}

fn fd2path (fd : c_int ) -> PathBuf {
    // let f = unsafe { fs::File::from_raw_fd(fd) };
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

unsafe fn pathget(ipath: *const libc::c_char) -> String {
    String::from(utils::ptr2str(ipath))
}

unsafe fn pathgetabs(ipath: *const libc::c_char, fd: c_int) -> String {
    let mut ipath = pathget(ipath);
    // eprintln!("WSROOT_BASE: {}", WSROOT_BASE.as_str());
    if ipath.starts_with("/") {
        crate::path::normalize(ipath.as_str()).replace(WSROOT_BASE.as_str(), "")
    } else {
        if (fd == AT_FDCWD || fd < 0) {
            if let Ok(cwd) = env::current_dir() {
                let mut x=cwd.to_str().unwrap().to_owned();
                x.push_str("/");
                x.push_str(ipath.as_str());
                crate::path::normalize(x.as_str()).replace(WSROOT_BASE.as_str(), "")
            } else {
                ipath.insert_str(0,"<UNKNOWNPATH>/");
                crate::path::normalize(ipath.as_str()).replace(WSROOT_BASE.as_str(), "")
            }
        } else {
            let mut p = crate::path::fd_to_pathstr(fd).unwrap();
            p.push_str("/");
            p.push_str(ipath.as_str());
            crate::path::normalize(p.as_str()).replace(WSROOT_BASE.as_str(), "")
        }
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
        let flags = if WISKTRACK.ends_with(".file") {
            (O_CREAT|O_WRONLY|O_APPEND|O_LARGEFILE)
        } else {
            (O_CREAT|O_WRONLY|O_APPEND|O_LARGEFILE|O_CLOEXEC)
        };
        if let Ok(f) = File::open(WISKTRACK.as_str(), flags as i32,
                                     (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32, WISKTRACKFD) {
            let tracker = Tracker {
                file :  f,
            };
            cevent!(Level::INFO, "Tracker Create: FD={:?}",tracker.file);
            tracker
        } else {
            errorexit!("Error opening track file: {}\n", WISKTRACK.as_str());
        }
    }

    pub fn initialize(&self) {
        // debug(format_args!("Tracker Initializer\n"));
        cevent!(Level::INFO, "Tracker Initialization:");
        setdebugmode!("program_start");

        let pcw = (("UUID", &UUID.to_owned()), ("PID", &PID.to_owned()), ("CWD", &CWD.to_owned()), ("WSROOT", &WSROOT.to_owned()));
        cevent!(Level::INFO, "{} CALLS {}", PUUID.as_str(), serde_json::to_string(&pcw).unwrap());
        (&self.file).write_all(format!("{} CALLS {}\n", PUUID.as_str(), serde_json::to_string(&pcw).unwrap()).as_bytes()).unwrap();
        // (&self.file).write_all(format!("{} CALLS {}\n", &PUUID.as_str(), serde_json::to_string(&UUID.as_str()).unwrap()).as_bytes()).unwrap();
        // (&self.file).write_all(format!("{} CMDLINE {}\n", &UUID.as_str(), serde_json::to_string(&CMDLINE.to_vec()).unwrap()).as_bytes()).unwrap();
        // (&self.file).write_all(format!("{} WISKENV {}\n", &UUID.as_str(), serde_json::to_string(&ENV.to_owned()).unwrap()).as_bytes()).unwrap();
        self.report("CMDLINE", &serde_json::to_string(&CMDLINE.to_vec()).unwrap());
        self.report("WISKENV", &serde_json::to_string(&ENV.to_owned()).unwrap());
        // (&self.file).write_all(format!("{} PID {}\n", UUID.as_str(), serde_json::to_string(&PID.to_owned()).unwrap()).as_bytes()).unwrap();
        // (&self.file).write_all(format!("{} CWD {}\n", UUID.as_str(), serde_json::to_string(&CWD.to_owned()).unwrap()).as_bytes()).unwrap();
        // (&self.file).write_all(format!("{} WSROOT {}\n", UUID.as_str(), serde_json::to_string(&WSROOT.to_owned()).unwrap()).as_bytes()).unwrap();
        // event!(Level::INFO, "Tracker Initialization Complete: {} CALLS {}, WISKENV: {}, CMD: {}",
        //         tracker.puuid, serde_json::to_string(&tracker.uuid).unwrap(), serde_json::to_string(&tracker.wiskfields).unwrap(),
        //         &tracker.cmdline.join(" "));
        // debug(format_args!("Tracker Initializer Complete\n"));
        cevent!(Level::INFO, "Tracker Initializer Complete");
    }
    
    pub fn report(self: &Self, op : &str, value: &str) {
        let mut minlen: usize = &UUID.as_str().len() + op.len() + 2;
        let mut availen: usize = SENDLIMIT - minlen;
        let mut lenleft = value.len();
        let mut ind = 0;
        let mut contin = "";
        static mut REOPENCOUNT:i32 =0;

        cevent!(Level::INFO, "op={} value={}", op, value);
        // println!("{} op={} value={}", self.uuid, op, value);
        while lenleft != 0 {
            let max = if lenleft > availen {lenleft = lenleft - availen; ind + availen } 
                    else { let x=lenleft; lenleft = 0; ind + x };
            contin = if lenleft > availen { "*" } else { " " };
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
        // (&self.file).flush().unwrap();
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
        // let args = utils::ptr2str(path);
        let args = (&pathgetabs(path,AT_FDCWD), );
        (&(*self).file).write_str(&UUID.as_str());
        (&(*self).file).write_str(" READLINK ");
        (&(*self).file).write_cstrptr(path, JSON);
        self.report("READLINK", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportsymlink(self: &Self, target: *const libc::c_char, linkpath: *const libc::c_char) {
        let args = (utils::ptr2str(target), pathgetabs(linkpath,AT_FDCWD));
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportsymlinkat(self: &Self, target: *const libc::c_char, newdirfd: libc::c_int, linkpath: *const libc::c_char) {
        let args = (utils::ptr2str(target), pathgetabs(linkpath, newdirfd));
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportlink(self: &Self, oldpath: *const c_char, newpath: *const c_char) {
        let args = (&pathgetabs(oldpath,AT_FDCWD), &pathgetabs(newpath,AT_FDCWD));
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportlinkat(self: &Self, olddirfd: c_int, oldpath: *const c_char, newdirfd: c_int, newpath: *const c_char, flags: c_int) {
        let args = (&pathgetabs(oldpath, olddirfd), &pathgetabs(newpath, newdirfd), flags);
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportunlink(self: &Self, pathname: *const libc::c_char) {
        let args = (pathgetabs(pathname,AT_FDCWD),);
        self.report("UNLINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportunlinkat(self: &Self, dirfd: libc::c_int, pathname: *const libc::c_char, flags: libc::c_int) {
        let args = (pathgetabs(pathname,dirfd),flags);
        self.report("UNLINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportchmod(self: &Self, pathname: *const libc::c_char, mode: libc::mode_t) {
        let args = (pathgetabs(pathname,AT_FDCWD), mode);
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
        let args = (pathgetabs(pathname,AT_FDCWD), mode);
        self.report("WRITES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportfopen(self: &Self, name: *const libc::c_char, mode: *const libc::c_char) {
        let args = (pathgetabs(name,AT_FDCWD), utils::ptr2str(mode));
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
        let oper = if (flags & O_RDWR) == O_RDWR {
            "READWRITES"
        } else if (flags & O_WRONLY) == O_WRONLY {
            "WRITES"
        } else {
            "READS"
        };
        if (flags & O_CREAT) == O_CREAT {
            let args = (pathgetabs(pathname,AT_FDCWD), flags, mode);
            self.report(oper, &serde_json::to_string(&args).unwrap());
        } else {
            let args = (pathgetabs(pathname,AT_FDCWD), flags);
            self.report(oper, &serde_json::to_string(&args).unwrap());
        }
    }

    pub unsafe fn reportexecvpe(self: &Self, variant: &str, path: *const libc::c_char,
                                argv: *const *const libc::c_char, envcstr: &Vec<CString>) {
        let mut vargv: Vec<&str> = vec![];
        let mut venv: Vec<Vec<&str>> = vec![];
        for i in 0 .. {
            let argptr: *const c_char = *(argv.offset(i));
            if argptr != ptr::null() {
                vargv.push(utils::ptr2str(argptr))
            } else {
                break;
            }
        }
        for i in envcstr.iter() {
            venv.push(i.to_str().unwrap().splitn(2,"=").collect());
        }
        let args = (variant, pathgetabs(path,AT_FDCWD), vargv, venv);
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportpopen(self: &Self, command: *const libc::c_char, ctype: *const libc::c_char) {
        let args = ("popen", "/bin/sh", utils::ptr2str(command), utils::ptr2str(ctype));
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportsystem(self: &Self, command: *const libc::c_char) {
        let args = ("system",  "/bin/sh", utils::ptr2str(command));
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportcoredumped(self: &Self, variant: &str, pid: libc::pid_t) {
        let args = (variant.to_owned(), ("PID", &PID.to_owned()), ("UUID", &UUID.to_owned()), ("CRASHPID", pid));
        self.report("COREDUMPED", &serde_json::to_string(&args).unwrap());
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} test_001 D\n", TRACKER.uuid)));
//         Ok(())
//     }

//     #[test]
//     fn report_test_002() -> io::Result<()> {
//         TRACKER.report("test_002", &"D".repeat(SENDLIMIT-32));
//         println!("FileName: {}", TRACKER.filename);
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} test_002 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
//         Ok(())
//     }

//     #[test]
//     fn report_tests_003() -> io::Result<()> {
//         TRACKER.report("test_003", &"D".repeat(SENDLIMIT-31));
//         println!("FileName: {}", TRACKER.filename);
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
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
//         let mut rfile = fs::File::open(&TRACKER.filename)?;
//         let mut buffer = String::new();
//         rfile.read_to_string(&mut buffer)?;
//         assert!(buffer.contains(&format!("{} EXECUTES [\"/bin/sh\",\"echo \\\"something\\\"\",\"ctype\"]\n", TRACKER.uuid)));
//         assert!(true);
//         Ok(())
//     }

// }