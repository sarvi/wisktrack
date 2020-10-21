#![feature(c_variadic)]

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

mod tracker;
mod utils;
mod path;

use std::{env, ptr};
// use tracker::{MY_DISPATCH_initialized, MY_DISPATCH, TRACKER, DEBUGMODE};
use std::ffi::{CStr, CString};
use core::cell::Cell;
use ctor::{ctor, dtor};
use paste::paste;
use tracing::{Level, event, };
use libc::{c_char,c_int,O_CREAT,O_TMPFILE,SYS_readlink, SYS_open, INT_MAX};
use nix::unistd::dup2;
use nix::unistd::PathconfVar;
use tracing::dispatcher::with_default;
use redhook::{debug, initialized};
use backtrace::Backtrace;
use std::io::{Error, Read, Result, Write};
use errno::{Errno, errno, set_errno};
use utils::WISKFD;
use tracker::{TRACKERFD, WISK_FDS, WISKMAP, TRACKER, DEBUGMODE, CMDLINE, UUID,
              APP64BITONLY_PATTERNS};


hook! {
    unsafe fn readlink(path: *const libc::c_char, buf: *mut libc::c_char, bufsiz: libc::size_t) -> libc::ssize_t => (my_readlink,SYS_readlink, true) {
        setdebugmode!("readlink");
        event!(Level::INFO, "readlink({}, {})", &UUID.as_str(), CStr::from_ptr(path).to_string_lossy());
        TRACKER.reportreadlink(path);
        real!(readlink)(path, buf, bufsiz)
    }
}

/* int creat(const char *pathname, mode_t mode); */
hook! {
    unsafe fn creat(pathname: *const libc::c_char, mode: libc::mode_t) -> c_int => (my_creat,-1,true) {
        setdebugmode!("creat");
        event!(Level::INFO, "creat({}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy());
        TRACKER.reportcreat(pathname, mode);
        real!(creat)(pathname, mode)
    }
}

hook! {
    unsafe fn fopen(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => (my_fopen,-1,true) {
        setdebugmode!("fopen");
        event!(Level::INFO, "fopen({}, {})", &UUID.as_str(), CStr::from_ptr(name).to_string_lossy());
        TRACKER.reportfopen(name, mode);
        real!(fopen)(name, mode)
    }
}

/* #ifdef HAVE_FOPEN64 */
#[cfg(target_arch = "x86_64")]
hook! {
    unsafe fn fopen64(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => (my_fopen64,-1,true) {
        setdebugmode!("fopen64");
        event!(Level::INFO, "fopen64({}, {})", &UUID.as_str(), CStr::from_ptr(name).to_string_lossy());
        TRACKER.reportfopen(name, mode);
        real!(fopen64)(name, mode)
    }
}
/* #endif */

/* typedef int (*__libc_open)(const char *pathname, int flags, ...); */
dhook! {
    unsafe fn open(args: std::ffi::VaListImpl, pathname: *const c_char, flags: c_int ) -> c_int => (my_open,true) {
        setdebugmode!("open");
        if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
            let mut ap: std::ffi::VaListImpl = args.clone();
            let mode: c_int = ap.arg::<c_int>();
            event!(Level::INFO, "open({}, {}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags, mode);
            TRACKER.reportopen(pathname,flags,mode);
            real!(open)(pathname, flags, mode)
        } else {
            event!(Level::INFO, "open({}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags);
            TRACKER.reportopen(pathname,flags,0);
            real!(open)(pathname, flags)
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
                // debug(format_args!("open64({}, {:X}, {:X})\n", CStr::from_ptr(pathname).to_string_lossy(), flags, mode));
                event!(Level::INFO, "open64({}, {}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags, mode);
                tempfd = real!(open64)(pathname, flags, mode)
            } else {
                // debug(format_args!("open64({}, {:X})\n", CStr::from_ptr(pathname).to_string_lossy(), flags));
                event!(Level::INFO, "open64({}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags);
                tempfd = real!(open64)(pathname, flags);
            }
            if tempfd >= 0 {
                // debug(format_args!("open64() Duping FD: {}\n", tempfd));
                let fd = dup2(tempfd, TRACKERFD+tempfd).unwrap();
                // debug(format_args!("open64() Duping FD: {}  -> {}\n", tempfd, fd));
                WISK_FDS.lock().unwrap().push(fd);
                // Set the FD_CLOEXEC flag on our end of the pipe, but not the child end.
                let flags = libc::fcntl(fd, libc::F_GETFD);
                libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
                fd
            } else {
                tempfd
            }
        } else {
            setdebugmode!("open64");
            if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
                let mut ap: std::ffi::VaListImpl = args.clone();
                let mode: c_int = ap.arg::<c_int>();
                // debug(format_args!("open64({}, {:X}, {:X})\n", CStr::from_ptr(pathname).to_string_lossy(), flags, mode));
                event!(Level::INFO, "open64({}, {}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags, mode);
                TRACKER.reportopen(pathname,flags,mode);
                real!(open64)(pathname, flags, mode)
            } else {
                // debug(format_args!("open64({}, {}, {:X})\n", &UUID.as_str(),CStr::from_ptr(pathname).to_string_lossy(), flags));
                event!(Level::INFO, "open64({}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags);
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
        if ! WISK_FDS.lock().unwrap().iter().any(|&i| i==fd) {
        // if fd != TRACKER.fd {
            // debug(format_args!("close({})\n", fd));
            event!(Level::INFO, "close({}, {})", &UUID.as_str(), fd);
            // TRACKER.reportclose(fd);
            real!(close)(fd)
        } else {
            // debug(format_args!("close({}) -----> Skipping\n", fd));
            0
        }
    }
}


// /* typedef int (*__libc_openat)(int dirfd, const char *path, int flags, ...); */
// dhook! {
//     unsafe fn openat(args: std::ffi::VaListImpl, dirfd: c_int, pathname: *const c_char, flags: c_int ) -> c_int => (my_openat,true) {
//         if (flags & O_CREAT) == O_CREAT {
//             let mut ap: std::ffi::VaListImpl = args.clone();
//             let mode: c_int = ap.arg::<c_int>();
//             event!(Level::INFO, "openat({}, {}, {}, {}, {})", &UUID.as_str(), dirfd, CStr::from_ptr(pathname).to_string_lossy(), flags, mode);
//             TRACKER.reportopen(pathname,flags,mode);
//             real!(openat)(dirfd, pathname, flags, mode)
//         } else {
//             setdebugmode!("openat");
//             event!(Level::INFO, "openat({}, {}, {}, {})", &UUID.as_str(), dirfd, CStr::from_ptr(pathname).to_string_lossy(), flags);
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
    let cwd = env::current_dir().unwrap();
    let ld_preload = envcstr[0].as_c_str().to_owned();
    let ld_preload_64bit = CString::new(
                               envcstr[0].as_c_str().to_str().unwrap()
                                         .replace("${LIB}/libwisktrack.so",
                                                  "lib/libwisktrack.so")).unwrap();

    let cwdstr= cwd.to_str().unwrap();
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
        if path::is_match(file.to_str().unwrap(), &APP64BITONLY_PATTERNS, &cwdstr) {
            envp[0] = ld_preload_64bit.as_ptr();
            // TRACKER.report("DEBUGDATA_64ONLY", &serde_json::to_string(&CMDLINE.to_vec()).unwrap());
            // TRACKER.report("APP64ONLY", &utils::cpptr2str(argvold, " "));
            utils::assert_ld_preload(&envp, true);
        } else {
            envp[0] = ld_preload.as_ptr();
            // TRACKER.report("DEBUGDATA", &serde_json::to_string(&CMDLINE.to_vec()).unwrap());
            utils::assert_ld_preload(&envp, false);        }
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
        if path::is_match(buffer.to_str().unwrap(), &APP64BITONLY_PATTERNS, &cwdstr) {
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
        // debug(format_args!("execl({}):\n", CStr::from_ptr(path).to_string_lossy()));
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

        event!(Level::INFO, "execl({}, {}, {})", &UUID.as_str(), CStr::from_ptr(path).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecv(path, argv);
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        // debug(format_args!("execv(): {:?}\n", env));
        event!(Level::INFO,"execl: {}: Updated Env {:?}", &UUID.as_str(), env);

        execvpe_common(path, argv, envcstr, false)
    }
}

//typedef int (*__libc_execlp)(const char *file, char *const arg, ...);

dhook! {
    unsafe fn execlp(args: std::ffi::VaListImpl, file: *const c_char) -> c_int => my_execlp {
        setdebugmode!("execlp");
        // debug(format_args!("execlp({}):\n", CStr::from_ptr(file).to_string_lossy()));
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
        event!(Level::INFO, "execlp({}, {}, {})", &UUID.as_str(), CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecvp(file, argv);
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        // debug(format_args!("execv=lp(): {:?}\n", env));
        event!(Level::INFO,"execlp: {}: Updated Env {:?}", &UUID.as_str(), env);

        execvpe_common(file, argv, envcstr, true)
    }
}

// typedef int (*__libc_execlpe)(const char *path, const char *arg,..., char * const envp[]);

dhook! {
    unsafe fn execle(args: std::ffi::VaListImpl, path: *const c_char) -> c_int => my_execle {
        setdebugmode!("execle");
        // debug(format_args!("execle({}):\n", CStr::from_ptr(path).to_string_lossy()));
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
        // debug(format_args!("execle(): {:?}\n", env));
        // event!(Level::INFO,"execle: {}: Updated Env {:?}", TRACKER.uuid, env);

        event!(Level::INFO, "execle({}, {}, {})", &UUID.as_str(), CStr::from_ptr(path).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecv(path, argv);

        execvpe_common(path, argv, envcstr, false)
    }
}


 /* int execv(const char *path, char *const argv[]); */

 hook! {
    unsafe fn execv(path: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => (my_execv,-1,true) {
        setdebugmode!("execv");
        // debug(format_args!("execv({}, {})\n", CStr::from_ptr(path).to_string_lossy(), utils::cpptr2str(argv, " ")));
        event!(Level::INFO, "execv({}, {}, \"{}\"F)", &UUID.as_str(), CStr::from_ptr(path).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecv(path, argv);
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        // debug(format_args!("execv(): {:?}\n", env));
        event!(Level::INFO,"execv: {}: Updated Env {:?}", &UUID.as_str(), env);

        execvpe_common(path, argv, envcstr, false)
    }
}

 /* int execvp(const char *file, char *const argv[]); */

hook! {
    unsafe fn execvp(file: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => (my_execvp,-1,true) {
        setdebugmode!("execvp");
        // debug(format_args!("execvp({}, {}, {})\n", std::process::id(), CStr::from_ptr(file).to_string_lossy(),utils::cpptr2str(argv, ",")));
        event!(Level::INFO, "execvp({}, {}, \"{}\"F)", &UUID.as_str(), CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, ","));
        TRACKER.reportexecvp(file, argv);
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        // debug(format_args!("execvp: Updated Env {:?}", env));
        event!(Level::INFO,"execvp: {}: Updated Env {:?}", &UUID.as_str(), env);

        execvpe_common(file, argv, envcstr, true)
    }
}

/* int execvpe(const char *file, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execvpe(file: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_execvpe,-1,true) {
        setdebugmode!("execvpe");
        // debug(format_args!("execvpe({}, {})\n", CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " ")));
        event!(Level::INFO, "execvpe({}, {}, \"{}\")", UUID.as_str(), CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecvpe(file, argv, envp);
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        // debug(format_args!("execvpe(): {:?}\n", env));
        // event!(Level::INFO,"execvpe: {}: Updated Env {:?}", TRACKER.uuid, env);

        execvpe_common(file, argv, envcstr, true)
    }
}


/* int execve(const char *pathname, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execve(pathname: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_execve,-1,true) {
        setdebugmode!("execve");
        // debug(format_args!("execve({}, {})\n", CStr::from_ptr(pathname).to_string_lossy(), utils::cpptr2str(argv, " ")));
        event!(Level::INFO, "execve({}, {}, \"{}\")", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(),utils::cpptr2str(argv, " "));
        TRACKER.reportexecvpe(pathname, argv, envp);
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        // debug(format_args!("execve(): {:?}\n", env));
        event!(Level::INFO,"execve: {}: Updated Env {:?}", &UUID.as_str(), env);

        execvpe_common(pathname, argv, envcstr, false)
    }
}


 /* int execveat(int dirfd, const char *pathname, char *const argv[], char *const envp[], int flags); */

// hook! {
//     unsafe fn execveat(dirfd: libc::c_int, pathname: *const libc::c_char,
//                        argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_execveat,-1,true) {
//         setdebugmode!("execveat");
//         // event!(Level::INFO, "execveat({}, {}, \"{}\")", TRACKER.uuid, CStr::from_ptr(pathname).to_string_lossy(),utils::cpptr2str(argv, " "));
//         // TRACKER.reportexecvpe(pathname, argv, envp);
//         // TRACKER.reportexecveat(dirfd, pathname, argv, envp);
//         let mut env = utils::cpptr2hashmap(envp);
//         utils::envupdate(&mut env,&TRACKER.wiskfields);
//         utils::hashmapassert(&env, vec!("LD_PRELOAD"));
//         let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
//         // event!(Level::INFO,"execveat: {}: Updated Env {:?}", TRACKER.uuid, env);
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
        // debug(format_args!("posix_spawnp({}, {})\n", CStr::from_ptr(path).to_string_lossy(), utils::cpptr2str(argv, " ")));
        event!(Level::INFO, "posix_spawnp({}, {}, \"{}\")", &UUID.as_str(), CStr::from_ptr(path).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecvpe(path, argv, envp);
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        event!(Level::INFO,"posix_spawn: {}: Updated Env {:?}", &UUID.as_str(), env);
        let mut envp = utils::vcstr2vecptr(&envcstr);
        envp.push(std::ptr::null());
        real!(posix_spawn)(pid, path, file_actions, attrp, argv, envp.as_ptr())
    }
}

/* int posix_spawnp(pid_t *pid, const char *file, const posix_spawn_file_actions_t *file_actions,
                    const posix_spawnattr_t *attrp, char *const argv[], char * const envp[]); */

hook! {
    unsafe fn posix_spawnp(pid: *mut libc::pid_t, file: *const libc::c_char, file_actions: *const libc::posix_spawn_file_actions_t,
                           attrp: *const libc::posix_spawnattr_t, argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_posix_spawnp,-1,true) {
        setdebugmode!("posix_spawnp");
        // debug(format_args!("posix_spawnp({}, {})\n", CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " ")));
        event!(Level::INFO, "posix_spawnp({}, {}, \"{}\")", &UUID.as_str(), CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecvpe(file, argv, envp);

        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env, vec!("LD_PRELOAD"));
        event!(Level::INFO,"posix_spawnp: {}: Updated Env {:?}", &UUID.as_str(), env);
        let mut envp = utils::vcstr2vecptr(&envcstr);
        envp.push(std::ptr::null());
        real!(posix_spawnp)(pid, file, file_actions, attrp, argv, envp.as_ptr())
    }
}


/* FILE popen(const char *command, const char *type); */

hook! {
    unsafe fn popen(command: *const libc::c_char, ctype: *const libc::c_char) -> *const libc::FILE => (my_popen,-1,true) {
        setdebugmode!("popen");
        // debug(format_args!("popen({}, {})\n", std::process::id(), CStr::from_ptr(command).to_string_lossy()));
        event!(Level::INFO, "popen({}, {})", &UUID.as_str(), CStr::from_ptr(command).to_string_lossy());
        TRACKER.reportpopen(command, ctype);
        utils::currentenvupdate(&WISKMAP);
        real!(popen)(command, ctype)
    }
}

/*  int system (const char *command) */

hook! {
    unsafe fn system(command: *const libc::c_char) -> libc::c_int => my_system {
        setdebugmode!("system");
        // debug(format_args!("system({},{})\n", std::process::id(), CStr::from_ptr(command).to_string_lossy()));
        event!(Level::INFO, "system({}, {})", &UUID.as_str(), CStr::from_ptr(command).to_string_lossy());
        TRACKER.reportsystem(command);
        utils::currentenvupdate(&WISKMAP);
        real!(system)(command)
    }
}


/* int symlink(const char *target, const char *linkpath); */
hook! {
    unsafe fn symlink(target: *const libc::c_char, linkpath: *const libc::c_char) -> libc::c_int => (my_symlink,-1,true) {
        setdebugmode!("symlink");
        event!(Level::INFO, "symlink({}, {}, {})", &UUID.as_str(), CStr::from_ptr(target).to_string_lossy(), CStr::from_ptr(linkpath).to_string_lossy());
        // TRACKER.reportsymlink(target, linkpath);
        real!(symlink)(target, linkpath)
    }
}

/* int symlinkat(const char *target, int newdirfd, const char *linkpath); */
hook! {
    unsafe fn symlinkat(target: *const libc::c_char, newdirfd: libc::c_int, linkpath: *const libc::c_char) -> libc::c_int => (my_symlinkat,-1,true) {
        setdebugmode!("symlinkat");
        event!(Level::INFO, "symlinkat({}, {}, {})", &UUID.as_str(), CStr::from_ptr(target).to_string_lossy(),CStr::from_ptr(linkpath).to_string_lossy() );
        TRACKER.reportsymlinkat(target, newdirfd, linkpath);
        real!(symlinkat)(target, newdirfd, linkpath)
    }
}

/* int link(const char *oldpath, const char *newpath); */
hook! {
    unsafe fn link(oldpath: *const libc::c_char, newpath: *const libc::c_char) -> libc::c_int => (my_link,-1,true) {
        setdebugmode!("link");
        event!(Level::INFO, "link({}, {}, {})", &UUID.as_str(), CStr::from_ptr(oldpath).to_string_lossy(),CStr::from_ptr(newpath).to_string_lossy());
        // TRACKER.reportlink(oldpath, newpath);
        real!(link)(oldpath, newpath)
    }
}

/* int linkat(int olddirfd, const char *oldpath, int newdirfd, const char *newpath, int flags); */
hook! {
    unsafe fn linkat(olddirfd: libc::c_int, oldpath: *const libc::c_char,
                     newdirfd: libc::c_int, newpath: *const libc::c_char, flags: libc::c_int) -> libc::c_int => (my_linkat,-1,true) {
        setdebugmode!("linkat");
        event!(Level::INFO, "linkat({}, {}, {})", &UUID.as_str(), CStr::from_ptr(oldpath).to_string_lossy(),CStr::from_ptr(newpath).to_string_lossy());
        // TRACKER.reportlinkat(olddirfd, oldpath, newdirfd, newpath, flags);
        real!(linkat)(olddirfd, oldpath, newdirfd, newpath, flags)
    }
}

/* int unlink(const char *pathname); */
hook! {
    unsafe fn unlink(pathname: *const libc::c_char) -> libc::c_int => (my_unlink,-1,true) {
        setdebugmode!("unlink");
        event!(Level::INFO, "unlink({}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy());
        // TRACKER.reportunlink(pathname);
        real!(unlink)(pathname)
    }
}

/* int unlinkat(int dirfd, const char *pathname, int flags); */
hook! {
    unsafe fn unlinkat(dirfd: libc::c_int, pathname: *const libc::c_char, flags: libc::c_int) -> libc::c_int => (my_unlinkat,-1,true) {
        setdebugmode!("unlinkat");
        event!(Level::INFO, "unlinkat({}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy());
        // TRACKER.reportunlinkat(dirfd, pathname, flags);
        real!(unlinkat)(dirfd, pathname, flags)
    }
}

/* int chmod(__const char *__file, __mode_t __mode); */
hook! {
    unsafe fn chmod(pathname: *const libc::c_char, mode: libc::mode_t) -> libc::c_int => (my_chmod,-1,true) {
        setdebugmode!("chmod");
        event!(Level::INFO, "chmod({}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy());
        // TRACKER.reportchmod(pathname, mode);
        // debug(format_args!("chmod({})", CStr::from_ptr(pathname).to_string_lossy()));
        real!(chmod)(pathname, mode)
    }
}

/* int fchmod(int __fd, __mode_t __mode); */
hook! {
    unsafe fn fchmod(fd: libc::c_int, mode: libc::mode_t) -> libc::c_int => (my_fchmod,-1,true) {
        setdebugmode!("fchmod");
        event!(Level::INFO, "fchmod({})", &UUID.as_str());
        // TRACKER.reportfchmod(fd, mode);
        real!(fchmod)(fd, mode)
    }
}

/* int fchmodat(int __fd, __const char *__file, __mode_t __mode, int flags); */
hook! {
    unsafe fn fchmodat(dirfd: libc::c_int, pathname: *const libc::c_char, mode: libc::mode_t, flags: libc::c_int) -> libc::c_int => (my_fchmodat,-1,true) {
        setdebugmode!("fchmodat");
        event!(Level::INFO, "fchmodat({}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy());
        // TRACKER.reportfchmodat(dirfd, pathname, mode, flags);
        real!(fchmodat)(dirfd, pathname, mode, flags)
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
    // real!(readlink);
    // real!(creat);
    // real!(open);
    // real!(openat);
    // real!(fopen);
    // #[cfg(target_arch = "x86_64")]
    // {
    //     real!(open64);
    //     real!(fopen64);
    // }
    real!(execv);
    real!(execvp);
    real!(execvpe);
    real!(execve);
    // real!(execveat);
    real!(posix_spawn);
    real!(posix_spawnp);
    real!(popen);
    // real!(link);
    // real!(linkat);
    // real!(symlink);
    // real!(symlinkat);
    // real!(unlink);
    // real!(unlinkat);
    // real!(chmod);
    // real!(fchmod);
    // real!(fchmodat);
}

#[cfg(not(test))]
#[ctor]
fn cfoo() {
    // debug(format_args!("Constructor: {}\n", std::process::id()));
    dlsym_intialize();
    // assert_ne!(&TRACKER.pid, "");
    // MY_DISPATCH;
    // MY_DISPATCH.with(|(tracing, my_dispatch, _guard)| { });
    tracker::initialize_statics();
    TRACKER.initialize();
    redhook::initialize();
    // debug(format_args!("Constructor Done: {}, {}\n", std::process::id(), serde_json::to_string(&CMDLINE.to_vec()).unwrap()));
}

#[dtor]
fn dfoo() {
    // debug(format_args!("Destructor Done: {}, {}\n", std::process::id(), serde_json::to_string(&CMDLINE.to_vec()).unwrap()));
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
