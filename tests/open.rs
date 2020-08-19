
extern crate wisktrack;
extern crate test_env_log;
use test_env_log::test;
use libc::{c_char, mode_t};


use std::ffi::CString;

extern {
    fn chmod(file: *const c_char, mode: mode_t);
    fn printf(format: *const c_char , ...);
}



#[test]
fn test_chmod_1() {
    // Statements here are executed when the compiled binary is called

    // Print text to the console
    let file = CString::new("/tmp/file1").expect("CString::new failed");
    unsafe {
        chmod(file.as_ptr(), 0);
        printf(file.as_ptr());
    }
}

#[test]
fn test_printf_1() {
    // Statements here are executed when the compiled binary is called

    // Print text to the console
    let msg = CString::new("Hello World in C").expect("CString::new failed");
    unsafe {
        printf(msg.as_ptr());
    }
}
