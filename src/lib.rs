#[macro_use]
extern crate redhook;
extern crate libc;

hook! {
    unsafe fn readlink(path: *const libc::c_char, buf: *mut libc::c_char, bufsiz: libc::size_t) -> libc::ssize_t => my_readlink {
        if let Ok(path) = std::str::from_utf8(std::ffi::CStr::from_ptr(path).to_bytes()) {
            if path == "test-panic" {
                panic!("Testing panics");
            }
            println!("readlink(\"{}\")", path);
        } else {
            println!("readlink(...)");
        }

        real!(readlink)(path, buf, bufsiz)
    }
}

hook! {
    unsafe fn fopen(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => my_fopen {
        real!(fopen)(name, mode)
    }
}

/* #ifdef HAVE_FOPEN64 */

hook! {
    unsafe fn fopen64(name: *const libc::c_char, mode: *const libc::c_char) -> *const libc::FILE => my_fopen64 {
        real!(fopen64)(name, mode)
    }
}
/* #endif */



/* 
typedef int (*__libc_fcntl)(int fd, int cmd, ...);
typedef int (*__libc_open)(const char *pathname, int flags, ...);
#ifdef HAVE_OPEN64
typedef int (*__libc_open64)(const char *pathname, int flags, ...);
#endif /* HAVE_OPEN64 */
typedef int (*__libc_openat)(int dirfd, const char *path, int flags, ...);
//typedef int (*__libc_execl)(const char *path, char *const arg, ...);
//typedef int (*__libc_execlp)(const char *file, char *const arg, ...);
//typedef int (*__libc_execlpe)(const char *path, const char *arg,..., char * const envp[]);

 */

 /* int execv(const char *path, char *const argv[]); */

 hook! {
    unsafe fn execv(path: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => my_execv {
        real!(execv)(path, argv)
    }
}

 /* int execvp(const char *file, char *const argv[]); */

hook! {
    unsafe fn execvp(file: *const libc::c_char, argv: *const *const libc::c_char) -> libc::c_int => my_execvp {
        real!(execvp)(file, argv)
    }
}

/* int execvpe(const char *file, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execvpe(file: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_execvpe {
        real!(execvpe)(file, argv, envp)
    }
}


/* int execve(const char *pathname, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn execve(pathname: *const libc::c_char,
                     argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_execve {
        real!(execve)(pathname, argv, envp)
    }
}


 /* int execveat(int dirfd, const char *pathname, char *const argv[], char *const envp[], int flags); */

hook! {
    unsafe fn execveat(dirfd: libc::c_int, pathname: *const libc::c_char,
                       argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_execveat {
        real!(execveat)(dirfd, pathname, argv, envp)
    }
}


 /* int posix_spawn(pid_t *pid, const char *path, const posix_spawn_file_actions_t *file_actions,
                    const posix_spawnattr_t *attrp, char *const argv[], char *const envp[]); */

hook! {
    unsafe fn posix_spawn(pid: *mut libc::pid_t, path: *const libc::c_char, file_actions: *const libc::posix_spawn_file_actions_t,
                           attrp: *const libc::posix_spawnattr_t, argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_posix_spawn {
        real!(posix_spawn)(pid, path, file_actions, attrp, argv, envp)
    }
}

/* int posix_spawnp(pid_t *pid, const char *file, const posix_spawn_file_actions_t *file_actions,
                    const posix_spawnattr_t *attrp, char *const argv[], char * const envp[]); */

hook! {
    unsafe fn posix_spawnp(pid: *mut libc::pid_t, file: *const libc::c_char, file_actions: *const libc::posix_spawn_file_actions_t,
                           attrp: *const libc::posix_spawnattr_t, argv: *const *const libc::c_char, envp: *const *const libc::c_char) -> libc::c_int => my_posix_spawnp {
        real!(posix_spawnp)(pid, file, file_actions, attrp, argv, envp)
    }
}


/* FILE popen(const char *command, const char *type); */

hook! {
    unsafe fn popen(command: *const libc::c_char, ctype: *const libc::c_char) -> *const libc::FILE => my_popen {
        real!(popen)(command, ctype)
    }
}

/* int symlink(const char *target, const char *linkpath); */
hook! {
    unsafe fn symlink(target: *const libc::c_char, linkpath: *const libc::c_char) -> libc::c_int => my_symlink {
        if let Ok(linkpath) = std::str::from_utf8(std::ffi::CStr::from_ptr(linkpath).to_bytes()) {
            println!("symlink(\"{}\")", linkpath);
        } else {
            println!("symlink(...)");
        }
        real!(symlink)(target, linkpath)
    }
}

/* int symlinkat(const char *target, int newdirfd, const char *linkpath); */
hook! {
    unsafe fn symlinkat(target: *const libc::c_char, newdirfd: libc::c_int, linkpath: *const libc::c_char) -> libc::c_int => my_symlinkat {
        if let Ok(linkpath) = std::str::from_utf8(std::ffi::CStr::from_ptr(linkpath).to_bytes()) {
            println!("symlinkat(\"{}\")", linkpath);
        } else {
            println!("symlinkat(...)");
        }
        real!(symlinkat)(target, newdirfd, linkpath)
    }
}

/* int link(const char *oldpath, const char *newpath); */
hook! {
    unsafe fn link(oldpath: *const libc::c_char, newpath: *const libc::c_char) -> libc::c_int => my_link {
        if let Ok(newpath) = std::str::from_utf8(std::ffi::CStr::from_ptr(newpath).to_bytes()) {
            println!("link(\"{}\")", newpath);
        } else {
            println!("link(...)");
        }
        real!(link)(oldpath, newpath)
    }
}

/* int linkat(int olddirfd, const char *oldpath, int newdirfd, const char *newpath, int flags); */
hook! {
    unsafe fn linkat(olddirfd: libc::c_int, oldpath: *const libc::c_char,
                     newdirfd: libc::c_int, newpath: *const libc::c_char, flags: libc::c_int) -> libc::c_int => my_linkat {
        if let Ok(newpath) = std::str::from_utf8(std::ffi::CStr::from_ptr(newpath).to_bytes()) {
            println!("linkat(\"{}\")", newpath);
        } else {
            println!("linkat(...)");
        }
        real!(linkat)(olddirfd, oldpath, newdirfd, newpath, flags)
    }
}

/* int unlink(const char *pathname); */
hook! {
    unsafe fn unlink(pathname: *const libc::c_char) -> libc::c_int => my_unlink {
        if let Ok(pathname) = std::str::from_utf8(std::ffi::CStr::from_ptr(pathname).to_bytes()) {
            println!("unlink(\"{}\")", pathname);
        } else {
            println!("unlink(...)");
        }
        real!(unlink)(pathname)
    }
}

/* int unlinkat(int dirfd, const char *pathname, int flags); */
hook! {
    unsafe fn unlinkat(dirfd: libc::c_int, pathname: *const libc::c_char, flags: libc::c_int) -> libc::c_int => my_unlinkat {
        if let Ok(pathname) = std::str::from_utf8(std::ffi::CStr::from_ptr(pathname).to_bytes()) {
            println!("unlinkat(\"{}\")", pathname);
        } else {
            println!("unlinkat(...)");
        }
        real!(unlinkat)(dirfd, pathname, flags)
    }
}

/* int chmod(__const char *__file, __mode_t __mode); */
hook! {
    unsafe fn chmod(file: *const libc::c_char, mode: libc::mode_t) -> libc::c_int => my_chmod {
        if let Ok(file) = std::str::from_utf8(std::ffi::CStr::from_ptr(file).to_bytes()) {
            println!("chmod(\"{}\")", file);
        } else {
            println!("chmod(...)");
        }
        real!(chmod)(file, mode)
    }
}

/* int fchmod(int __fd, __mode_t __mode); */
hook! {
    unsafe fn fchmod(fd: libc::c_int, mode: libc::mode_t) -> libc::c_int => my_fchmod {
        println!("fchmod(...)");
        real!(fchmod)(fd, mode)
    }
}

/* int fchmodat(int __fd, __const char *__file, __mode_t __mode, int flags); */
hook! {
    unsafe fn fchmodat(fd: libc::c_int, file: *const libc::c_char, mode: libc::mode_t, flags: libc::c_int) -> libc::c_int => my_fchmodat {
        if let Ok(file) = std::str::from_utf8(std::ffi::CStr::from_ptr(file).to_bytes()) {
            println!("fchmodat(\"{}\")", file);
        } else {
            println!("fchmodat(...)");
        }
        real!(fchmodat)(fd, file, mode, flags)
    }
}
