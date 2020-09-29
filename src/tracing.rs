// use tracing::instrument;
use tracing::Level;
use tracing::dispatcher::Dispatch;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::FmtSubscriber;





pub fn make_dispatch(tracevar: &str) -> (bool, Dispatch, WorkerGuard) {
    let file_appender;
    let tracing;
    if let Ok(tracefile) =  env::var(tracevar) {
        file_appender = tracing_appender::rolling::never("", tracefile);
        tracing = true
    } else {
        file_appender = tracing_appender::rolling::never("", "/dev/null");
        tracing = false
    }
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_writer(non_blocking)
        .finish();
    (tracing, Dispatch::new(subscriber), guard)
}

#[macro_export]
macro_rules! dotrace {

     ({ $($body:tt)* }) => {
        if stringify!($real_fn) == "fopen" && !MY_DISPATCH_initialized.with(Cell::get) {
            $($body)*
        } else {
            MY_DISPATCH.with(|(tracing, my_dispatch, _guard)| {
                // println!("tracing: {}", tracing);
                if *tracing {
                    with_default(&my_dispatch, || {
                        // event!(Level::INFO, "{}()", stringify!($real_fn));
                        $($body)*
                    })
                } else {
                    $($body)*
                }
            })
        }
        
     };
}



macro vhook

if $reqforinit {
    if !MY_DISPATCH_initialized.with(Cell::get) {
        $($body)*
    } else {
        MY_DISPATCH.with(|(tracing, my_dispatch, _guard)| {
            // println!("tracing: {}, {:?}", tracing, $va);
            if *tracing {
                with_default(&my_dispatch, || {
                    // event!(Level::INFO, "{}()", stringify!($real_fn));
                    $($body)*
                })
            } else {
                $($body)*
            }
        })
    }
} else {
    MY_DISPATCH.with(|(tracing, my_dispatch, _guard)| {
        // println!("tracing: {}, {:?}", tracing, $va);
        if *tracing {
            with_default(&my_dispatch, || {
                // event!(Level::INFO, "{}()", stringify!($real_fn));
                $($body)*
            })
        } else {
            $($body)*
        }
    })
}






dhook
if $reqforinit {
    if !MY_DISPATCH_initialized.with(Cell::get) {
        $($body)*
    } else {
        MY_DISPATCH.with(|(tracing, my_dispatch, _guard)| {
            // println!("tracing: {}, {:?}", tracing, $va);
            if *tracing {
                with_default(&my_dispatch, || {
                    // event!(Level::INFO, "{}()", stringify!($real_fn));
                    $($body)*
                })
            } else {
                $($body)*
            }
        })
    }
} else {
    MY_DISPATCH.with(|(tracing, my_dispatch, _guard)| {
        // println!("tracing: {}, {:?}", tracing, $va);
        if *tracing {
            with_default(&my_dispatch, || {
                // event!(Level::INFO, "{}()", stringify!($real_fn));
                $($body)*
            })
        } else {
            $($body)*
        }
    })
}
