use std::mem;
use std::env;
use std::ffi::{CString, CStr, OsString};
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions, metadata, create_dir_all};
use std::string::String;
use std::env::var;
use std::collections::HashMap;
use libc::{c_char,c_int, mode_t};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use base_62;
use filepath::FilePath;


const SENDLIMIT: usize = 4094;
// const SENDLIMIT: usize = 100;

type Envar = (String, String);

// #[derive(Serialize, Deserialize, Debug)]
// pub enum Ops {
//     CALLS(CString),
//     PID(CString),
//     PPID(CString),
//     PWD(CString),
//     CMDPATH(CString),
//     CMD(Vec<CString>),
//     ENV(Vec<Envar>),
//     READS(Vec<CString>),
//     COMPLETE(Vec<CString>),
//     WRITES(Vec<CString>),
//     LINKS(Vec<CString,CString>),
//     CHMODS(Vec<CString>)
// }

pub struct Tracker {
    pub wsroot: String,
    pub filename: String,
    pub file: File,
    pub uuid: String,
    pub puuid: String,
    pub env : HashMap<String, String>,
}

fn fd2path (fd : c_int ) -> PathBuf {
    let f = unsafe { File::from_raw_fd(fd) };
    let fp = f.path().unwrap();
    println!("{}",fp.as_path().to_str().unwrap());
    mem::forget(f); 
    fp
}

impl Tracker {
    pub fn init() -> Tracker {
        let wsroot:String = match env::var("WISK_WSROOT") {
            Ok(wsroot) => wsroot,
            Err(_) => String::from("/tmp")
        };
        if !Path::new(&wsroot).exists() {
            create_dir_all(&wsroot).unwrap();
        }
        let puuid:String = match env::var("WISK_UUID") {
            Ok(uuid) => uuid,
            Err(_) => String::from("XXXXXXXXXXXXXXXXXXXXXX")
        };
        let uuid:String = format!("{}", base_62::encode(Uuid::new_v4().as_bytes()));
        let fname = match var("WISK_TRACKFILE") {
            Ok(v) => v,
            Err(_) => String::from(format!("{}/wisktrack/track.{}", wsroot, uuid)),
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
        println!("Track Data: {}", fname);
        // println!("Ennvironment: {:?}", map.clone().into_iter().collect::<Vec<(String, String)>>());
        // let mut file = File::create(&filename).unwrap();
        // wisk_report(file, &PARENT_UUID, "Calls", &UUID);
        // write!(self.file, "{} Calls \"{}\"", *PARENT_UUID, *UUID).unwrap();
        let tracker = Tracker {
            wsroot : wsroot.to_string(),
            filename : fname.to_string(),
            file : OpenOptions::new().create(true).append(true).open(&fname).unwrap(),
            uuid  : uuid,
            puuid :  puuid,
            env : map,
        };
        (&tracker.file).write_all(format!("{} CALLS {}\n", tracker.puuid, serde_json::to_string(&tracker.uuid).unwrap()).as_bytes()).unwrap();
        tracker
    }
    
    pub fn report(self: &Self, op : &str, value: &str) {
        let mut minlen: usize = self.uuid.len() + op.len() + 2;
        let mut availen: usize = SENDLIMIT - minlen;
        let mut lenleft = value.len();
        let mut ind = 0;
        let mut contin = "";
        println!("{} op={} value={}", self.uuid, op, value);
        while lenleft != 0 {
            let max = if lenleft > availen {lenleft = lenleft - availen; ind + availen } 
                    else { let x=lenleft; lenleft = 0; ind + x };
            println!("minlen={} valeft={} ind={} max={}\n{} {} {}", minlen, lenleft, ind, max,
                    self.uuid, op, contin);
            (&self.file).write_all(format!("{} {} {}{}\n", self.uuid, op, contin, &value[ind..max]).as_bytes()).unwrap();
            contin = "*";
            ind = max ;
            minlen = self.uuid.len() + op.len() + 2 + 1;
            availen = SENDLIMIT - minlen;
        };
        (&self.file).flush().unwrap();
    }
    
    // pub fn reportop(self: &Self, op: Ops) {
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

    pub unsafe fn reportfopen(self: &Self, name: *const libc::c_char, mode: *const libc::c_char) {
        let args = (CStr::from_ptr(name).to_str().unwrap(), CStr::from_ptr(mode).to_str().unwrap());
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

    pub unsafe fn reportsymlink(self: &Self, target: *const libc::c_char, linkpath: *const libc::c_char) {
        let args = (CStr::from_ptr(target).to_str().unwrap(), CStr::from_ptr(linkpath).to_str().unwrap());
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportsymlinkat(self: &Self, target: *const libc::c_char, newdirfd: libc::c_int, linkpath: *const libc::c_char) {
        let args = (CStr::from_ptr(target).to_str().unwrap(), newdirfd, CStr::from_ptr(linkpath).to_str().unwrap());
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportlink(self: &Self, oldpath: *const c_char, newpath: *const c_char) {
        let args = (CStr::from_ptr(oldpath).to_str().unwrap(), CStr::from_ptr(newpath).to_str().unwrap());
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportlinkat(self: &Self, olddirfd: c_int, oldpath: *const c_char, newdirfd: c_int, newpath: *const c_char, flags: c_int) {
        let oldpath = CStr::from_ptr(oldpath).to_str().unwrap();
        let newpath = CStr::from_ptr(newpath).to_str().unwrap();
        // let oldpathstr: OsString;
        // let mut olddirpath: PathBuf;
        // let oldpath = if oldpath.starts_with("/") {
        //     oldpath
        // } else {
        //     let mut olddirpath = fd2path(olddirfd);
        //     olddirpath.push(oldpath);
        //     oldpathstr = olddirpath.into_os_string();
        //     &oldpathstr.into_string().unwrap()
        // };

        let args = (olddirfd, oldpath, newdirfd, newpath, flags);
        self.report("LINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportunlink(self: &Self, pathname: *const libc::c_char) {
        let args = (CStr::from_ptr(pathname).to_str().unwrap(),);
        self.report("UNLINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportunlinkat(self: &Self, dirfd: libc::c_int, pathname: *const libc::c_char, flags: libc::c_int) {
        let args = (dirfd, CStr::from_ptr(pathname).to_str().unwrap(),flags);
        self.report("UNLINKS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportchmod(self: &Self, pathname: *const libc::c_char, mode: libc::mode_t) {
        let args = (CStr::from_ptr(pathname).to_str().unwrap(), mode);
        self.report("CHMODS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportfchmod(self: &Self, fd: libc::c_int, mode: libc::mode_t) {
        let args = (fd,mode);
        self.report("CHMODS", &serde_json::to_string(&args).unwrap());
    }

    pub unsafe fn reportfchmodat(self: &Self, dirfd: libc::c_int, pathname: *const libc::c_char, mode: libc::mode_t, flags: libc::c_int) {
        let args = (dirfd, CStr::from_ptr(pathname).to_str().unwrap(),mode,flags);
        self.report("CHMODS", &serde_json::to_string(&args).unwrap());
    }
}


lazy_static! {
    pub static ref TRACKER : Tracker = Tracker::init();
}


#[cfg(test)]
mod report_tests {
    use std::io;
    use libc::{opendir, dirfd};
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
    use libc::{opendir, dirfd};
    use super::*;

    #[test]
    fn test_link() -> io::Result<()> {
        unsafe {
            TRACKER.reportlink(CString::new("/a/b/link").unwrap().as_ptr(),CString::new("/x/y/z").unwrap().as_ptr());
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/link\",\"/x/y/z\"]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_linkat() -> io::Result<()> {
        let fd = unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportlinkat(fd, CString::new("/a/b/linkat").unwrap().as_ptr(),fd, CString::new("/x/y/z").unwrap().as_ptr(), 300);
            // TRACKER.reportlinkat(fd, CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/z").unwrap().as_ptr(), 300);
            fd
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} LINKS [{},\"/a/b/linkat\",{},\"/x/y/z\",300]\n", TRACKER.uuid, fd, fd)));
        // assert!(buffer.contains(&format!("{} LINKS [{},\"/tmp/a/b/c\",{},\"/tmp/x/y/z\",300]\n", TRACKER.uuid, fd, fd)));
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
    fn test_symlink() -> io::Result<()> {
        unsafe {
            TRACKER.reportsymlink(CString::new("/a/b/symlink").unwrap().as_ptr(),CString::new("/x/y/z").unwrap().as_ptr());
        }
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/symlink\",\"/x/y/z\"]\n", TRACKER.uuid)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_symlinkat() -> io::Result<()> {
        let fd = unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportsymlinkat(CString::new("/a/b/symlinkat").unwrap().as_ptr(),fd, CString::new("/x/y/z").unwrap().as_ptr());
            // TRACKER.reportsymlinkat(CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/z").unwrap().as_ptr());
            fd
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} LINKS [\"/a/b/symlinkat\",{},\"/x/y/z\"]\n", TRACKER.uuid, fd)));
        // assert!(buffer.contains(&format!("{} LINKS [\"/tmp/a/b/c\",{},\"/tmp/x/y/z\"]\n", TRACKER.uuid, fd)));
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
        let fd = unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportunlinkat(fd, CString::new("/a/b/unlinkat").unwrap().as_ptr(),300);
            // TRACKER.reportsymlinkat(CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/z").unwrap().as_ptr());
            fd
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} UNLINKS [{},\"/a/b/unlinkat\",300]\n", TRACKER.uuid, fd)));
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
        let fd = unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportfchmod(fd,0);
            // TRACKER.reportsymlinkat(CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/z").unwrap().as_ptr());
            fd
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} CHMODS [{},0]\n", TRACKER.uuid,fd)));
        assert!(true);
        Ok(())
    }

    #[test]
    fn test_fchmodat() -> io::Result<()> {
        let fd = unsafe {
            let fd = dirfd(opendir(CString::new("/tmp").unwrap().as_ptr()));
            TRACKER.reportfchmodat(fd, CString::new("/a/b/fchmodat").unwrap().as_ptr(),0,0);
            // TRACKER.reportsymlinkat(CString::new("a/b/c").unwrap().as_ptr(),fd, CString::new("x/y/z").unwrap().as_ptr());
            fd
        };
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} CHMODS [{},\"/a/b/fchmodat\",0,0]\n", TRACKER.uuid, fd)));
        assert!(true);
        Ok(())
    }

}

