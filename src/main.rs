#[macro_use]
extern crate tracing;

mod handlers;

mod backend;
mod grabs;
mod input;
mod state;

use std::env;

use smithay::reexports::calloop::EventLoop;
use smithay::reexports::wayland_server::Display;
use state::State;
pub use state::Twm;
use tracing_subscriber::EnvFilter;

pub struct LoopData {
    state: State,
}

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let directives = env::var("RUST_LOG").unwrap_or_else(|_| "twm=debug,info".to_owned());
    let env_filter = EnvFilter::builder().parse_lossy(directives);
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let mut event_loop: EventLoop<LoopData> = EventLoop::try_new().unwrap();
    let display = Display::new().unwrap();
    let display_handle = display.handle();
    let state = State::new(event_loop.handle(), event_loop.get_signal(), display);

    let mut data = LoopData { state };

    event_loop
        .run(None, &mut data, move |data| {
            // twm is running
            data.state.twm.display_handle.flush_clients().unwrap();
        })
        .unwrap();
}
