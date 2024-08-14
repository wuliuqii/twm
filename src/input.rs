use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
    KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
};
use smithay::input::keyboard::{FilterResult, Keysym};
use smithay::input::pointer::{AxisFrame, ButtonEvent, MotionEvent, RelativeMotionEvent};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::SERIAL_COUNTER;

use crate::state::State;

enum KeyAction {
    Quit,
    Terminal,
    ChangeVt(i32),
}

impl State {
    pub fn process_input_event<I: InputBackend>(
        &mut self,
        change_vt: &mut dyn FnMut(i32),
        event: InputEvent<I>,
    ) {
        let _span = tracy_client::span!("process_input_event");
        trace!("process_input_event");

        match event {
            InputEvent::Keyboard { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);
                let press_state = event.state();

                let action = self.twm.seat.get_keyboard().unwrap().input(
                    self,
                    event.key_code(),
                    press_state,
                    serial,
                    time,
                    |_, _, keysym| {
                        if press_state == KeyState::Pressed {
                            let sym = keysym.modified_sym();
                            if sym == Keysym::Q {
                                FilterResult::Intercept(KeyAction::Quit)
                            } else if sym == Keysym::T {
                                FilterResult::Intercept(KeyAction::Terminal)
                            } else if sym >= Keysym::XF86_Switch_VT_1
                                || sym <= Keysym::XF86_Switch_VT_12
                            {
                                // let vt = (sym.raw() - Keysym::XF86_Switch_VT_1.raw() + 1) as i32;
                                let vt = sym.raw().wrapping_sub(Keysym::XF86_Switch_VT_1.raw())
                                    as i32
                                    + 1;

                                FilterResult::Intercept(KeyAction::ChangeVt(vt))
                            } else {
                                FilterResult::Forward
                            }
                        } else {
                            FilterResult::Forward
                        }
                    },
                );

                match action {
                    Some(KeyAction::Quit) => {
                        info!("quitting because Q was pressed");
                        self.twm.stop_signal.stop();
                    }
                    Some(KeyAction::Terminal) => {
                        std::process::Command::new("foot").spawn().ok();
                    }
                    Some(KeyAction::ChangeVt(vt)) => {
                        (*change_vt)(vt);
                    }
                    None => {}
                }
            }
            InputEvent::PointerMotion { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();

                let pointer = self.twm.seat.get_pointer().unwrap();
                let mut pointer_location = pointer.current_location();

                pointer_location += event.delta();

                let output = self.twm.space.outputs().next().unwrap();
                let output_geo = self.twm.space.output_geometry(output).unwrap();

                pointer_location.x = pointer_location.x.clamp(0., output_geo.size.w as f64);
                pointer_location.y = pointer_location.y.clamp(0., output_geo.size.h as f64);

                let under = self.twm.surface_under(pointer_location);
                pointer.motion(
                    self,
                    under.clone(),
                    &MotionEvent {
                        location: pointer_location,
                        serial,
                        time: event.time_msec(),
                    },
                );

                pointer.relative_motion(
                    self,
                    under,
                    &RelativeMotionEvent {
                        delta: event.delta(),
                        delta_unaccel: event.delta_unaccel(),
                        utime: event.time(),
                    },
                );

                self.twm.queue_redraw();
            }
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.twm.space.outputs().next().unwrap();

                let output_geo = self.twm.space.output_geometry(output).unwrap();

                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let serial = SERIAL_COUNTER.next_serial();

                let pointer = self.twm.seat.get_pointer().unwrap();

                let under = self.twm.surface_under(pos);

                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);

                self.twm.queue_redraw();
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.twm.seat.get_pointer().unwrap();
                let keyboard = self.twm.seat.get_keyboard().unwrap();

                let serial = SERIAL_COUNTER.next_serial();

                let button = event.button_code();

                let button_state = event.state();

                if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
                    if let Some((window, _loc)) = self
                        .twm
                        .space
                        .element_under(pointer.current_location())
                        .map(|(w, l)| (w.clone(), l))
                    {
                        self.twm.space.raise_element(&window, true);
                        keyboard.set_focus(
                            self,
                            Some(window.toplevel().unwrap().wl_surface().clone()),
                            serial,
                        );
                        self.twm.space.elements().for_each(|window| {
                            window.toplevel().unwrap().send_pending_configure();
                        });
                    } else {
                        self.twm.space.elements().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().unwrap().send_pending_configure();
                        });
                        keyboard.set_focus(self, Option::<WlSurface>::None, serial);
                    }
                };

                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event, .. } => {
                let source = event.source();

                let horizontal_amount = event.amount(Axis::Horizontal).unwrap_or_else(|| {
                    event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.
                });
                let vertical_amount = event.amount(Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.
                });
                let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
                let vertical_amount_discrete = event.amount_v120(Axis::Vertical);

                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                if horizontal_amount != 0.0 {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);
                    if let Some(discrete) = horizontal_amount_discrete {
                        frame = frame.v120(Axis::Horizontal, discrete as i32);
                    }
                }
                if vertical_amount != 0.0 {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    if let Some(discrete) = vertical_amount_discrete {
                        frame = frame.v120(Axis::Vertical, discrete as i32);
                    }
                }

                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        frame = frame.stop(Axis::Vertical);
                    }
                }

                let pointer = self.twm.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }
}
