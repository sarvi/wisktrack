#![feature(c_variadic)]
#[macro_use]
extern crate redhook;
extern crate core;
extern crate libc;
extern crate tracing;
extern crate ctor;
#[macro_use]
extern crate lazy_static;
extern crate tracing_appender;
extern crate tracing_subscriber;

mod tracker;

use tracker::TRACKER;
use std::ffi::CStr;
use core::cell::Cell;
use ctor::{ctor, dtor};
// use tracing::instrument;
use tracing::{Level, event, };
use libc::{c_char,c_int,O_CREAT};
use tracing::dispatcher::{with_default, Dispatch};
use tracing_appender::non_blocking::WorkerGuard;
use redhook::ld_preload::make_dispatch;


thread_local! {
    #[allow(nonstandard_style)]
    static MY_DISPATCH_initialized: ::core::cell::Cell<bool> = false.into();
}
thread_local! {
    static MY_DISPATCH: (bool, Dispatch, WorkerGuard) = {
        let ret = make_dispatch("WISK_TRACE");
        MY_DISPATCH_initialized.with(|it| it.set(true));
        ret
    };
}


// hook! {
//     unsafe fn readlink(path: *const libc::c_char, buf: *mut libc::c_char, bufsiz: libc::size_t) -> libc::ssize_t => my_readlink {
//         TRACKER.reportreadlink(path, buf, bufsize);
//         event!(Level::INFO, "readlink({}", CStr::from_ptr(path).to_string_lossy());
//         real!(readlink)(path, buf, bufsiz)
//     }
// }

/* int creat(const char *pathname, mode_t mode); */
hook! {
    unsafe fn creat(pathname: *const libc::c_char, mode: libc::mode_t) -> c_int => my_creat {
        TRACKER.reportcreat(pathname, mode);
        event!(Level::INFO, "creat({})", CStr::from_ptr(pathname).to_string_lossy());
        real!(creat)(pathname, mode)
    }
}

hook! {
    unsafe fn fopen(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => my_fopen {
        TRACKER.reportfopen(name, mode);
        event!(Level::INFO, "fopen({})", CStr::from_ptr(name).to_string_lossy());
        real!(fopen)(name, mode)
    }
}

/* #ifdef HAVE_FOPEN64 */
#[cfg(target_arch = "x86")]
hook! {
    unsafe fn fopen64(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => my_fopen64 {
        TRACKER.reportfopen(name, mode);
        event!(Level::INFO, "fopen64({})", CStr::from_ptr(name).to_string_lossy());
        real!(fopen64)(name, mode)
    }
}
/* #endif */

/* typedef int (*__libc_open)(const char *pathname, int flags, ...); */
dhook! {
    unsafe fn open(args: std::ffi::VaListImpl, pathname: *const c_char, flags: c_int ) -> c_int => (my_open, orig_open) {
        event!(Level::INFO, "open({}, {}, {})", *&TRACKER.uuid,
               CStr::from_ptr(pathname).to_string_lossy(), flags);
        if (flags & O_CREAT) == O_CREAT {
            let mut ap: std::ffi::VaListImpl = args.clone();
            let mode: c_int = ap.arg::<c_int>();
            TRACKER.reportopen(pathname,flags,mode);
            real!(orig_open)(pathname, flags, mode)
        } else {
            TRACKER.reportopen(pathname,flags,0);
            real!(orig_open)(pathname, flags)
        }
    }
}

/*
#ifdef HAVE_OPEN64
   typedef int (*__libc_open64)(const char *pathname, int flags, ...);
#endif */
#[cfg(target_arch = "x86")]
dhook! {
    unsafe fn open64(args: std::ffi::VaListImpl, pathname: *const c_char, flags: c_int ) -> c_int => (my_open64, orig_open64) {
        event!(Level::INFO, "open({}, {}, {})", *&TRACKER.uuid,
               CStr::from_ptr(pathname).to_string_lossy(), flags);
        if (flags & O_CREAT) == O_CREAT {
            let mut ap: std::ffi::VaListImpl = args.clone();
            let mode: c_int = ap.arg::<c_int>();
            TRACKER.reportopen(pathname,flags,mode);
            real!(orig_open64)(pathname, flags, mode)
        } else {
            TRACKER.reportopen(pathname,flags,0);
            real!(orig_open64)(pathname, flags)
        }
    }
}


/* typedef int (*__libc_openat)(int dirfd, const char *path, int flags, ...); */
vhook! {
    unsafe fn vopenat(args: std::ffi::VaList, dirfd: c_int, path: *const c_char, flags: c_int ) -> c_int => my_vopenat {
        event!(Level::INFO, "vopenat({}, {}, {})", dirfd, CStr::from_ptr(path).to_string_lossy(), flags);
        real!(vopenat)(dirfd, path, flags, args)
    }
}


dhook! {
    unsafe fn openat(args: std::ffi::VaListImpl, dirfd: c_int, path: *const c_char, flags: c_int ) -> c_int => my_openat {
        event!(Level::INFO, "openat({}, {}, {}, {})", *&TRACKER.uuid,
               dirfd, CStr::from_ptr(path).to_string_lossy(), flags);
        let mut aq: std::ffi::VaListImpl;
        aq  =  args.clone();
        my_vopenat(dirfd, path, flags, aq.as_va_list())
    }
}


/* 
typedef int (*__libc_fcntl)(int fd, int cmd, ...);
//typedef int (*__libc_execl)(const char *path, char *const arg, ...);
//typedef int (*__libc_execlp)(const char *file, char *const arg, ...);
//typedef int (*__libc_execlpe)(const char *path, const char *arg,..., char * const envp[]);

 */

 /* int execv(const char *path, char *const argv[]); */

 hook! {
    unsafe fn execv(path: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => my_execv {
        TRACKER.reportexecv(path, argv);
        event!(Level::INFO, "execv({})", CStr::from_ptr(path).to_string_lossy());
        real!(execv)(path, argv)
    }
}

 /* int execvp(const char *file, char *const argv[]); */

hook! {
    unsafe fn execvp(file: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => my_execvp {
        TRACKER.reportexecv(file, argv);
        event!(Level::INFO, "execvp({})", CStr::from_ptr(file).to_string_lossy());
        real!(execvp)(file, argv)
    }
}

/* int execvpe(const char *file, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execvpe(file: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_execvpe {
        TRACKER.reportexecvpe(file, argv, envp);
        event!(Level::INFO, "execvpe({})", CStr::from_ptr(file).to_string_lossy());
        real!(execvpe)(file, argv, envp)
    }
}


/* int execve(const char *pathname, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execve(pathname: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_execve {
        // TRACKER.reportexecve(pathname, argv, envp);
        event!(Level::INFO, "open({})", CStr::from_ptr(pathname).to_string_lossy());
        real!(execve)(pathname, argv, envp)
    }
}


 /* int execveat(int dirfd, const char *pathname, char *const argv[], char *const envp[], int flags); */

hook! {
    unsafe fn execveat(dirfd: libc::c_int, pathname: *const libc::c_char,
                       argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_execveat {
        // TRACKER.reportexecveat(dirfd, pathname, argv, envp);
        event!(Level::INFO, "open({})", CStr::from_ptr(pathname).to_string_lossy());
        real!(execveat)(dirfd, pathname, argv, envp)
    }
}


 /* int posix_spawn(pid_t *pid, const char *path, const posix_spawn_file_actions_t *file_actions,
                    const posix_spawnattr_t *attrp, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn posix_spawn(pid: *mut libc::pid_t, path: *const libc::c_char, file_actions: *const libc::posix_spawn_file_actions_t,
                           attrp: *const libc::posix_spawnattr_t, argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_posix_spawn {
        // TRACKER.reportunlinkat(dirfd, pathname, flags);
        event!(Level::INFO, "open({})", CStr::from_ptr(path).to_string_lossy());
        real!(posix_spawn)(pid, path, file_actions, attrp, argv, envp)
    }
}

/* int posix_spawnp(pid_t *pid, const char *file, const posix_spawn_file_actions_t *file_actions,
                    const posix_spawnattr_t *attrp, char *const argv[], char * const envp[]); */

hook! {
    unsafe fn posix_spawnp(pid: *mut libc::pid_t, file: *const libc::c_char, file_actions: *const libc::posix_spawn_file_actions_t,
                           attrp: *const libc::posix_spawnattr_t, argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_posix_spawnp {
        // TRACKER.reportposix_spawnp(pid, file, file_actios, attrp, argv, envp);
        event!(Level::INFO, "open({})", CStr::from_ptr(file).to_string_lossy());
        real!(posix_spawnp)(pid, file, file_actions, attrp, argv, envp)
    }
}


/* FILE popen(const char *command, const char *type); */

hook! {
    unsafe fn popen(command: *const libc::c_char, ctype: *const libc::c_char) -> *const libc::FILE => my_popen {
        TRACKER.reportpopen(command, ctype);
        event!(Level::INFO, "popen({})", CStr::from_ptr(command).to_string_lossy());
        real!(popen)(command, ctype)
    }
}

/* int symlink(const char *target, const char *linkpath); */
hook! {
    unsafe fn symlink(target: *const libc::c_char, linkpath: *const libc::c_char) -> libc::c_int => my_symlink {
        TRACKER.reportsymlink(target, linkpath);
        event!(Level::INFO, "symlink({}, {})", CStr::from_ptr(target).to_string_lossy(), CStr::from_ptr(linkpath).to_string_lossy());
        real!(symlink)(target, linkpath)
    }
}

/* int symlinkat(const char *target, int newdirfd, const char *linkpath); */
hook! {
    unsafe fn symlinkat(target: *const libc::c_char, newdirfd: libc::c_int, linkpath: *const libc::c_char) -> libc::c_int => my_symlinkat {
        TRACKER.reportsymlinkat(target, newdirfd, linkpath);
        event!(Level::INFO, "symlinkat({}, {})", CStr::from_ptr(target).to_string_lossy(),CStr::from_ptr(linkpath).to_string_lossy() );
        real!(symlinkat)(target, newdirfd, linkpath)
    }
}

/* int link(const char *oldpath, const char *newpath); */
hook! {
    unsafe fn link(oldpath: *const libc::c_char, newpath: *const libc::c_char) -> libc::c_int => my_link {
        TRACKER.reportlink(oldpath, newpath);
        event!(Level::INFO, "link({}, {})", CStr::from_ptr(oldpath).to_string_lossy(),CStr::from_ptr(newpath).to_string_lossy());
        real!(link)(oldpath, newpath)
    }
}

/* int linkat(int olddirfd, const char *oldpath, int newdirfd, const char *newpath, int flags); */
hook! {
    unsafe fn linkat(olddirfd: libc::c_int, oldpath: *const libc::c_char,
                     newdirfd: libc::c_int, newpath: *const libc::c_char, flags: libc::c_int) -> libc::c_int => my_linkat {
        TRACKER.reportlinkat(olddirfd, oldpath, newdirfd, newpath, flags);
        event!(Level::INFO, "linkat({}, {})", CStr::from_ptr(oldpath).to_string_lossy(),CStr::from_ptr(newpath).to_string_lossy());
        real!(linkat)(olddirfd, oldpath, newdirfd, newpath, flags)
    }
}

/* int unlink(const char *pathname); */
hook! {
    unsafe fn unlink(pathname: *const libc::c_char) -> libc::c_int => my_unlink {
        TRACKER.reportunlink(pathname);
        event!(Level::INFO, "unlink({})", CStr::from_ptr(pathname).to_string_lossy());
        real!(unlink)(pathname)
    }
}

/* int unlinkat(int dirfd, const char *pathname, int flags); */
hook! {
    unsafe fn unlinkat(dirfd: libc::c_int, pathname: *const libc::c_char, flags: libc::c_int) -> libc::c_int => my_unlinkat {
        TRACKER.reportunlinkat(dirfd, pathname, flags);
        event!(Level::INFO, "unlinkat({})", CStr::from_ptr(pathname).to_string_lossy());
        real!(unlinkat)(dirfd, pathname, flags)
    }
}

/* int chmod(__const char *__file, __mode_t __mode); */
hook! {
    unsafe fn chmod(pathname: *const libc::c_char, mode: libc::mode_t) -> libc::c_int => my_chmod {
        TRACKER.reportchmod(pathname, mode);
        event!(Level::INFO, "chmod({})", CStr::from_ptr(pathname).to_string_lossy());
        println!("chmod({})", CStr::from_ptr(pathname).to_string_lossy());
        real!(chmod)(pathname, mode)
    }
}

/* int fchmod(int __fd, __mode_t __mode); */
hook! {
    unsafe fn fchmod(fd: libc::c_int, mode: libc::mode_t) -> libc::c_int => my_fchmod {
        TRACKER.reportfchmod(fd, mode);
        event!(Level::INFO, "fchmod()");
        real!(fchmod)(fd, mode)
    }
}

/* int fchmodat(int __fd, __const char *__file, __mode_t __mode, int flags); */
hook! {
    unsafe fn fchmodat(dirfd: libc::c_int, pathname: *const libc::c_char, mode: libc::mode_t, flags: libc::c_int) -> libc::c_int => my_fchmodat {
        TRACKER.reportfchmodat(dirfd, pathname, mode, flags);
        event!(Level::INFO, "fchmodat({})", CStr::from_ptr(pathname).to_string_lossy());
        real!(fchmodat)(dirfd, pathname, mode, flags)
    }
}


vhook! {
    unsafe fn vprintf(args: std::ffi::VaList, format: *const c_char ) -> c_int => my_vprintf {
        event!(Level::INFO, "vprintf({})", CStr::from_ptr(format).to_string_lossy());
        real!(vprintf)(format, args)
    }
}


dhook! {
    unsafe fn printf(args: std::ffi::VaListImpl, format: *const c_char ) -> c_int => my_printf {
        event!(Level::INFO, "printf({}, {})", *&TRACKER.uuid,
               CStr::from_ptr(format).to_string_lossy());
        let mut aq: std::ffi::VaListImpl;
        aq  =  args.clone();
        my_vprintf(format, aq.as_va_list())
    }
}



#[ctor]
fn cfoo() {
    if !MY_DISPATCH_initialized.with(Cell::get) {
        println!("Constructor(cfoo):\n\tUUID: {},\tFILE: {:?}", *&TRACKER.uuid, *&TRACKER.file);
    } else {
        MY_DISPATCH.with(|(tracing, my_dispatch, _guard)| {
            if *tracing {
                println!("tracing: {}", tracing);
                with_default(&my_dispatch, || {
                    event!(Level::INFO, "Constructor(cfoo):\n\tUUID: {},\tFILE: {:?}",
                           *&TRACKER.uuid, *&TRACKER.file);
                })
            }
        })
    }
}

#[dtor]
fn dfoo() {
//   println!("Hello, world! Destructor");
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
