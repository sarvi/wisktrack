


#[macro_export]
macro_rules! cevent {
    ($lvl:expr, $form:tt, $($arg:tt)*) => ({
        // TRACER.write_all(format!(concat!("{}: ", $form, "\n"), UUID.as_str(), $($arg)* ).as_bytes());
        // eprintln!(concat!("{}-{}: ", $form), UUID.as_str(), process::id(), $($arg)* );
    });
    ($lvl:expr, $form:tt) => ({
        // TRACER.write_all(format!(concat!("{}: ", $form, "\n"), UUID.as_str(), ).as_bytes());
        // eprintln!(concat!("{}-{}: ", $form), UUID.as_str(), process::id() );
    });
}


#[macro_export]
macro_rules! event {
    ($lvl:expr, $form:tt, $($arg:tt)*) => ({
        // TRACER.write_all(format!(concat!("{}: ", $form, "\n"), UUID.as_str(), $($arg)* ).as_bytes());
        // eprintln!(concat!("{}: ", $form), UUID.as_str(), $($arg)* );
    });
    ($lvl:expr, $form:tt) => ({
        // TRACER.write_all(format!(concat!("{}: ", $form, "\n"), UUID.as_str(), ).as_bytes());
        // eprintln!(concat!("{}: ", $form), UUID.as_str(), );
    });
}

#[macro_export]
macro_rules! errorexit {
    ($msg:tt) => { { eprintln!( concat!("Backtrace:\n{:?}\nWISK_ERROR: ", $msg, "\nParentUUID: {}, UUID: {}, PID: {} Cmd: {:?}"),
                                Backtrace::new(), PUUID.as_str(), UUID.as_str(), process::id(), std::env::args().collect::<Vec<String>>());
                                panic!() } };
    ($msg:tt, $($arg:expr),*) => { { eprintln!( concat!("Backtrace:\n{:?}\nWISK_ERROR: ", $msg, "\nParentUUID: {}, UUID: {}, PID: {} Cmd: {:?}"),
                                                Backtrace::new(), $($arg),*, PUUID.as_str(), UUID.as_str(), process::id(), std::env::args().collect::<Vec<String>>());
                                                panic!() } };
}

#[macro_export]
macro_rules! wiskassert {
    ($cond:expr, $msg:tt) => { assert!($cond, concat!("WISK_ERROR: ", $msg, "\nParentUUID: {}, UUID: {}, PID: {}Cmd: {:?}\n{:?}"),
                                                PUUID.as_str(), UUID.as_str(), process::id(), std::env::args().collect::<Vec<String>>(), Backtrace::new()) };
    ($cond:expr, $msg:tt, $($arg:expr),*) => { assert!($cond, concat!("WISK_ERROR: ", $msg, "\nParentUUID: {}, UUID: {}, PID: {}Cmd: {:?}\n{:?}"),
                                                $($arg),* , PUUID.as_str(), UUID.as_str(), process::id(), std::env::args().collect::<Vec<String>>(), Backtrace::new()) };
}

#[macro_export]
macro_rules! errormsg {
    ($msg:tt) => { format!( concat!("WISK_ERROR: ", $msg, "\nParentUUID: {}, UUID: {}, PID: {}Cmd: {:?}"),
                                                PUUID.as_str(), UUID.as_str(), process::id(), std::env::args().collect::<Vec<String>>()) };
    ($msg:tt, $($arg:expr),*) => { format!( concat!("WISK_ERROR: ", $msg, "\nParentUUID: {}, UUID: {}, PID: {}Cmd: {:?}"),
                                                $($arg),* , PUUID.as_str(), UUID.as_str(), process::id(), std::env::args().collect::<Vec<String>>()) };
}
