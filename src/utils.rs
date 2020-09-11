
use std::ffi::{CStr, CString};
use std::{env, ptr};
use libc::{c_char};
use std::collections::HashMap;

pub fn vcstr2vecptr(vcstr: &Vec<CString>) -> Vec<*const c_char> {
    let vecptr: Vec<_> = vcstr.iter() // do NOT into_iter()
                              .map(|arg| arg.as_ptr())
                              .collect();
    vecptr
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

pub fn cpptr2hashmap(vecptr: *const *const libc::c_char) -> HashMap<String,String> {
    let mut hash: HashMap<String,String> = HashMap::new();
    for i in 0 .. {
        unsafe {
            let argptr: *const c_char = *(vecptr.offset(i));
            if argptr != ptr::null() {
                let kv:Vec<&str> = CStr::from_ptr(argptr).to_str().unwrap().splitn(1,"=").collect();
                hash.insert(kv[0].to_string(), kv[1].to_string());
            } else {
                break;
            }    
        }
    }
    hash
}

pub fn hashmap2vcstr(hash: &HashMap<String,String>) -> Vec<CString> {
    let x:Vec<CString> = hash.iter()
                             .map(|(k,v)| CString::new(format!("{}={}",k,v)).unwrap())
                             .collect();
    x
}

pub fn envupdate(env: &mut HashMap<String,String>, fields: &Vec<(String,String)>) {
    for (k,v) in fields.iter() {
        if k == "LD_PRELOAD" {
            env.insert(k.to_string(),v.to_string());
        } else if k == "LD_LIBRARY_PATH" {
            env.insert(k.to_string(),v.to_string());
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


#[cfg(test)]
mod env_tests {
    use std::io;
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
}
