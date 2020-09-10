use std::mem;
use std::{env, ptr};
use std::ffi::{CStr};
use std::os::unix::io::FromRawFd;
// use std::sync::Mutex;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions, metadata, create_dir_all};
use std::string::String;
use std::env::var;
use std::process;
use std::collections::HashMap;
use libc::{c_char,c_int, O_CREAT};
use uuid::Uuid;
// use serde::{Serialize, Deserialize};
use base_62;
use filepath::FilePath;
use tracing::dispatcher::{with_default, Dispatch};
use tracing_appender::non_blocking::WorkerGuard;
use tracing::{Level, event, };
use redhook::ld_preload::make_dispatch;



const SENDLIMIT: usize = 4094;
// const SENDLIMIT: usize = 100;

pub struct Tracker {
    pub wsroot: String,
    pub cwd: String,
    pub filename: String,
    pub file: File,
    pub uuid: String,
    pub puuid: String,
    pub pid: String,
    pub env : HashMap<String, String>,
}

fn fd2path (fd : c_int ) -> PathBuf {
    let f = unsafe { File::from_raw_fd(fd) };
    let fp = f.path().unwrap();
    // println!("{}",fp.as_path().to_str().unwrap());
    mem::forget(f); 
    fp
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
            dirpath = fd2path(fd);
        } else {
            dirpath = PathBuf::from(&TRACKER.cwd);
        }
        dirpath.push(ipath);
        path2str(dirpath)
    };
    if ipath.starts_with(&TRACKER.wsroot) {
        ipath.replacen(&TRACKER.wsroot,"",1)
    } else {
        ipath
    }
}



impl Tracker {
    pub fn init() -> Tracker {
        MY_DISPATCH.with(|(_tracing, my_dispatch, _guard)| {
            with_default(&my_dispatch, || {
                event!(Level::INFO, "Tracker Initialization:\n");
                let wsroot:String = match env::var("WISK_WSROOT") {
                    Ok(mut wsroot) => {
                        if !wsroot.ends_with("/") {
                            wsroot.push_str("/")
                        }
                        if !Path::new(&wsroot).exists() {
                            create_dir_all(&wsroot).unwrap();
                        }
                        wsroot
                    },
                    Err(_) => String::new(),
                };
                let puuid:String = match env::var("WISK_UUID") {
                    Ok(uuid) => uuid,
                    Err(_) => String::from("XXXXXXXXXXXXXXXXXXXXXX")
                };
                let uuid:String = format!("{}", base_62::encode(Uuid::new_v4().as_bytes()));
                let fname = match var("WISK_TRACKFILE") {
                    Ok(v) => v,
                    Err(_) => {
                        if wsroot.is_empty() {
                            if !Path::new("/tmp/wisktrack").exists() {
                                create_dir_all("/tmp/wisktrack").unwrap();
                            }
                            String::from(format!("/tmp/wisktrack/track.{}", uuid))
                        } else {
                            String::from(format!("{}/wisktrack/track.{}", wsroot, uuid))
                        }
                    },
                };
                if !Path::new(&fname).parent().unwrap().exists() {
                    create_dir_all(Path::new(&fname).parent().unwrap()).unwrap();
                }
                let trackdir = Path::new(&fname).parent().unwrap();
                if !metadata(&trackdir).unwrap().is_dir() {
                    create_dir_all(trackdir).unwrap();
                }
                let fname = if Path::new(&fname).exists() && metadata(&fname).unwrap().is_dir() {
                    String::from(format!("{}/wisk_track.{}", fname, uuid))
                } else { String::from(&fname) };

                let mut map = HashMap::new();
                for (key, val) in env::vars_os() {
                    // Use pattern bindings instead of testing .is_some() followed by .unwrap()
                    if let (Ok(k), Ok(v)) = (key.into_string(), val.into_string()) {
                        map.insert(k, v);
                    }
                }
                let cwdostr = env::current_dir().unwrap().into_os_string();
                // println!("Track Data: {}", fname);
                let tracker = Tracker {
                    wsroot : wsroot.to_string(),
                    filename : fname.to_string(),
                    file : OpenOptions::new().create(true).append(true).open(&fname).unwrap(),
                    uuid  : uuid,
                    puuid :  puuid,
                    pid: process::id().to_string(),
                    cwd : cwdostr.into_string().unwrap(),
                    env : map,
                };
                (&tracker.file).write_all(format!("{} CALLS {}\n", tracker.puuid, serde_json::to_string(&tracker.uuid).unwrap()).as_bytes()).unwrap();
                (&tracker.file).write_all(format!("{} CWD {}\n", tracker.uuid, serde_json::to_string(&tracker.cwd).unwrap()).as_bytes()).unwrap();
                (&tracker.file).write_all(format!("{} WSROOT {}\n", tracker.uuid, serde_json::to_string(&tracker.wsroot).unwrap()).as_bytes()).unwrap();
                let envars:Vec<(String,String)> = env::vars_os().map(|(k,v)| (k.into_string().unwrap(),v.into_string().unwrap()))
                                                                .collect();
                (&tracker.file).write_all(format!("{} ENVIRONMENT {}\n", tracker.uuid, serde_json::to_string(&envars).unwrap()).as_bytes()).unwrap();
                event!(Level::INFO, "Tracker Initialization Complete: {} CALLS {}, WSROOT: {}, CWD: {}",
                       tracker.puuid, serde_json::to_string(&tracker.uuid).unwrap(), serde_json::to_string(&tracker.cwd).unwrap(),
                       serde_json::to_string(&tracker.wsroot).unwrap());
                tracker
            })
        })
    }
    
    pub fn report(self: &Self, op : &str, value: &str) {
        let mut minlen: usize = self.uuid.len() + op.len() + 2;
        let mut availen: usize = SENDLIMIT - minlen;
        let mut lenleft = value.len();
        let mut ind = 0;
        let mut contin = "";
        // println!("{} op={} value={}", self.uuid, op, value);
        while lenleft != 0 {
            let max = if lenleft > availen {lenleft = lenleft - availen; ind + availen } 
                    else { let x=lenleft; lenleft = 0; ind + x };
            // println!("minlen={} valeft={} ind={} max={}\n{} {} {}", minlen, lenleft, ind, max,
            //         self.uuid, op, contin);
            (&self.file).write_all(format!("{} {} {}{}\n", self.uuid, op, contin, &value[ind..max]).as_bytes()).unwrap();
            contin = "*";
            ind = max ;
            minlen = self.uuid.len() + op.len() + 2 + 1;
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

    pub unsafe fn reportpopen(self: &Self, command: *const libc::c_char, ctype: *const libc::c_char) {
        let args = ("/bin/sh", cstr2str(command), cstr2str(ctype));
        self.report("EXECUTES", &serde_json::to_string(&args).unwrap());
    }

}

thread_local! {
    #[allow(nonstandard_style)]
    pub static MY_DISPATCH_initialized: ::core::cell::Cell<bool> = false.into();
}

thread_local! {
    pub static MY_DISPATCH: (bool, Dispatch, WorkerGuard) = {
        let ret = make_dispatch("WISK_TRACE");
        MY_DISPATCH_initialized.with(|it| it.set(true));
        ret
    };
}




lazy_static! {
    pub static ref TRACKER : Tracker = Tracker::init();
}


#[cfg(test)]
mod report_tests {
    use std::io;
    use super::*;

    #[test]
    fn report_test_000() -> io::Result<()> {
        TRACKER.report("test_000", "");
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(!buffer.contains(&format!("\n\n")));
        assert!(!buffer.contains(&format!("{} test_000\n", TRACKER.uuid)));
        Ok(())
    }

    #[test]
    fn report_test_001() -> io::Result<()> {
        TRACKER.report("test_001", "D");
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_001 D\n", TRACKER.uuid)));
        Ok(())
    }

    #[test]
    fn report_test_002() -> io::Result<()> {
        TRACKER.report("test_002", &"D".repeat(SENDLIMIT-32));
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_002 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
        Ok(())
    }

    #[test]
    fn report_tests_003() -> io::Result<()> {
        TRACKER.report("test_003", &"D".repeat(SENDLIMIT-31));
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_003 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
        assert!(buffer.contains(&format!("{} test_003 *{}\n", TRACKER.uuid, &"D".repeat(1))));
        Ok(())
    }

    #[test]
    fn report_test_004() -> io::Result<()> {
        TRACKER.report("test_004", &"D".repeat(SENDLIMIT*2-9));
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_004 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
        assert!(buffer.contains(&format!("{} test_004 *{}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-33))));
        Ok(())
    }

    #[test]
    fn report_test_005() -> io::Result<()> {
        TRACKER.report("test_005", &"D".repeat(SENDLIMIT*2-(32*2)));
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_005 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-32))));
        assert!(buffer.contains(&format!("{} test_005 *{}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-33))));
        assert!(buffer.contains(&format!("{} test_005 *{}\n", TRACKER.uuid, &"D".repeat(1))));
        Ok(())
    }
}

#[cfg(test)]
mod reportop_tests {
    use std::io;
    use std::ffi::{CString};
    use libc::{opendir, dirfd};
    use super::*;

    #[test]
    fn test_link() -> io::Result<()> {
        unsafe {
            TRACKER.reportlink(CString::new("/a/b/c").unwrap().as_ptr(),CString::new("/x/y/link").unwrap().as_ptr());
            TRACKER.reportlink(CString::new("/a/b/c").unwrap().as_ptr(),CString::new("x/y/link").unwrap().as_ptr());
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"/x/y/link\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"{}/x/y/link\"]\n", TRACKER.uuid, TRACKER.cwd)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_linkat() -> io::Result<()> {
        unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportlinkat(fd, CString::new("/a/b/c").unwrap().as_ptr(),fd, CString::new("/x/y/linkat").unwrap().as_ptr(), 300);
            TRACKER.reportlinkat(fd, CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/linkat").unwrap().as_ptr(), 300);
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"/x/y/linkat\",300]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} LINKS [\"/tmp/a/b/c\",\"/tmp/x/y/linkat\",300]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_symlink() -> io::Result<()> {
        unsafe {
            TRACKER.reportsymlink(CString::new("/a/b/c").unwrap().as_ptr(),CString::new("/x/y/symlink").unwrap().as_ptr());
            TRACKER.reportsymlink(CString::new("/a/b/c").unwrap().as_ptr(),CString::new("x/y/symlink").unwrap().as_ptr());
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"/x/y/symlink\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"{}/x/y/symlink\"]\n", TRACKER.uuid, TRACKER.cwd)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_symlinkat() -> io::Result<()> {
        unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportsymlinkat(CString::new("/a/b/c").unwrap().as_ptr(),fd, CString::new("/x/y/symlinkat").unwrap().as_ptr());
            TRACKER.reportsymlinkat(CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/symlinkat").unwrap().as_ptr());
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/c\",\"/x/y/symlinkat\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} LINKS [\"a/b/c\",\"/tmp/x/y/symlinkat\"]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_unlink() -> io::Result<()> {
        unsafe {
            TRACKER.reportunlink(CString::new("/a/b/unlink").unwrap().as_ptr());
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} UNLINKS [\"/a/b/unlink\"]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_unlinkat() -> io::Result<()> {
        unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportunlinkat(fd, CString::new("/a/b/unlinkat").unwrap().as_ptr(),300);
            TRACKER.reportunlinkat(fd, CString::new("a/b/unlinkat").unwrap().as_ptr(),300);
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} UNLINKS [\"/a/b/unlinkat\",300]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} UNLINKS [\"/tmp/a/b/unlinkat\",300]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_chmod() -> io::Result<()> {
        unsafe {
            TRACKER.reportchmod(CString::new("/a/b/chmod").unwrap().as_ptr(), 0);
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} CHMODS [\"/a/b/chmod\",0]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_fchmod() -> io::Result<()> {
        unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportfchmod(fd,0);
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} CHMODS [\"/tmp\",0]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_fchmodat() -> io::Result<()> {
        unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportfchmodat(fd, CString::new("/a/b/fchmodat").unwrap().as_ptr(),0,0);
            TRACKER.reportfchmodat(fd, CString::new("a/b/fchmodat").unwrap().as_ptr(),0,0);
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} CHMODS [\"/a/b/fchmodat\",0,0]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} CHMODS [\"/tmp/a/b/fchmodat\",0,0]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_creat() -> io::Result<()> {
        unsafe {
            TRACKER.reportcreat(CString::new("/a/b/creat").unwrap().as_ptr(),0);
            TRACKER.reportcreat(CString::new("a/b/creat").unwrap().as_ptr(),0);
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} WRITES [\"/a/b/creat\",0]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} WRITES [\"{}/a/b/creat\",0]\n", TRACKER.uuid, TRACKER.cwd)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_fopen() -> io::Result<()> {
        unsafe {
            TRACKER.reportfopen(CString::new("/a/b/reads").unwrap().as_ptr(),CString::new("r").unwrap().as_ptr());
            TRACKER.reportfopen(CString::new("/a/b/readsplus").unwrap().as_ptr(),CString::new("r+").unwrap().as_ptr());
            TRACKER.reportfopen(CString::new("/a/b/writes").unwrap().as_ptr(),CString::new("w").unwrap().as_ptr());
            TRACKER.reportfopen(CString::new("/a/b/writesplus").unwrap().as_ptr(),CString::new("w+").unwrap().as_ptr());
            TRACKER.reportfopen(CString::new("/a/b/appends").unwrap().as_ptr(),CString::new("a").unwrap().as_ptr());
            TRACKER.reportfopen(CString::new("/a/b/appendsplus").unwrap().as_ptr(),CString::new("a+").unwrap().as_ptr());
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} READS [\"/a/b/reads\",\"r\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} READS [\"/a/b/readsplus\",\"r+\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} WRITES [\"/a/b/readsplus\",\"r+\"]\n", TRACKER.uuid)));

        assert!(buffer.contains(&format!("{} WRITES [\"/a/b/writes\",\"w\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} WRITES [\"/a/b/writesplus\",\"w+\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} READS [\"/a/b/writesplus\",\"w+\"]\n", TRACKER.uuid)));


        assert!(buffer.contains(&format!("{} WRITES [\"/a/b/appends\",\"a\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} WRITES [\"/a/b/appendsplus\",\"a+\"]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} READS [\"/a/b/appendsplus\",\"a+\"]\n", TRACKER.uuid)));

        assert!(true);
        Ok(())
    }

    #[test]
    fn test_execv() -> io::Result<()> {
        let argv = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];
        let cstr_argv: Vec<_> = argv.iter()
                                    .map(|arg| CString::new(arg.as_str()).unwrap())
                                    .collect();
        let mut p_argv: Vec<_> = cstr_argv.iter() // do NOT into_iter()
                                          .map(|arg| arg.as_ptr())
                                          .collect();
        p_argv.push(std::ptr::null());
        unsafe {
            TRACKER.reportexecv(CString::new("/a/b/execv").unwrap().as_ptr(), p_argv.as_ptr());
            TRACKER.reportexecv(CString::new("a/b/execv").unwrap().as_ptr(), p_argv.as_ptr());
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} EXECUTES [\"/a/b/execv\",[\"arg1\",\"arg2\",\"arg3\"]]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} EXECUTES [\"{}/a/b/execv\",[\"arg1\",\"arg2\",\"arg3\"]]\n", TRACKER.uuid, TRACKER.cwd)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_execvpe() -> io::Result<()> {
        let args = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];
        let env = vec!["A=B".to_string(), "C=D".to_string(), "E=F".to_string()];
        let cstr_args: Vec<_> = args.iter()
                                    .map(|arg| CString::new(arg.as_str()).unwrap())
                                    .collect();
        let mut p_args: Vec<_> = cstr_args.iter() // do NOT into_iter()
                                          .map(|arg| arg.as_ptr())
                                          .collect();
        p_args.push(std::ptr::null());
        let cstr_env: Vec<_> = env.iter()
                                    .map(|arg| CString::new(arg.as_str()).unwrap())
                                    .collect();
        let mut p_env: Vec<_> = cstr_env.iter() // do NOT into_iter()
                                          .map(|arg| arg.as_ptr())
                                          .collect();
        p_env.push(std::ptr::null());
        unsafe {
            TRACKER.reportexecvpe(CString::new("/a/b/execvpe").unwrap().as_ptr(), p_args.as_ptr(), p_env.as_ptr());
            TRACKER.reportexecvpe(CString::new("a/b/execvpe").unwrap().as_ptr(), p_args.as_ptr(), p_env.as_ptr());
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} EXECUTES [\"/a/b/execvpe\",[\"arg1\",\"arg2\",\"arg3\"],[[\"A\",\"B\"],[\"C\",\"D\"],[\"E\",\"F\"]]]\n", TRACKER.uuid)));
        assert!(buffer.contains(&format!("{} EXECUTES [\"{}/a/b/execvpe\",[\"arg1\",\"arg2\",\"arg3\"],[[\"A\",\"B\"],[\"C\",\"D\"],[\"E\",\"F\"]]]\n", TRACKER.uuid, TRACKER.cwd)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_popen() -> io::Result<()> {
        unsafe {
            TRACKER.reportpopen(CString::new("echo \"something\"").unwrap().as_ptr(),
                                CString::new("ctype").unwrap().as_ptr());
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} EXECUTES [\"/bin/sh\",\"echo \\\"something\\\"\",\"ctype\"]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

}

