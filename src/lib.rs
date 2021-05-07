#![feature(c_variadic)]
#![feature(thread_local)]

#[macro_use]
extern crate redhook;
extern crate core;
extern crate libc;
extern crate tracing;
extern crate ctor;
extern crate paste;
extern crate nix;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate lazy_static;
extern crate tracing_appender;
extern crate tracing_subscriber;
extern crate backtrace;
extern crate string_template;
extern crate regex;
extern crate errno;

#[macro_use]
mod debug;
mod common;
mod tracer;
mod tracker;
mod utils;
mod path;
mod fs;
mod bufwriter;

use std::{env, ptr};
// use tracker::{MY_DISPATCH_initialized, MY_DISPATCH, TRACKER, DEBUGMODE};
use std::ffi::{CStr, CString};
use core::cell::Cell;
use ctor::{ctor, dtor};
use std::sync::atomic;
use paste::paste;
use std::process;
// use std::fs;
use tracing::{Level };
use libc::{c_char,c_int,O_CREAT,O_TMPFILE,SYS_readlink, SYS_open, SYS_close, INT_MAX};
use nix::unistd::dup2;
use nix::unistd::PathconfVar;
use tracing::dispatcher::with_default;
use redhook::{debug, initialized};
use backtrace::Backtrace;
use std::io::{Error, Read, Result, Write};
use std::os::unix::io::FromRawFd;
use errno::{Errno, errno, set_errno};
use common::{WISKFDS, WISKTRACKFD, WISKTRACEFD, WISKTRACE, PUUID, UUID, PID};
use tracer::{TRACER};
use tracker::{WISKMAP, TRACKER, DEBUGMODE, CMDLINE, APP64BITONLY_PATTERNS};

hook! {
    unsafe fn readlink(path: *const libc::c_char, buf: *mut libc::c_char, bufsiz: libc::size_t) -> libc::ssize_t => (my_readlink,SYS_readlink, true) {
        setdebugmode!("readlink");
        if initialized() {
            TRACKER.reportreadlink(path);
        }
        real!(readlink)(path, buf, bufsiz)
    }
}

/* int creat(const char *pathname, mode_t mode); */
hook! {
    unsafe fn creat(pathname: *const libc::c_char, mode: libc::mode_t) -> c_int => (my_creat,-1,true) {

        setdebugmode!("creat");
        if initialized() {
            TRACKER.reportcreat(pathname, mode);
        }
        real!(creat)(pathname, mode)
    }
}

hook! {
    unsafe fn fopen(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => (my_fopen,-1,true) {

        setdebugmode!("fopen");
        if !initialized() {
            setdebugmode!("fopen64");
            real!(fopen)(name, mode)
            // let tempfd: c_int = libc::fileno(f);
            // if tempfd >= 0 {
            //     // debug(format_args!("open64() Duping FD: {}\n", tempfd));
            //     let fd = dup2(tempfd, WISKFD.fetch_add(1,atomic::Ordering::SeqCst) as i32).unwrap();
            //     libc::syscall(SYS_close, tempfd);
            //     // debug(format_args!("open64() Duping FD: {}  -> {}\n", tempfd, fd));
            //     // Set the FD_CLOEXEC flag on our end of the pipe, but not the child end.
            //     let flags = libc::fcntl(fd, libc::F_GETFD);
            //     libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
            //     libc::fdopen(fd, mode)
            // } else {
            //     f
            // }
        } else {
            TRACKER.reportfopen(name, mode);
            real!(fopen)(name, mode)
        }
    }
}

/* #ifdef HAVE_FOPEN64 */
#[cfg(target_arch = "x86_64")]
hook! {
    unsafe fn fopen64(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => (my_fopen64,-1,true) {

        if !initialized() {
            setdebugmode!("fopen64");
            real!(fopen64)(name, mode)
            // let tempfd: c_int = libc::fileno(f);
            // if tempfd >= 0 {
            //     // debug(format_args!("open64() Duping FD: {}\n", tempfd));
            //     let fd = dup2(tempfd, WISKFD.fetch_add(1,atomic::Ordering::SeqCst) as i32).unwrap();
            //     libc::syscall(SYS_close, tempfd);
            //     // debug(format_args!("open64() Duping FD: {}  -> {}\n", tempfd, fd));
            //     // Set the FD_CLOEXEC flag on our end of the pipe, but not the child end.
            //     let flags = libc::fcntl(fd, libc::F_GETFD);
            //     libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
            //     libc::fdopen(fd, mode)
            // } else {
            //     f
            // }
        } else {
            TRACKER.reportfopen(name, mode);
            real!(fopen64)(name, mode)
        }
    }
}
/* #endif */

/* typedef int (*__libc_open)(const char *pathname, int flags, ...); */
dhook! {
    unsafe fn open(args: std::ffi::VaListImpl, pathname: *const c_char, flags: c_int ) -> c_int => (my_open,true) {

        setdebugmode!("open");
        if !initialized() {
            // let tempfd: c_int;

            if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
                let mut ap: std::ffi::VaListImpl = args.clone();
                let mode: c_int = ap.arg::<c_int>();
                real!(open)(pathname, flags, mode)
            } else {
                real!(open)(pathname, flags)
            }
            // if tempfd >= 0 {
            //     // debug(format_args!("open64() Duping FD: {}\n", tempfd));
            //     let fd = dup2(tempfd, WISKFD.fetch_add(1,atomic::Ordering::SeqCst) as i32).unwrap();
            //     libc::syscall(SYS_close, tempfd);
            //     // debug(format_args!("open64() Duping FD: {}  -> {}\n", tempfd, fd));
            //     // Set the FD_CLOEXEC flag on our end of the pipe, but not the child end.
            //     let flags = libc::fcntl(fd, libc::F_GETFD);
            //     libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
            //     fd
            // } else {
            //     tempfd
            // }
        } else {
            if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
                let mut ap: std::ffi::VaListImpl = args.clone();
                let mode: c_int = ap.arg::<c_int>();
                TRACKER.reportopen(pathname,flags,mode);
                real!(open)(pathname, flags, mode)
            } else {
                TRACKER.reportopen(pathname,flags,0);
                real!(open)(pathname, flags)
            }
        }
    }
}

// /*
// #ifdef HAVE_OPEN64
//    typedef int (*__libc_open64)(const char *pathname, int flags, ...);
// #endif */
#[cfg(target_arch = "x86_64")]
dhook! {
    unsafe fn open64(args: std::ffi::VaListImpl, pathname: *const c_char, flags: c_int ) -> c_int => (my_open64, true) {

        if !initialized() {
            let tempfd: c_int;

            if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
                let mut ap: std::ffi::VaListImpl = args.clone();
                let mode: c_int = ap.arg::<c_int>();
                real!(open64)(pathname, flags, mode)
            } else {
                real!(open64)(pathname, flags)
            }
            // if tempfd >= 0 {
            //     // debug(format_args!("open64() Duping FD: {}\n", tempfd));
            //     let fd = dup2(tempfd, WISKFD.fetch_add(1,atomic::Ordering::SeqCst) as i32).unwrap();
            //     libc::syscall(SYS_close, tempfd);
            //     // debug(format_args!("open64() Duping FD: {}  -> {}\n", tempfd, fd));
            //     // Set the FD_CLOEXEC flag on our end of the pipe, but not the child end.
            //     let flags = libc::fcntl(fd, libc::F_GETFD);
            //     libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
            //     fd
            // } else {
            //     tempfd
            // }
        } else {
            setdebugmode!("open64");
            if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
                let mut ap: std::ffi::VaListImpl = args.clone();
                let mode: c_int = ap.arg::<c_int>();
                TRACKER.reportopen(pathname,flags,mode);
                real!(open64)(pathname, flags, mode)
            } else {
                TRACKER.reportopen(pathname,flags,0);
                real!(open64)(pathname, flags)
            }
        }
    }
}

// /*
// #ifdef HAVE_OPEN64
//    typedef int close(int fd);;
// #endif */
hook! {
    unsafe fn close(fd : c_int ) -> c_int => my_close {
        // We cannot do any mallocsas this can cause recursion, if this code is executed
        // from a signal handler closing files and cleaning up a exited/forked process, while
        // the main thread is also in malloc.
        // setdebugmode!("close");
        // event!(Level::INFO, "close({})", fd);
        if WISKFDS.read().unwrap().iter().any(|&i| i==fd) {
            0
        } else if !initialized() {
            real!(close)(fd)
        } else {
            // debug(format_args!("close({})\n", fd));
            // event!(Level::INFO, "close---({}, {})", &UUID.as_str(), fd);
            // tracker::reportclose(fd);
            real!(close)(fd)
         }
    }
}


// /* typedef int (*__libc_openat)(int dirfd, const char *path, int flags, ...); */
// dhook! {
//     unsafe fn openat(args: std::ffi::VaListImpl, dirfd: c_int, pathname: *const c_char, flags: c_int ) -> c_int => (my_openat,true) {
//         if (flags & O_CREAT) == O_CREAT {
//             let mut ap: std::ffi::VaListImpl = args.clone();
//             let mode: c_int = ap.arg::<c_int>();
//             TRACKER.reportopen(pathname,flags,mode);
//             real!(openat)(dirfd, pathname, flags, mode)
//         } else {
//             setdebugmode!("openat");
//             TRACKER.reportopen(pathname,flags,0);
//             real!(openat)(dirfd, pathname, flags)
//         }
//     }
// }

/* 
typedef int (*__libc_fcntl)(int fd, int cmd, ...);

 */



fn execvpe_common (
    file: *const libc::c_char, argv: *const *const libc::c_char,
    envcstr: Vec<CString>, search: bool) -> c_int {
    let mut envp = utils::vcstr2vecptr(&envcstr);
    envp.push(std::ptr::null());
    let mut e = errno();
    let cwd = if let Ok(cwd) = env::current_dir() {
        cwd.to_str().unwrap().to_owned()
    } else {
        String::new()
    };
    let ld_preload = CString::new(
                         envcstr[0].as_c_str().to_str().unwrap()
                                   .replace("lib64/libwisktrack.so",
                                            "${LIB}/libwisktrack.so")).unwrap();
    let ld_preload_64bit = CString::new(
                               ld_preload.to_str().unwrap()
                                         .replace("${LIB}/libwisktrack.so",
                                                  "lib64/libwisktrack.so")).unwrap();

    let file = unsafe {
        assert!(!file.is_null());
        CStr::from_ptr(file) 
    };
    let file_bytes = file.to_bytes();
    /* We check the simple case first. */
    if (file_bytes.len() == 0) {
        e.0 = libc::ENOENT;
        set_errno(e);
        return -1;
    }
    let argvold = argv;
    let argv = utils::cpptr2vecptr(argv);
     /* Don't search when it contains a slash.  */
    if ( !search || file_bytes.iter().any(|i| *i as char == '/'))  { //. strchr (file, '/') != NULL)
        if path::is_match(file.to_str().unwrap(), &APP64BITONLY_PATTERNS, cwd.as_str()) {
            envp[0] = ld_preload_64bit.as_ptr();
            // TRACKER.report("DEBUGDATA_64ONLY", &serde_json::to_string(&CMDLINE.to_vec()).unwrap());
            // TRACKER.report("APP64ONLY", &utils::cpptr2str(argvold, " "));
            utils::assert_ld_preload(&envp, true);
        } else {
            envp[0] = ld_preload.as_ptr();
            // TRACKER.report("DEBUGDATA", &serde_json::to_string(&CMDLINE.to_vec()).unwrap());
            utils::assert_ld_preload(&envp, false);
        }
        utils::assert_execenv(&envp, &PUUID);
        let rv = unsafe { real!(execve)(file.as_ptr(), argv.as_ptr(), envp.as_ptr()) };
        return rv;
    }

    let path = match env::var("PATH") {
        Ok(path) => path,
        Err(_) => "/bin:/usr/bin".to_owned(),
    };

    let mut got_eacces: bool = false;
    /* The resulting string maximum size would be potentially a entry
       in PATH plus '/' (path_len + 1) and then the the resulting file name
       plus '\0' (file_len since it already accounts for the '\0').  */
    for p in path.split(":") {
        let mut p = p.to_owned();
        p.push_str("/");
        p.push_str(file.to_str().unwrap());
        let buffer = CString::new(p.as_str()).expect("Trouble Mapping executable String to CString");
        if path::is_match(buffer.to_str().unwrap(), &APP64BITONLY_PATTERNS, cwd.as_str()) {
            envp[0] = ld_preload_64bit.as_ptr();
            // TRACKER.report("DEBUGDATA_64ONLY", &serde_json::to_string(&CMDLINE.to_vec()).unwrap());
            // TRACKER.report("DEBUGDATA_64ONLY", &utils::cpptr2str(argvold, " "));
            utils::assert_ld_preload(&envp, true);
            // panic!();
        } else {
            envp[0] = ld_preload.as_ptr();
            // TRACKER.report("DEBUGDATA", &serde_json::to_string(&CMDLINE.to_vec()).unwrap());
            utils::assert_ld_preload(&envp, false);
        }
        utils::assert_execenv(&envp, &PUUID);
        let rv = unsafe { real!(execve)(buffer.as_ptr(), argv.as_ptr(), envp.as_ptr()) };
        match errno().0 {
            libc::EACCES => {
                /* Record that we got a 'Permission denied' error.  If we end
                up finding no executable we can use, we want to diagnose
                that we did find one but were denied access.  */
                got_eacces = true;                
            },
            libc::ENOENT | libc::ESTALE | libc::ENOTDIR => { },
                /* Those errors indicate the file is missing or not executable
                by us, in which case we want to just try the next path
                directory.  */
            libc::ENODEV | libc::ETIMEDOUT => { },
                /* Some strange filesystems like AFS return even
                stranger error numbers.  They cannot reasonably mean
                anything else so ignore those, too.  */
            _ => {
                /* Some other error means we found an executable file, but
                    something went wrong executing it; return the error to our
                    caller.  */
                return -1;
            }
        }
    }
    /* We tried every element and none of them worked.  */
    if (got_eacces)  {
        /* At least one failure was due to permissions, so report that
           error.  */
        e.0 = libc::EACCES;
        set_errno(e);
    }
    return -1;
   }

// typedef int (*__libc_execl)(const char *path, char *const arg, ...);

 dhook! {
    unsafe fn execl(args: std::ffi::VaListImpl, path: *const c_char) -> c_int => my_execl {
        setdebugmode!("execl");
        let mut ap: std::ffi::VaListImpl = args.clone();
        let mut vecptrs: Vec<_> =  vec!();
        while true {
            let arg: *const c_char = ap.arg::<* const c_char>();
            if arg.is_null() {
                vecptrs.push(arg);
                break;
            }
            vecptrs.push(arg);
        }
        let argv = vecptrs.as_ptr();
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        TRACKER.reportexecvpe("execl", path, argv, &envcstr);
        execvpe_common(path, argv, envcstr, false)
    }
}

//typedef int (*__libc_execlp)(const char *file, char *const arg, ...);

dhook! {
    unsafe fn execlp(args: std::ffi::VaListImpl, file: *const c_char) -> c_int => my_execlp {
        setdebugmode!("execlp");
        let mut ap: std::ffi::VaListImpl = args.clone();
        let mut vecptrs: Vec<_> =  vec!();
        while true {
            let arg: *const c_char = ap.arg::<* const c_char>();
            if arg.is_null() {
                vecptrs.push(arg);
                break;
            }
            vecptrs.push(arg);
        }
        let argv = vecptrs.as_ptr();
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        TRACKER.reportexecvpe("execlp", file, argv, &envcstr);
        execvpe_common(file, argv, envcstr, true)
    }
}

// typedef int (*__libc_execlpe)(const char *path, const char *arg,..., char * const envp[]);

dhook! {
    unsafe fn execle(args: std::ffi::VaListImpl, path: *const c_char) -> c_int => my_execle {
        setdebugmode!("execle");
        let mut ap: std::ffi::VaListImpl = args.clone();
        let mut vecptrs: Vec<_> =  vec!();
        while true {
            let arg: *const c_char = ap.arg::<* const c_char>();
            if arg.is_null() {
                vecptrs.push(arg);
                break;
            }
            vecptrs.push(arg);
        }
        let argv = vecptrs.as_ptr();
        let envp: *const *const libc::c_char = ap.arg::<*const *const libc::c_char>();
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        TRACKER.reportexecvpe("execle", path, argv, &envcstr);
        execvpe_common(path, argv, envcstr, false)
    }
}


 /* int execv(const char *path, char *const argv[]); */

 hook! {
    unsafe fn execv(path: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => (my_execv,-1,true) {
        setdebugmode!("execv");
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        TRACKER.reportexecvpe("execv", path, argv, &envcstr);
        execvpe_common(path, argv, envcstr, false)
    }
}

 /* int execvp(const char *file, char *const argv[]); */

hook! {
    unsafe fn execvp(file: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => (my_execvp,-1,true) {
        setdebugmode!("execvp");
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        TRACKER.reportexecvpe("execvp", file, argv, &envcstr);
        execvpe_common(file, argv, envcstr, true)
    }
}

/* int execvpe(const char *file, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execvpe(file: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_execvpe,-1,true) {
        setdebugmode!("execvpe");
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        TRACKER.reportexecvpe("execv", file, argv, &envcstr);
        execvpe_common(file, argv, envcstr, true)
    }
}


/* int execve(const char *pathname, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execve(pathname: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_execve,-1,true) {
        setdebugmode!("execve");
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        TRACKER.reportexecvpe("execve", pathname, argv, &envcstr);
        execvpe_common(pathname, argv, envcstr, false)
    }
}


 /* int execveat(int dirfd, const char *pathname, char *const argv[], char *const envp[], int flags); */

// hook! {
//     unsafe fn execveat(dirfd: libc::c_int, pathname: *const libc::c_char,
//                        argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_execveat,-1,true) {
//         setdebugmode!("execveat");
//         // TRACKER.reportexecvpe(pathname, argv, envp);
//         // TRACKER.reportexecveat(dirfd, pathname, argv, envp);
//         let mut env = utils::cpptr2hashmap(envp);
//         utils::envupdate(&mut env,&TRACKER.wiskfields);
//         utils::hashmapassert(&env, vec!("LD_PRELOAD"));
//         let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
//         let mut envp = utils::vcstr2vecptr(&envcstr);
//         envp.push(std::ptr::null());
//         real!(execveat)(dirfd, pathname, argv, envp.as_ptr())
//     }
// }


 /* int posix_spawn(pid_t *pid, const char *path, const posix_spawn_file_actions_t *file_actions,
                    const posix_spawnattr_t *attrp, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn posix_spawn(pid: *mut libc::pid_t, path: *const libc::c_char, file_actions: *const libc::posix_spawn_file_actions_t,
                           attrp: *const libc::posix_spawnattr_t, argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_posix_spawn,-1,true) {
        setdebugmode!("posix_spawn");
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        let mut envp = utils::vcstr2vecptr(&envcstr);
        envp.push(std::ptr::null());
        let ld_preload = envcstr[0].as_c_str().to_owned();
        let ld_preload_64bit = CString::new(
                                   envcstr[0].as_c_str().to_str().unwrap()
                                             .replace("${LIB}/libwisktrack.so",
                                                      "lib64/libwisktrack.so")).unwrap();
        let pathstr = CStr::from_ptr(path);
        if path::is_match(pathstr.to_str().unwrap(), &APP64BITONLY_PATTERNS, "") {
            envp[0] = ld_preload_64bit.as_ptr();
            utils::assert_ld_preload(&envp, true);
        } else {
            envp[0] = ld_preload.as_ptr();
            utils::assert_ld_preload(&envp, false);
        }
        utils::assert_execenv(&envp, &PUUID);

        TRACKER.reportexecvpe("posix_spawn", path, argv, &envcstr);
        real!(posix_spawn)(pid, path, file_actions, attrp, argv, envp.as_ptr())
    }
}

/* int posix_spawnp(pid_t *pid, const char *file, const posix_spawn_file_actions_t *file_actions,
                    const posix_spawnattr_t *attrp, char *const argv[], char * const envp[]); */

hook! {
    unsafe fn posix_spawnp(pid: *mut libc::pid_t, file: *const libc::c_char, file_actions: *const libc::posix_spawn_file_actions_t,
                           attrp: *const libc::posix_spawnattr_t, argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_posix_spawnp,-1,true) {
        setdebugmode!("posix_spawnp");

        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        let mut envp = utils::vcstr2vecptr(&envcstr);
        envp.push(std::ptr::null());
        TRACKER.reportexecvpe("posix_spawnp", file, argv, &envcstr);
        real!(posix_spawnp)(pid, file, file_actions, attrp, argv, envp.as_ptr())
    }
}


/* FILE popen(const char *command, const char *type); */

hook! {
    unsafe fn popen(command: *const libc::c_char, ctype: *const libc::c_char) -> *const libc::FILE => (my_popen,-1,true) {
        setdebugmode!("popen");
        TRACKER.reportpopen(command, ctype);
        utils::currentenvupdate(&WISKMAP);
        let x = real!(popen)(command, ctype);
        x
    }
}

/*  int system (const char *command) */

hook! {
    unsafe fn system(command: *const libc::c_char) -> libc::c_int => my_system {
        setdebugmode!("system");
        TRACKER.reportsystem(command);
        utils::currentenvupdate(&WISKMAP);
        let x = real!(system)(command);
        x
    }
}

/* int rename(const char *old, const char *new); */
hook! {
    unsafe fn rename(old: *const libc::c_char, new: *const libc::c_char) -> libc::c_int => (my_rename,-1,true) {
        setdebugmode!("rename");
        if initialized() {
            TRACKER.reportrename(old, new);
        }
        real!(rename)(old, new)
    }
}


/* int renameat(int olddirfd, const char *oldpath, int newdirfd, const char *newpath); */
hook! {
    unsafe fn renameat(olddirfd: libc::c_int, oldpath: *const libc::c_char, newdirfd: libc::c_int, newpath: *const libc::c_char) -> libc::c_int => (my_renameat,-1,true) {
        setdebugmode!("renameat");
        if initialized() {
            TRACKER.reportrenameat(olddirfd, oldpath, newdirfd, newpath);
        }
        real!(renameat)(olddirfd, oldpath, newdirfd, newpath)
    }
}


/* int renameat2(int olddirfd, const char *oldpath, int newdirfd, const char *newpath, unsigned int flags); */
hook! {
    unsafe fn renameat2(olddirfd: libc::c_int, oldpath: *const libc::c_char, newdirfd: libc::c_int, newpath: *const libc::c_char, flags: libc::c_int) -> libc::c_int => (my_renameat2,-1,true) {
        setdebugmode!("renameat2");
        if initialized() {
            TRACKER.reportrenameat2(olddirfd, oldpath, newdirfd, newpath, flags);
        }
        real!(renameat2)(olddirfd, oldpath, newdirfd, newpath, flags)
    }
}


/* int symlink(const char *target, const char *linkpath); */
hook! {
    unsafe fn symlink(target: *const libc::c_char, linkpath: *const libc::c_char) -> libc::c_int => (my_symlink,-1,true) {
        setdebugmode!("symlink");
        if initialized() {
            TRACKER.reportsymlink(target, linkpath);
        }
        real!(symlink)(target, linkpath)
    }
}

/* int symlinkat(const char *target, int newdirfd, const char *linkpath); */
hook! {
    unsafe fn symlinkat(target: *const libc::c_char, newdirfd: libc::c_int, linkpath: *const libc::c_char) -> libc::c_int => (my_symlinkat,-1,true) {
        setdebugmode!("symlinkat");
        if initialized() {
            TRACKER.reportsymlinkat(target, newdirfd, linkpath);
        }
        real!(symlinkat)(target, newdirfd, linkpath)
    }
}

/* int link(const char *oldpath, const char *newpath); */
hook! {
    unsafe fn link(oldpath: *const libc::c_char, newpath: *const libc::c_char) -> libc::c_int => (my_link,-1,true) {
        setdebugmode!("link");
        if initialized() {
            TRACKER.reportlink(oldpath, newpath);
        }
        real!(link)(oldpath, newpath)
    }
}

/* int linkat(int olddirfd, const char *oldpath, int newdirfd, const char *newpath, int flags); */
hook! {
    unsafe fn linkat(olddirfd: libc::c_int, oldpath: *const libc::c_char,
                     newdirfd: libc::c_int, newpath: *const libc::c_char, flags: libc::c_int) -> libc::c_int => (my_linkat,-1,true) {
        setdebugmode!("linkat");
        if initialized() {
            TRACKER.reportlinkat(olddirfd, oldpath, newdirfd, newpath, flags);
        }
        real!(linkat)(olddirfd, oldpath, newdirfd, newpath, flags)
    }
}

/* int unlink(const char *pathname); */
hook! {
    unsafe fn unlink(pathname: *const libc::c_char) -> libc::c_int => (my_unlink,-1,true) {
        setdebugmode!("unlink");
        if initialized() {
            TRACKER.reportunlink(pathname);
        }
        real!(unlink)(pathname)
    }
}

/* int unlinkat(int dirfd, const char *pathname, int flags); */
hook! {
    unsafe fn unlinkat(dirfd: libc::c_int, pathname: *const libc::c_char, flags: libc::c_int) -> libc::c_int => (my_unlinkat,-1,true) {
        setdebugmode!("unlinkat");
        if initialized() {
            TRACKER.reportunlinkat(dirfd, pathname, flags);
        }
        real!(unlinkat)(dirfd, pathname, flags)
    }
}

/* int chmod(__const char *__file, __mode_t __mode); */
hook! {
    unsafe fn chmod(pathname: *const libc::c_char, mode: libc::mode_t) -> libc::c_int => (my_chmod,-1,true) {
        setdebugmode!("chmod");
        if initialized() {
            TRACKER.reportchmod(pathname, mode);
        }
        real!(chmod)(pathname, mode)
    }
}

/* int fchmod(int __fd, __mode_t __mode); */
hook! {
    unsafe fn fchmod(fd: libc::c_int, mode: libc::mode_t) -> libc::c_int => (my_fchmod,-1,true) {
        setdebugmode!("fchmod");
        if initialized() {
            TRACKER.reportfchmod(fd, mode);
        }
        real!(fchmod)(fd, mode)
    }
}

/* int fchmodat(int __fd, __const char *__file, __mode_t __mode, int flags); */
hook! {
    unsafe fn fchmodat(dirfd: libc::c_int, pathname: *const libc::c_char, mode: libc::mode_t, flags: libc::c_int) -> libc::c_int => (my_fchmodat,-1,true) {
        setdebugmode!("fchmodat");
        if initialized() {
            TRACKER.reportfchmodat(dirfd, pathname, mode, flags);
        }
        real!(fchmodat)(dirfd, pathname, mode, flags)
    }
}

/* pid_t wait(int *status); */
hook! {
    unsafe fn wait(status: *const libc::c_int) -> libc::pid_t => (my_wait,-1,true) {
        let localpid: libc::pid_t;
        let localstatus: *const libc::c_int;
        // setdebugmode!("wait");
        if status.is_null() {
            let stat: libc::c_int = 0;
            localstatus = &stat;
            localpid = real!(wait)(localstatus);
        } else {
            localstatus = status;
            localpid = real!(wait)(localstatus);
        }
        if libc::WCOREDUMP(*localstatus) && localpid > 0 {
            // if initialized() {
                TRACKER.reportcoredumped("wait", localpid);
            // }
        }
        localpid
    }
}


/* pid_t waitpid(pid_t pid, int *status, int options); */
hook! {
    unsafe fn waitpid(pid: libc::pid_t, status: *const libc::c_int, options: libc::c_int) -> libc::pid_t => (my_waitpid,-1,true) {
        let localpid: libc::pid_t;
        let localstatus: *const libc::c_int;
        // setdebugmode!("waitpid");
        if status.is_null() {
            let stat: libc::c_int = 0;
            localstatus = &stat;
            localpid = real!(waitpid)(pid, localstatus, options);
        } else {
            localstatus = status;
            localpid = real!(waitpid)(pid, localstatus, options);
        }
        if libc::WCOREDUMP(*localstatus) && localpid > 0 {
            // if initialized() {
                TRACKER.reportcoredumped("waitpid", localpid);
            // }
        }
        localpid
    }
}

/* int waitid(idtype_t idtype, id_t id, siginfo_t *infop, int options); */
hook! {
    unsafe fn waitid(pid: libc::pid_t, id: libc::id_t, infop: *const libc::siginfo_t, options: libc::c_int) -> libc::c_int => (my_waitid,-1,true) {
        let localpid: libc::pid_t;
        let localstatus: *const libc::c_int;
        // setdebugmode!("waitid");
        localpid = real!(waitid)(pid, id, infop, options);
        if libc::WCOREDUMP((*infop).si_status()) && localpid > 0 {
            // if initialized() {
                TRACKER.reportcoredumped("waitid", localpid);
            // }
        }
        localpid
    }
}

// vhook! {
//     unsafe fn vprintf(args: std::ffi::VaList, format: *const c_char ) -> c_int => my_vprintf {
//         event!(Level::INFO, "vprintf({})", CStr::from_ptr(format).to_string_lossy());
//         real!(vprintf)(format, args)
//     }
// }


// dhook! {
//     unsafe fn printf(args: std::ffi::VaListImpl, format: *const c_char ) -> c_int => my_printf {
//         // event!(Level::INFO, "printf({}, {})", *&TRACKER.uuid,
//         //        CStr::from_ptr(format).to_string_lossy());
//         let mut aq: std::ffi::VaListImpl;
//         aq  =  args.clone();
//         my_vprintf(format, aq.as_va_list())
//     }
// }

fn dlsym_intialize() {
    real!(readlink);
    real!(creat);
    real!(open);
    // real!(openat);
    real!(fopen);
    #[cfg(target_arch = "x86_64")]
    {
        real!(open64);
        real!(fopen64);
    }
    real!(execv);
    real!(execvp);
    real!(execvpe);
    real!(execve);
    // real!(execveat);
    real!(posix_spawn);
    real!(posix_spawnp);
    real!(popen);
    real!(link);
    real!(linkat);
    real!(symlink);
    real!(symlinkat);
    real!(unlink);
    real!(unlinkat);
    real!(chmod);
    real!(fchmod);
    real!(fchmodat);
    real!(wait);
    real!(waitpid);
    real!(waitid);
}

#[cfg(not(test))]
#[ctor]
fn cfoo() {
    cevent!(Level::INFO, "Constructor: {}\n", std::process::id());
    cevent!(Level::INFO, "Incoming Environment: {:?}\n", env::vars_os().map(|(x,y)| x.to_str().unwrap().to_owned()).collect::<Vec<String>>());
    dlsym_intialize();
    tracker::initialize_constructor_statics();
    redhook::initialize();
    cevent!(Level::INFO, "Constructor Done: {}, {}\n", std::process::id(), serde_json::to_string(&CMDLINE.to_vec()).unwrap());
}

#[cfg(not(test))]
#[dtor]
fn dfoo() {
    cevent!(Level::INFO, "Destructor Done: {}, {}\n", std::process::id(), serde_json::to_string(&CMDLINE.to_vec()).unwrap());
    (&TRACKER.file).flush().unwrap();
}



#[cfg(test)]
mod libtests {
    use std::ffi::CString;

    extern {
        fn chmod(file: *const libc::c_char, mode: libc::mode_t);
        fn printf(format: *const libc::c_char , ...);
    }



    #[test]
    fn lib_mytests() {
        // Statements here are executed when the compiled binary is called

        // Print text to the console
        let file = CString::new("/tmp/file1 %d\n").expect("CString::new failed");
        unsafe {
            printf(file.as_ptr(), 0);
            chmod(file.as_ptr(), 0);
        }
    }
}
