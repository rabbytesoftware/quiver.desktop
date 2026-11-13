//! Quiver Desktop. `quiver` starts the real app; `quiver --demo` starts it over seeded data and no
//! daemon (screenshots, design review).

use std::process::ExitCode;
use std::sync::Arc;

use quiver_app::services::AppServices;
use quiver_app::{app, demo};

fn main() -> ExitCode {
    quiver_app::display::prefer_x11();
    let demo_mode = std::env::args().any(|arg| arg == "--demo");
    let started = if demo_mode {
        // The demo owns its runtime for the whole process: leaked on purpose, the OS reclaims it.
        match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => demo::services(Box::leak(Box::new(runtime))),
            Err(err) => Err(quiver_app::services::Error::Runtime(err.to_string())),
        }
    } else {
        AppServices::start().map(Arc::new)
    };
    match started {
        Ok(services) => {
            app::run(
                services,
                demo_mode,
                demo_mode.then_some(demo::seed as fn(&_)),
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("quiver could not start: {err}");
            ExitCode::FAILURE
        }
    }
}
