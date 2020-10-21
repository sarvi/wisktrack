
use std::sync::Mutex;
use std::ffi::{CStr, CString};
use std::{env, ptr};
use core::sync::atomic::{AtomicUsize, Ordering};
use nix::fcntl::OFlag;
use std::io::{Read, Write};
use std::fmt;
use std::error;
use libc::{c_char,c_int, O_CREAT, O_APPEND, O_LARGEFILE, O_CLOEXEC, AT_FDCWD, SYS_open, S_IRUSR, S_IWUSR, S_IRGRP, S_IWGRP};
use std::os::unix::io::{FromRawFd,AsRawFd,IntoRawFd, RawFd};
use std::collections::{HashMap, BTreeMap};
use std::path::{Path, PathBuf};
use std::fs::{File,read_to_string};
use nix::unistd::dup3;
use tracing::{Level, event};
use redhook::debug;
use serde::de;
use regex::{RegexSet};

pub static WISKFD: AtomicUsize = AtomicUsize::new(800);

pub fn vcstr2vecptr(vcstr: &Vec<CString>) -> Vec<*const c_char> {
    let vecptr: Vec<_> = vcstr.iter() // do NOT into_iter()
                              .map(|arg| arg.as_ptr())
                              .collect();
    vecptr
}

pub fn cpptr2vecptr(vecptr: *const *const libc::c_char) -> Vec<*const c_char> {
    let mut vcstr: Vec<*const c_char> = vec!();
    for i in 0 .. {
        unsafe {
            let argptr: *const c_char = *(vecptr.offset(i));
            if argptr != ptr::null() {
                vcstr.push(argptr);
            } else {
                vcstr.push(argptr);
                break;
            }
        }
    }
    vcstr
}


pub fn cpptr2vcstr(vecptr: *const *const libc::c_char) -> Vec<CString> {
    let mut vcstr: Vec<CString> = vec!();
    for i in 0 .. {
        unsafe {
            let argptr: *const c_char = *(vecptr.offset(i));
            if argptr != ptr::null() {
                vcstr.push(CStr::from_ptr(argptr).to_owned());
            } else {
                break;
            }    
        }
    }
    vcstr
}

pub fn cpptr2str(vecptr: *const *const libc::c_char, sep: &str) -> String {
    let mut str: String = String::new();
    for i in 0 .. {
        unsafe {
            let argptr: *const c_char = *(vecptr.offset(i));
            if argptr != ptr::null() {
                    if i != 0 {
                        str.push_str(sep);
                    }
                    str.push_str(CStr::from_ptr(argptr).to_str().unwrap());
            } else {
                str.push_str(sep);
                str.push_str("NULL");
                break;
            }
        }
    }
    str
}


pub fn cpptr2hashmap(vecptr: *const *const libc::c_char) -> HashMap<String,String> {
    let mut hash: HashMap<String,String> = HashMap::new();
    for i in 0 .. {
        unsafe {
            let argptr: *const c_char = *(vecptr.offset(i));
            if argptr != ptr::null() {
                let kv:Vec<&str> = CStr::from_ptr(argptr).to_str().unwrap().splitn(2,'=').collect();
                let t = CStr::from_ptr(argptr).to_str().unwrap();
                hash.insert(kv[0].to_string(), kv[1].to_string());
            } else {
                break;
            }    
        }
    }
    hash
}

pub fn hashmap2vcstr(hash: &HashMap<String,String>, order: Vec<&str>) -> Vec<CString> {
    let mut x:Vec<CString> = order.iter()
                              .map(|k| CString::new(format!("{}={}",k,hash[k.to_owned()]))
                                                .unwrap())
                              .collect();
    let mut remain:Vec<CString> = hash.into_iter()
                     .filter(|(k, v)| !order.iter().any(|i| k==i))
                     .map(|(k, v)| CString::new(format!("{}={}",k,v)).unwrap())
                     .collect();
    x.append(&mut remain);
    // eprintln!("VCSTR: {:#?}", x);
    x
}

pub fn hashmapassert(hash: &HashMap<String,String>, mut values: Vec<&str>) -> bool {
    let mut mv:Vec<(&str,&str)> = vec!();
    for (k,v) in hash {
        for (pos, e) in values.iter().enumerate() {
            if e == k {
                values.remove(pos);
                mv.push((k.as_str(), v.as_str()));
                break;
            }
        }
    }
    assert_eq!(values.len(),0, "Missing environment variables: {:?}", values);
    // if values.len() == 0 {
    //     event!(Level::INFO, "hashassert(match): {:?}",mv);
    // } else {
    //     event!(Level::INFO, "hashassert(no-match):");
    // }
    (values.len() == 0)
}

pub fn assert_ld_preload(envp: &Vec<*const c_char>, bit64: bool) {
    let mut found=false;
    for i in envp.iter().enumerate() {
        if envp[i.0].is_null() {
            continue
        }
        unsafe {
            let x = CStr::from_ptr(envp[i.0]).to_str().unwrap();
            if x.starts_with("LD_PRELOAD=") {
                found=true;
                if bit64 {
                    assert!(x.contains("lib64/libwisktrack.so"), "LD_PRELOAD is wrong. Set to {}",x);
                } else {
                    assert!(x.contains("${LIB}/libwisktrack.so"), "LD_PRELOAD is wrong. Set to {}",x);
                }
                assert!(x.contains("LD_PRELOAD="), "Does not have LD_PRELOAD. Set to {}",x);
                assert_eq!(i.0,0, "LD_PRELOAD in other places");
            }
        }
    }
}

pub fn envupdate(env: &mut HashMap<String,String>, fields: &Vec<(String,String)>) {
    for (k,v) in fields.iter() {
        if k == "LD_PRELOAD" {
            if let Some(cv) = env.get_mut(k) {
                if !cv.split(" ").any(|i| (*i).ends_with("libwisktrack.so")) {
                    // Ideally this should be push_str(). insert_str() because 
                    // XR ljam build uses alib cpio_preload.so that doesnt like this.
                    cv.insert_str(0, " ");
                    cv.insert_str(0, v);
                }
            } else {
                env.insert(k.to_string(),v.to_string());
            }
        } else {
            env.insert(k.to_string(),v.to_string());
        }
    }
}

pub fn envgetcurrent() -> HashMap<String,String> {
    let hash: HashMap<String,String> = env::vars_os()
                                .map(|(k,v)| (k.into_string().unwrap(),v.into_string().unwrap()))
                                .collect();
    hash
}

pub fn currentenvupdate(fields: &Vec<(String,String)>) {
    for (k,v) in fields.iter() {
        if k == "LD_PRELOAD" {
            if let Ok(mut cv) = env::var(k) {
                if !cv.split(" ").any(|i| (*i).ends_with("libwisktrack.so")) {
                    // assert_eq!(true, false, "");
                    cv.push_str(" ");
                    cv.push_str(v);
                    env::set_var(k,cv.as_str());
                }
            } else {
                env::set_var(k,v.as_str());
            }
        } else {
            env::set_var(k,v.as_str());
        }
    }
}

pub fn envextractwisk(fields: Vec<&str>) -> Vec<(String,String)> {
    let mut wiskmap: Vec<(String,String)> = vec!();
    use std::env::VarError::NotPresent;
    assert!(env::var_os("LD_PRELOAD").unwrap().to_str().unwrap().contains("${LIB}/libwisktrack.so"),
            "Initialization LD_PRELOAD is wrong. Set to {}",
            env::var_os("LD_PRELOAD").unwrap().to_str().unwrap());
    for k in fields.iter() {
        if let Some(eval) = env::var_os(k) {
            wiskmap.push(((*k).to_owned(), eval.into_string().unwrap()));
            env::remove_var(k);
        }
    }
    wiskmap
}

// pub fn getexecutable(file: *const c_char, argv: *const *const libc::c_char,
//                      mut env: HashMap<String,String>)
//                      -> (*const c_char, Vec<CString>, *const *const libc::c_char) {
//     if unsafe { *file } == b'/' {
//         utils::envupdate(&mut env,&WISKMAP);
//         (file, argv, vec!(), env)
//     } else {
//         // filestr = CStr::from_ptr(file);
//         utils::envupdate(&mut env,&WISKMAP);
//         (file, argv, vec!(), env)
//     }
// }


pub fn internal_open(filename: &str, mode: i32) -> File {
    let fd = WISKFD.fetch_add(1, Ordering::Relaxed) as i32;
    let filename = CString::new(filename).unwrap();
    let tempfd = unsafe {
        libc::syscall(SYS_open, filename.as_ptr(), mode,
                      S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP)
    };
    let fd = dup3(tempfd as i32, fd, OFlag::from_bits(O_CLOEXEC|O_LARGEFILE|O_APPEND|O_CREAT).unwrap()).unwrap();
    let f:File = unsafe { FromRawFd::from_raw_fd(fd as i32) };
    debug(format_args!("File Descriptor: {}, {:?}", tempfd, &f));
    (&f).write_all(format!("Something new\n").as_bytes()).unwrap();
    f
}

pub fn read_config<T>(basepath: &str, conf: &str) -> Result<T, String>
where
    T: de::DeserializeOwned,
{
    let cfgpath: PathBuf;
    if basepath.ends_with("libwisktrack.so") {
        let c = Path::new(basepath).parent().ok_or_else(|| {
            format!("Error accessing path parent of {}\n", basepath)
        })?;
        let c = c.parent().ok_or_else(|| {
            format!("Error accessing path parent of {}\n", c.display())
        })?;
        cfgpath = c.to_path_buf();
    } else {
        cfgpath = Path::new(basepath).join("wisk");
    }
    let cfgpath = Path::new(&cfgpath).join("config").join(conf).canonicalize().map_err(
        |e| format!("Error finding {} path: {:?}", e.to_string(), cfgpath)
    )?;
    let mut file = File::open(cfgpath.to_owned()).map_err(
        |e| format!("Error opening {} path: {:?}", e.to_string(), cfgpath)
    )?;
    let mut content = String::new();
    let size = file.read_to_string(&mut content).map_err(
        |e| format!("Error reading {} path: {:?}", e.to_string(), cfgpath)
    )?;
    serde_yaml::from_str(content.as_str()).map_err(
        |e| format!("Error parsing {} path: {:?}", e.to_string(), cfgpath)
    )
}


#[cfg(test)]
mod env_tests {
    use std::io;
    use std::env::VarError::NotPresent;
    use std::ffi::{CString};
    use libc::{opendir, dirfd};
    use super::*;

    #[test]
    fn test_vcstr2vecptr() -> io::Result<()> {
        let env = vec![CString::new("A=B").unwrap(), CString::new("C=D").unwrap(), CString::new("E=F").unwrap()];
        let mut vecptr = vcstr2vecptr(&env);
        for (i,j) in env.iter().zip(vecptr.iter_mut()) {
            unsafe {
                assert_eq!(i.as_c_str(), CStr::from_ptr(*j));
            }
        }
        Ok(())
    }

    #[test]
    fn test_vecptr2vcstr() -> io::Result<()> {
        let env = vec![CString::new("A=B").unwrap(), CString::new("C=D").unwrap(), CString::new("E=F").unwrap()];
        let vecptr: Vec<_> = env.iter() // do NOT into_iter()
                              .map(|arg| arg.as_ptr())
                              .collect();
        let mut vcstr = cpptr2vcstr(vecptr.as_ptr());
        for (i,j) in env.iter().zip(vcstr.iter_mut()) {
            unsafe {
                assert_eq!(i, j);
            }
        }
        Ok(())
    }

    // #[test]
    // fn test_execvpe() -> io::Result<()> {
    //     let env = vec!["A=B".to_string(), "C=D".to_string(), "E=F".to_string()];
    //     let cstr_env: Vec<_> = env.iter()
    //                                 .map(|arg| CString::new(arg.as_str()).unwrap())
    //                                 .collect();
    //     let mut p_env: Vec<_> = cstr_env.iter() // do NOT into_iter()
    //                                       .map(|arg| arg.as_ptr())
    //                                       .collect();
    //     p_env.push(std::ptr::null());
    //     envupdate(p_env);
    //     unsafe {
    //         TRACKER.reportexecvpe(CString::new("/a/b/execvpe").unwrap().as_ptr(), p_args.as_ptr(), p_env.as_ptr());
    //         TRACKER.reportexecvpe(CString::new("a/b/execvpe").unwrap().as_ptr(), p_args.as_ptr(), p_env.as_ptr());
    //     }
    //     let mut rfile = File::open(&TRACKER.filename)?;
    //     let mut buffer = String::new();
    //     rfile.read_to_string(&mut buffer)?;
    //     assert!(buffer.contains(&format!("{} EXECUTES [\"/a/b/execvpe\",[\"arg1\",\"arg2\",\"arg3\"],[[\"A\",\"B\"],[\"C\",\"D\"],[\"E\",\"F\"]]]\n", TRACKER.uuid)));
    //     assert!(buffer.contains(&format!("{} EXECUTES [\"{}/a/b/execvpe\",[\"arg1\",\"arg2\",\"arg3\"],[[\"A\",\"B\"],[\"C\",\"D\"],[\"E\",\"F\"]]]\n", TRACKER.uuid, TRACKER.cwd)));
    //     assert!(true);
    //     Ok(())
    // }

    #[test]
    fn test_envextractwisk_1() -> io::Result<()> {
        let fields = vec!["WISK_TRACE", "WISK_TRACK", "LD_PRELOAD"];
        let env = vec![("WISK_TRACE", "tracefile"), ("WISK_TRACK", "track.file"), ("LD_PRELOAD", "lib_wisktrack.so"),
                       ("LD_LIBRARY_PATH", "abc1/abc1:abc1/abc1:abc1/abc1:")];
        for (k,v) in env.iter() {
            env::set_var(k,v);
        }
        let wiskmap = envextractwisk(fields);
        
        assert_eq!(env::var("WISK_TRACE"), Err(NotPresent));
        assert_eq!(env::var("WISK_TRACK"), Err(NotPresent));
        assert_eq!(env::var("LD_PRELOAD"), Err(NotPresent));
        Ok(())
    }

    // #[test]
    // fn test_internalopen_1() -> io::Result<()> {
    //     let f = internal_open("/tmp/test1", O_CREAT);
    //     (&f).write_all(format!("Something new\n").as_bytes()).unwrap();
    //     Ok(())
    // }
}

#[cfg(test)]
mod config_tests {
    use std::io;
    use std::env::VarError::NotPresent;
    use std::ffi::{CString};
    use libc::{opendir, dirfd};
    use super::*;

    #[test]
    // #[should_panic]
    fn test_readconfig_doesnotexist() -> io::Result<()> {
        #[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Deserialize)]
        struct Point {
            x: i32,
        }
        debug(format_args!("current_dir: {}\n", std::env::current_dir().unwrap().to_string_lossy()));
        let cpath = std::env::current_dir().unwrap().join("tests/config/libwisktrack.so");
        debug(format_args!("current_dir: {}\n", cpath.to_string_lossy()));
        let x: Result<Point,String> = read_config(cpath.to_str().unwrap(), "doesnotexist.ini");
        match x {
            Ok(v) => assert!(false),
            Err(e) => assert!(e.starts_with("Error finding No such file or directory (os error 2) path:"), e),
        }
        Ok(())
    }


    #[test]
    fn test_readconfig_good() -> io::Result<()> {
        #[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Deserialize)]
        struct Point {
            x: i32,
            #[serde(default)]
            dx: i32,
            v: Vec<String>,
            #[serde(default)]
            dv: Vec<String>,
            h: BTreeMap<String,i32>,
            #[serde(default)]
            dh: BTreeMap<String,i32>,
        }
        debug(format_args!("current_dir: {}\n", std::env::current_dir().unwrap().to_string_lossy()));
        let cpath = std::env::current_dir().unwrap().join("tests/config/libwisktrack.so");
        debug(format_args!("current_dir: {}\n", cpath.to_string_lossy()));
        let x: Result<Point,String> = read_config(cpath.to_str().unwrap(), "testgood.ini");
        assert_eq!(x, Ok(Point {
            x:2, dx:0,
            v: vec!["some 1".to_owned(), "some 2".to_owned(), "some 3".to_owned()],
            dv: vec![],
            h: [("Key 1".to_owned(), 100),("Key 2".to_owned(), 50),("Key 3".to_owned(), 10)].iter().cloned().collect(),
            dh: [].iter().cloned().collect(),
        }));

        Ok(())
    }

    #[test]
    fn test_readconfig_badvalues() -> io::Result<()> {
        #[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Deserialize)]
        struct Point {
            x: i32,
            y: i32,
        }
        debug(format_args!("current_dir: {}\n", std::env::current_dir().unwrap().to_string_lossy()));
        let cpath = std::env::current_dir().unwrap().join("tests/config/libwisktrack.so");
        debug(format_args!("current_dir: {}\n", cpath.to_string_lossy()));
        let x: Result<Point,String> = read_config(cpath.to_str().unwrap(), "testbadvalues.ini");
        match x {
            Ok(v) => assert!(false),
            Err(e) => assert!(e.starts_with(
                "Error parsing x: invalid type: floating point `1`, expected i32 at line 2 column 4 path:"), e),
        }

        Ok(())
    }
}
