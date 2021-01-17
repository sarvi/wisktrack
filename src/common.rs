use std::env;
use libc;
use std::process;
use core::sync::atomic::AtomicUsize;
use std::sync::{Once, RwLock};
use uuid::Uuid;

pub static WISKFDBASE:i32 = 800;
pub static WISKTRACKFD:i32 = WISKFDBASE + 0;
pub static WISKTRACEFD:i32 = WISKFDBASE + 1;
// pub static WISKFD: AtomicUsize = AtomicUsize::new((WISKTRACEFD as usize)+1);

lazy_static! {
    pub static ref WISKFDS: RwLock<Vec<libc::c_int>> = RwLock::new(vec![]);
    pub static ref WISKTRACE:String = {
        let mut fname:String = match env::var("WISK_TRACE") {
            Ok(v) =>  {
                if v.is_empty() {
                    "/dev/null".to_owned()
                } else {
                    v
                }
            },
            Err(_) => {
                // cevent!(Level::INFO, "WISK_TRACE is missing\n");
                "/dev/null".to_owned()
            },
        };
        fname
    };

    pub static ref PUUID:String = {
        // event!(Level::INFO, "PUUID Initializing");
        match env::var("WISK_PUUID") {
            Ok(uuid) => uuid,
            Err(_) => String::from("XXXXXXXXXXXXXXXXXXXXXX")
        }
    };

    pub static ref UUID : String = {
        // eprintln!("UUID Initializing");
        let x = format!("{}", base_62::encode(Uuid::new_v4().as_bytes()));
        // eprintln!("{}: UUID Initializing", x);
        x
    };

    pub static ref PID : String = process::id().to_string();
}

