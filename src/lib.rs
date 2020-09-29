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
extern crate lazy_static;
extern crate tracing_appender;
extern crate tracing_subscriber;
extern crate backtrace;

mod tracker;
mod utils;

use std::{env, ptr};
// use tracker::{MY_DISPATCH_initialized, MY_DISPATCH, TRACKER, DEBUGMODE};
use tracker::{WISKMAP, TRACKER, DEBUGMODE, CMDLINE, UUID};
use std::ffi::CStr;
use core::cell::Cell;
use ctor::{ctor, dtor};
use paste::paste;
use tracing::{Level, event, };
use libc::{c_char,c_int,O_CREAT,O_TMPFILE,SYS_readlink, SYS_open};
use tracing::dispatcher::with_default;
use utils::WISKFD;
use redhook::{debug, initialized};
use backtrace::Backtrace;
use std::io::{Error, Read, Result, Write};


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

// hook! {
//     unsafe fn fopen(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => (my_fopen,-1,true) {
//         setdebugmode!("fopen");
//         event!(Level::INFO, "fopen({}, {})", &UUID.as_str(), CStr::from_ptr(name).to_string_lossy());
//         TRACKER.reportfopen(name, mode);
//         real!(fopen)(name, mode)
//     }
// }

// /* #ifdef HAVE_FOPEN64 */
// #[cfg(target_arch = "x86_64")]
// hook! {
//     unsafe fn fopen64(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => (my_fopen64,-1,true) {
//         setdebugmode!("fopen64");
//         event!(Level::INFO, "fopen64({}, {})", &UUID.as_str(), CStr::from_ptr(name).to_string_lossy());
//         TRACKER.reportfopen(name, mode);
//         real!(fopen64)(name, mode)
//     }
// }
/* #endif */

// /* typedef int (*__libc_open)(const char *pathname, int flags, ...); */
// dhook! {
//     unsafe fn open(args: std::ffi::VaListImpl, pathname: *const c_char, flags: c_int ) -> c_int => (my_open,true) {
//         setdebugmode!("open");
//         if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
//             let mut ap: std::ffi::VaListImpl = args.clone();
//             let mode: c_int = ap.arg::<c_int>();
//             event!(Level::INFO, "open({}, {}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags, mode);
//             TRACKER.reportopen(pathname,flags,mode);
//             real!(open)(pathname, flags, mode)
//         } else {
//             event!(Level::INFO, "open({}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags);
//             TRACKER.reportopen(pathname,flags,0);
//             real!(open)(pathname, flags)
//         }
//     }
// }

// // /*
// // #ifdef HAVE_OPEN64
// //    typedef int (*__libc_open64)(const char *pathname, int flags, ...);
// // #endif */
// #[cfg(target_arch = "x86_64")]
// dhook! {
//     unsafe fn open64(args: std::ffi::VaListImpl, pathname: *const c_char, flags: c_int ) -> c_int => (my_open64, true) {
//         if !initialized() {
//             if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
//                 let mut ap: std::ffi::VaListImpl = args.clone();
//                 let mode: c_int = ap.arg::<c_int>();
//                 // debug(format_args!("open64({}, {:X}, {:X})\n", CStr::from_ptr(pathname).to_string_lossy(), flags, mode));
//                 event!(Level::INFO, "open64({}, {}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags, mode);
//                 real!(open64)(pathname, flags, mode)
//             } else {
//                 // debug(format_args!("open64({}, {:X})\n", CStr::from_ptr(pathname).to_string_lossy(), flags));
//                 event!(Level::INFO, "open64({}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags);
//                 real!(open64)(pathname, flags)
//             }
//         } else {
//             setdebugmode!("open64");
//             if ((flags & O_CREAT) == O_CREAT) || ((flags & O_TMPFILE) == O_TMPFILE) {
//                 let mut ap: std::ffi::VaListImpl = args.clone();
//                 let mode: c_int = ap.arg::<c_int>();
//                 // debug(format_args!("open64({}, {:X}, {:X})\n", CStr::from_ptr(pathname).to_string_lossy(), flags, mode));
//                 event!(Level::INFO, "open64({}, {}, {}, {})", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(), flags, mode);
//                 TRACKER.reportopen(pathname,flags,mode);
//                 real!(open64)(pathname, flags, mode)
//             } else {
//                 // event!(Level::INFO, "open64({}, {}, {})", TRACKER.uuid, CStr::from_ptr(pathname).to_string_lossy(), flags);
//                 TRACKER.reportopen(pathname,flags,0);
//                 debug(format_args!("open64({}, {}, {:X})\n", &UUID.as_str(),CStr::from_ptr(pathname).to_string_lossy(), flags));
//                 real!(open64)(pathname, flags)
//             }
//         }
//     }
// }


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
//typedef int (*__libc_execl)(const char *path, char *const arg, ...);
//typedef int (*__libc_execlp)(const char *file, char *const arg, ...);
//typedef int (*__libc_execlpe)(const char *path, const char *arg,..., char * const envp[]);

 */

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
        let envcstr = utils::hashmap2vcstr(&env);
        // debug(format_args!("execv(): {:?}\n", env));
        event!(Level::INFO,"execv: {}: Updated Env {:?}", &UUID.as_str(), env);
        let mut envp = utils::vcstr2vecptr(&envcstr);
        envp.push(std::ptr::null());
        real!(execve)(path, argv, envp.as_ptr())
        // real!(execv)(path, argv)
    }
}

 /* int execvp(const char *file, char *const argv[]); */

 fn testfunc() {
    debug(format_args!("Tracker: pid: {}, {:?}\n", std::process::id(), &TRACKER.file));
    // (&TRACKER.file).write_all(b"Hello, world!").unwrap();
    // (&TRACKER.file.write_all(format!("Hello World\n").as_bytes()).unwrap();
 }

hook! {
    unsafe fn execvp(file: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => (my_execvp,-1,true) {
        setdebugmode!("execvp");
        // debug(format_args!("execvp({}, {}, {})\n", std::process::id(), CStr::from_ptr(file).to_string_lossy(),utils::cpptr2str(argv, " ")));
        // testfunc();
        event!(Level::INFO, "execvp({}, {}, \"{}\"F)", &UUID.as_str(), CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecvp(file, argv);
        let mut env = utils::envgetcurrent();
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env);
        // debug(format_args!("execvpe: Updated Env {:?}", env));
        event!(Level::INFO,"execvpe: {}: Updated Env {:?}", &UUID.as_str(), env);
        let mut envp = utils::vcstr2vecptr(&envcstr);
        envp.push(std::ptr::null());
        real!(execvpe)(file, argv, envp.as_ptr())
        // real!(execvp)(file, argv)
    }
}

/* int execvpe(const char *file, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execvpe(file: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_execvpe,-1,true) {
        setdebugmode!("execvpe");
        // testfunc();
        // debug(format_args!("execvpe({}, {})\n", CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " ")));
        event!(Level::INFO, "execvpe({}, {}, \"{}\")", UUID.as_str(), CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " "));
        TRACKER.reportexecvpe(file, argv, envp);
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env);
        // debug(format_args!("execvpe(): {:?}\n", env));
        // event!(Level::INFO,"execvpe: {}: Updated Env {:?}", TRACKER.uuid, env);
        let mut envp = utils::vcstr2vecptr(&envcstr);
        envp.push(std::ptr::null());
        real!(execvpe)(file, argv, envp.as_ptr())
        // real!(execvpe)(file, argv, envp)
    }
}


/* int execve(const char *pathname, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execve(pathname: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => (my_execve,-1,true) {
        setdebugmode!("execve");
        // testfunc();
        // debug(format_args!("execve({}, {})\n", CStr::from_ptr(pathname).to_string_lossy(), utils::cpptr2str(argv, " ")));
        event!(Level::INFO, "execve({}, {}, \"{}\")", &UUID.as_str(), CStr::from_ptr(pathname).to_string_lossy(),utils::cpptr2str(argv, " "));
        TRACKER.reportexecvpe(pathname, argv, envp);
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env);
        // debug(format_args!("execve(): {:?}\n", env));
        event!(Level::INFO,"execve: {}: Updated Env {:?}", &UUID.as_str(), env);
        let mut envp = utils::vcstr2vecptr(&envcstr);
        envp.push(std::ptr::null());
        real!(execve)(pathname, argv, envp.as_ptr())
        // real!(execve)(pathname, argv, envp)
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
//         let envcstr = utils::hashmap2vcstr(&env);
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
        event!(Level::INFO, "posix_spawn({}, {}, \"{}\")", &UUID.as_str(), CStr::from_ptr(path).to_string_lossy(), utils::cpptr2str(argv, " "));
        // TRACKER.reportunlinkat(dirfd, pathname, flags);
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env);
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
        event!(Level::INFO, "posix_spawnp({}, {}, \"{}\")", &UUID.as_str(), CStr::from_ptr(file).to_string_lossy(), utils::cpptr2str(argv, " "));
        // TRACKER.reportposix_spawnp(pid, file, file_actions, attrp, argv, envp);
        let mut env = utils::cpptr2hashmap(envp);
        utils::envupdate(&mut env,&WISKMAP);
        utils::hashmapassert(&env, vec!("LD_PRELOAD"));
        let envcstr = utils::hashmap2vcstr(&env);
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
        event!(Level::INFO, "popen({}, {})", &UUID.as_str(), CStr::from_ptr(command).to_string_lossy());
        // TRACKER.reportpopen(command, ctype);
        real!(popen)(command, ctype)
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
