
extern crate libc;
use std::ffi::CString;

extern {
    fn chmod(file: *const libc::c_char, mode: libc::mode_t);
    fn printf(format: *const libc::c_char , ...);
}


fn main() {
    // Statements here are executed when the compiled binary is called

    // Print text to the console
    let file = CString::new("/tmp/file1").expect("CString::new failed");
    unsafe {
        chmod(file.as_ptr(), 0);
        printf(file.as_ptr());
    }
    println!("Hello World!");
}
