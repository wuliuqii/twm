use std::time::Duration;

use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::calloop::LoopHandle;
use smithay::utils::{Rectangle, Transform};

use crate::state::{OutputRenderElements, State, Twm};
use crate::LoopData;

pub struct Winit {
    output: Output,
    backend: WinitGraphicsBackend<GlesRenderer>,
    damage_tracker: OutputDamageTracker,
}

impl Winit {
    pub fn seat_name(&self) -> String {
        "winit".to_owned()
    }

    pub fn renderer(&mut self) -> &mut GlesRenderer {
        self.backend.renderer()
    }

    pub fn render(
        &mut self,
        twm: &mut Twm,
        elements: &[OutputRenderElements<
            GlesRenderer,
            WaylandSurfaceRenderElement<GlesRenderer>,
        >],
    ) {
        let _span = tracy_client::span!("Winit::render");

        let size = self.backend.window_size();
        let damage = Rectangle::from_loc_and_size((0, 0), size);
        self.backend.bind().unwrap();
        self.damage_tracker
            .render_output(self.backend.renderer(), 0, elements, [0.1, 0.1, 0.1, 1.0])
            .unwrap();
        self.backend.submit(Some(&[damage])).unwrap();
    }
}

impl Winit {
    pub fn new(event_loop: LoopHandle<LoopData>) -> Self {
        let (backend, mut winit_event_loop) = winit::init().unwrap();

        let mode = Mode {
            size: backend.window_size(),
            refresh: 60_000,
        };

        let output = Output::new(
            "winit".to_string(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "twm".into(),
                model: "Winit".into(),
            },
        );

        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );
        output.set_preferred(mode);

        let damage_tracker = OutputDamageTracker::from_output(&output);

        let timer = Timer::immediate();
        event_loop
            .insert_source(timer, move |_, _, data| {
                winit_event_loop.dispatch_new_events(|event| match event {
                    WinitEvent::Resized { size, .. } => {
                        data.state
                            .twm
                            .output
                            .as_ref()
                            .unwrap()
                            .change_current_state(
                                Some(Mode {
                                    size,
                                    refresh: 60_000,
                                }),
                                None,
                                None,
                                None,
                            );
                    }
                    WinitEvent::Input(event) => data.state.process_input_event(&mut |_| (), event),
                    WinitEvent::Redraw => data.state.twm.queue_redraw(),
                    WinitEvent::CloseRequested => {
                        data.state.twm.stop_signal.stop();
                    }
                    WinitEvent::Focus(_) => (),
                });
                TimeoutAction::ToDuration(Duration::from_millis(16))
            })
            .unwrap();

        Self {
            output,
            backend,
            damage_tracker,
        }
    }

    pub fn init(&mut self, twm: &mut Twm) {
        let _global = self.output.create_global::<State>(&twm.display_handle);
        twm.space.map_output(&self.output, (0, 0));
        twm.output = Some(self.output.clone());
    }
}
