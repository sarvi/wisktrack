use std::env;
use std::sync::Mutex;
use std::io::prelude::*;
use std::path::Path;
use std::fs::{File, OpenOptions, metadata, create_dir_all};
use std::string::String;
use std::env::var;
use std::collections::HashMap;
use uuid::Uuid;
use serde::{Serialize, Deserialize};


const SENDLIMIT: usize = 4094;
// const SENDLIMIT: usize = 100;

type Envar = (String, String);

#[derive(Serialize, Deserialize, Debug)]
pub enum Ops {
    CALLS(String),
    PID(String),
    PPID(String),
    PWD(String),
    CMDPATH(String),
    CMD(Vec<String>),
    ENV(Vec<Envar>),
    READS(Vec<String>),
    COMPLETE(Vec<String>),
    WRITES(Vec<String>),
    LINKS(Vec<String>),
    CHMODS(Vec<String>)
}

pub struct Tracker {
    pub filename: String,
    pub file: File,
    pub uuid: String,
    pub puuid: String,
    pub env : HashMap<String, String>,
    // pub mutex : Mutex,
    // mut file: &File, uuid: &str, 
}

impl Tracker {
    pub fn init() -> Tracker {
        let wsroot:String = match env::var("WISK_WSROOT") {
            Ok(wsroot) => wsroot,
            Err(_) => String::from("/tmp")
        };
        if !Path::new(&wsroot).exists() {
            create_dir_all(&wsroot).unwrap();
        }
        let puuid:String = match env::var("WISK_UUID") {
            Ok(uuid) => uuid,
            Err(_) => String::from("XXXXXXXXXXXXXXXXXXXXXXXX")
        };
        let uuid:String = format!("{}", Uuid::new_v4().to_simple().encode_lower(&mut Uuid::encode_buffer()));
        let fname = match var("WISK_TRACKFILE") {
            Ok(v) => v,
            Err(_) => String::from(format!("{}/wisktrack/track.{}", wsroot, uuid)),
        };
        if !Path::new(&fname).parent().unwrap().exists() {
            create_dir_all(Path::new(&fname).parent().unwrap()).unwrap();
        }
        let trackdir = Path::new(&fname).parent().unwrap();
        if !metadata(&trackdir).unwrap().is_dir() {
            create_dir_all(trackdir).unwrap();
        }
        let fname = if Path::new(&fname).exists() && metadata(&fname).unwrap().is_dir() {
            String::from(format!("{}/wisk_track.{}", fname, uuid))
        } else { String::from(&fname) };
 
        let mut map = HashMap::new();
        // let mutex = Mutex::new();
        for (key, val) in env::vars_os() {
            // Use pattern bindings instead of testing .is_some() followed by .unwrap()
            if let (Ok(k), Ok(v)) = (key.into_string(), val.into_string()) {
                map.insert(k, v);
            }
        }
        println!("Track Data: {}", fname);
        // let mut file = File::create(&filename).unwrap();
        // wisk_reportop(file, &PARENT_UUID, "Calls", &UUID);
        // write!(self.file, "{} Calls \"{}\"", *PARENT_UUID, *UUID).unwrap();
        let tracker = Tracker {
            filename : fname.to_string(),
            file : OpenOptions::new().create(true).append(true).open(&fname).unwrap(),
            // file : File::create(&fname).unwrap(),
            uuid  : uuid,
            puuid :  puuid,
            env : map,
            // mutex: mutex,
        };
        (&tracker.file).write_all(format!("{} CALLS {}\n", tracker.puuid, serde_json::to_string(&tracker.uuid).unwrap()).as_bytes()).unwrap();
        // write!(&tracker.file, "{} CALLS {}\n", tracker.puuid, serde_json::to_string(&tracker.uuid).unwrap()).unwrap();
        tracker
    }
    
    pub fn wisk_reportop(self: &Self, op : &str, value: &str) {
        let mut minlen: usize = self.uuid.len() + op.len() + 2;
        let mut availen: usize = SENDLIMIT - minlen;
        let mut lenleft = value.len();
        let mut ind = 0;
        let mut contin = "";
        println!("{} op={} value={}", self.uuid, op, value);
        while lenleft != 0 {
            let max = if lenleft > availen {lenleft = lenleft - availen; ind + availen } 
                    else { let x=lenleft; lenleft = 0; ind + x };
            println!("minlen={} valeft={} ind={} max={}\n{} {} {}", minlen, lenleft, ind, max,
                    self.uuid, op, contin);
            (&self.file).write_all(format!("{} {} {}{}\n", self.uuid, op, contin, &value[ind..max]).as_bytes()).unwrap();
            // write!(&self.file, "{} {} {}{}\n", self.uuid, op, contin, &value[ind..max]).unwrap();
            contin = "*";
            ind = max ;
            minlen = self.uuid.len() + op.len() + 2 + 1;
            availen = SENDLIMIT - minlen;
        };
        (&self.file).flush().unwrap();
    }
    
    pub fn wisk_reportops(self: &Self, mut op: Ops) {
        // if let Ops::ENV(ref mut map) = op {
        //     for (key, val) in env::vars_os() {
        //         if let (Ok(k), Ok(v)) = (key.into_string(), val.into_string()) {
        //             map.append(vec!(k, v));
        //         }
        //     }
        // }
        let serialized = serde_json::to_string(&op).unwrap();
        println!("serialized = {:?}", serialized);
        self.wisk_reportop("ENV", &serialized);
    }

}


lazy_static! {
    pub static ref TRACKER : Tracker = Tracker::init();
    // pub static ref TRACKER : Mutex<Tracker> = Mutex::new(Tracker::init());
}


#[cfg(test)]
mod reportop_tests {
    use std::io;
    use super::*;

    #[test]
    fn reportop_test_000() -> io::Result<()> {
        TRACKER.wisk_reportop("test_000", "");
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(!buffer.contains(&format!("\n\n")));
        assert!(!buffer.contains(&format!("{} test_000\n", TRACKER.uuid)));
        Ok(())
    }

    #[test]
    fn reportop_test_001() -> io::Result<()> {
        TRACKER.wisk_reportop("test_001", "D");
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_001 D\n", TRACKER.uuid)));
        // TRACKER.wisk_reportop("tests_001", "D");
        Ok(())
    }

    #[test]
    fn reportop_test_002() -> io::Result<()> {
        TRACKER.wisk_reportop("test_002", &"D".repeat(SENDLIMIT-42));
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_002 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-42))));
        Ok(())
    }

    #[test]
    fn reportop_tests_003() -> io::Result<()> {
        TRACKER.wisk_reportop("test_003", &"D".repeat(SENDLIMIT-41));
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_003 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-42))));
        assert!(buffer.contains(&format!("{} test_003 *{}\n", TRACKER.uuid, &"D".repeat(1))));
        // assert!(buffer, format!("X C {}\nX C *{}\n", &"D".repeat(SENDLIMIT-4), &"D".repeat(1)));
        Ok(())
    }

    #[test]
    fn reportop_test_004() -> io::Result<()> {
        TRACKER.wisk_reportop("test_004", &"D".repeat(SENDLIMIT*2-9));
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_004 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-42))));
        assert!(buffer.contains(&format!("{} test_004 *{}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-43))));
        Ok(())
    }

    #[test]
    fn reportop_test_005() -> io::Result<()> {
        TRACKER.wisk_reportop("test_005", &"D".repeat(SENDLIMIT*2-(42*2)));
        println!("FileName: {}", TRACKER.filename);
        let mut rfile = File::open(&TRACKER.filename)?;
        let mut buffer = String::new();
        rfile.read_to_string(&mut buffer)?;
        assert!(buffer.contains(&format!("{} test_005 {}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-42))));
        assert!(buffer.contains(&format!("{} test_005 *{}\n", TRACKER.uuid, &"D".repeat(SENDLIMIT-43))));
        assert!(buffer.contains(&format!("{} test_005 *{}\n", TRACKER.uuid, &"D".repeat(1))));
        Ok(())
    }
}

