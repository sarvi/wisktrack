use std::path::{Path, PathBuf};
use std::process;
use std::fs::create_dir_all;
use std::io::Write;
use libc::{O_CREAT,O_WRONLY,O_APPEND,O_LARGEFILE,S_IRUSR,S_IWUSR,S_IRGRP,S_IWGRP};
use backtrace::Backtrace;
use crate::fs::{File};
use crate::common::{WISKTRACE, WISKFDS, WISKTRACEFD, PUUID, UUID};


pub struct Tracer {
    pub file: File,
    pub fd: i32,
}

lazy_static! {
    pub static ref TRACER:Tracer = Tracer::new();
}

impl Tracer {
    pub fn new() -> Tracer {
        cevent!(Level::INFO, "Tracer Initializer\n");
        let p = Path::new(WISKTRACE.as_str());
        if !p.parent().unwrap().exists() {
            cevent!(Level::INFO, "parent: {:?}", p.parent().unwrap());
            create_dir_all(p.parent().unwrap()).unwrap();
        }
        if let Ok(f) = File::open(WISKTRACE.as_str(), (O_CREAT|O_WRONLY|O_APPEND|O_LARGEFILE) as i32,
                                     (S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP) as i32, WISKTRACEFD) {
            let fd = f.as_raw_fd();
            let tracer = Tracer {
                file :  f,
                fd : fd,
            };
            WISKFDS.write().unwrap().push(tracer.fd);
            // cevent!(Level::INFO, "Tracer Initializer: Done\n");
            // tracer.write_all(format!("{}: Tracer Initializer: Done \n", UUID.as_str()).as_bytes());
            tracer
        } else {
            errorexit!("Error opening track file: {}\n", WISKTRACE.as_str());
        }
    }

    pub fn write_all(self: &Self, value: &[u8]) {
        (&self.file).write_all(value).unwrap();
    }
}
