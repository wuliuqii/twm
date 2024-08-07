#[macro_use]
extern crate tracing;

mod handlers;

mod grabs;
mod input;
mod state;
mod winit;

use std::env;

use smithay::reexports::calloop::EventLoop;
use smithay::reexports::wayland_server::{Display, DisplayHandle};
pub use state::Twm;
use tracing_subscriber::EnvFilter;

pub struct CalloopData {
    state: Twm,
    display_handle: DisplayHandle,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("RUST_BACKTRACE", "1");

    let directives = env::var("RUST_LOG").unwrap_or_else(|_| "twm=debug,info".to_owned());
    let env_filter = EnvFilter::builder().parse_lossy(directives);
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let mut event_loop: EventLoop<CalloopData> = EventLoop::try_new().unwrap();

    let display: Display<Twm> = Display::new().unwrap();
    let display_handle = display.handle();
    let state = Twm::new(&mut event_loop, display);

    let mut data = CalloopData {
        state,
        display_handle,
    };

    crate::winit::init_winit(&mut event_loop, &mut data)?;

    let mut args = std::env::args().skip(1);
    let flag = args.next();
    let arg = args.next();

    match (flag.as_deref(), arg) {
        (Some("-c") | Some("--command"), Some(command)) => {
            std::process::Command::new(command).spawn().ok();
        }
        _ => {
            std::process::Command::new("foot").spawn().ok();
        }
    }

    event_loop.run(None, &mut data, move |_| {
        // Smallvil is running
    })?;

    Ok(())
}
