use std::env;
use std::sync::Arc;
use std::time::Duration;

use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::ImportAll;
use smithay::desktop::space::{space_render_elements, SpaceRenderElements};
use smithay::desktop::{PopupManager, Space, Window, WindowSurfaceType};
use smithay::input::{Seat, SeatState};
use smithay::output::Output;
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, LoopHandle, LoopSignal, Mode, PostAction};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::WmCapabilities;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Display, DisplayHandle};
use smithay::render_elements;
use smithay::utils::{Logical, Point};
use smithay::wayland::compositor::{CompositorClientState, CompositorState};
use smithay::wayland::output::OutputManagerState;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;
use smithay::wayland::socket::ListeningSocketSource;

use crate::backend::tty::Tty;
use crate::backend::winit::Winit;
use crate::backend::Backend;
use crate::LoopData;

pub struct Twm {
    pub start_time: std::time::Instant,
    pub event_loop: LoopHandle<'static, LoopData>,
    pub stop_signal: LoopSignal,
    pub display_handle: DisplayHandle,

    pub space: Space<Window>,

    // Smithay State
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<State>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,

    pub seat: Seat<State>,
    pub output: Option<Output>,

    pub pointer_buffer: SolidColorBuffer,

    // Set to `true` if there's a redraw queued on the event loop. Reset to `false` in redraw()
    // which means that you cannot queue more than one redraw at once.
    pub redraw_queued: bool,
    pub waiting_for_vblank: bool,
}

pub struct State {
    pub backend: Backend,
    pub twm: Twm,
}

impl State {
    pub fn new(
        event_loop: LoopHandle<'static, LoopData>,
        stop_signal: LoopSignal,
        display: Display<State>,
    ) -> Self {
        let has_display =
            env::var_os("WAYLAND_DISPLAY").is_some() || env::var_os("DISPLAY").is_some();

        let mut backend = if has_display {
            Backend::Winit(Winit::new(event_loop.clone()))
        } else {
            Backend::Tty(Tty::new(event_loop.clone()))
        };

        let mut twm = Twm::new(event_loop, stop_signal, display, &backend);
        backend.init(&mut twm);

        Self { backend, twm }
    }
}

impl Twm {
    pub fn new(
        event_loop: LoopHandle<'static, LoopData>,
        stop_signal: LoopSignal,
        display: Display<State>,
        backend: &Backend,
    ) -> Self {
        let start_time = std::time::Instant::now();

        let display_handle = display.handle();

        let compositor_state = CompositorState::new::<State>(&display_handle);
        let xdg_shell_state = XdgShellState::new_with_capabilities::<State>(
            &display_handle,
            [
                WmCapabilities::Fullscreen,
                WmCapabilities::Maximize,
                WmCapabilities::WindowMenu,
            ],
        );
        let shm_state = ShmState::new::<State>(&display_handle, vec![]);
        let output_manager_state =
            OutputManagerState::new_with_xdg_output::<State>(&display_handle);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<State>(&display_handle);
        let popups = PopupManager::default();

        // A seat is a group of keyboards, pointer and touch devices.
        // A seat typically has a pointer and maintains a keyboard focus and a pointer focus.
        let mut seat: Seat<State> = seat_state.new_wl_seat(&display_handle, backend.seat_name());

        // Notify clients that we have a keyboard, for the sake of the example we assume that
        // keyboard is always present. You may want to track keyboard hot-plug in real
        // compositor.
        seat.add_keyboard(Default::default(), 200, 25).unwrap();

        // Notify clients that we have a pointer (mouse)
        // Here we assume that there is always pointer plugged in
        seat.add_pointer();

        // A space represents a two-dimensional plane. Windows and Outputs can be mapped onto it.
        //
        // Windows get a position and stacking order through mapping.
        // Outputs become views of a part of the Space and can be rendered via Space::render_output.
        let space = Space::default();

        // Creates a new listening socket, automatically choosing the next available `wayland`
        // socket name.
        let listening_socket = ListeningSocketSource::new_auto().unwrap();

        // Get the name of the listening socket.
        // Clients will connect to this socket.
        let socket_name = listening_socket.socket_name().to_os_string();

        event_loop
            .insert_source(listening_socket, move |client_stream, _, data| {
                // Inside the callback, you should insert the client into the display.
                //
                // You may also associate some data with the client when inserting the client.
                data.state
                    .twm
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        std::env::set_var("WAYLAND_DISPLAY", &socket_name);
        info!(
            "listening on Wayland socket: {}",
            socket_name.to_string_lossy()
        );

        // You also need to add the display itself to the event loop, so that client events will be
        // processed by wayland-server.
        event_loop
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, data| {
                    // SAFETY: we don't drop the display.
                    unsafe {
                        display.get_mut().dispatch_clients(&mut data.state).unwrap();
                    }
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        let pointer_buffer = SolidColorBuffer::new((16, 16), [1., 0.8, 0., 1.]);

        Self {
            start_time,
            stop_signal,
            event_loop,
            display_handle,

            space,

            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            popups,

            seat,
            output: None,

            pointer_buffer,

            redraw_queued: false,
            waiting_for_vblank: false,
        }
    }

    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, (p + location).to_f64()))
            })
    }

    pub fn queue_redraw(&mut self) {
        if self.redraw_queued || self.waiting_for_vblank {
            return;
        }

        self.redraw_queued = true;

        self.event_loop.insert_idle(|data| {
            data.state.twm.redraw(&mut data.state.backend);
        });
    }

    pub fn redraw(&mut self, backend: &mut Backend) {
        let _span = tracy_client::span!("redraw");

        assert!(self.redraw_queued);
        assert!(!self.waiting_for_vblank);
        self.redraw_queued = false;

        let elements = space_render_elements(
            backend.renderer(),
            [&self.space],
            self.output.as_ref().unwrap(),
            1.,
        )
        .unwrap();

        let mut elements: Vec<_> = elements
            .into_iter()
            .map(OutputRenderElements::from)
            .collect();
        elements.insert(
            0,
            OutputRenderElements::Pointer(SolidColorRenderElement::from_buffer(
                &self.pointer_buffer,
                self.seat
                    .get_pointer()
                    .unwrap()
                    .current_location()
                    .to_physical_precise_round(1.),
                1.,
                1.,
                Kind::Unspecified,
            )),
        );

        backend.render(self, &elements);

        let output = self.output.as_ref().unwrap();
        self.space.elements().for_each(|window| {
            window.send_frame(
                output,
                self.start_time.elapsed(),
                Some(Duration::ZERO),
                |_, _| Some(output.clone()),
            )
        });

        self.space.refresh();
    }
}

render_elements! {
    pub OutputRenderElements<R, E> where R: ImportAll;
    Space=SpaceRenderElements<R, E>,
    Pointer = SolidColorRenderElement,
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
