//! Shared scaffolding for the viewer-style example applications.
//!
//! Every example previously re-implemented the same camera interaction loop:
//! arrow-key orbit, right-drag rotate, scroll-wheel dolly, plus the standard
//! `Tick`/`Event` subscription batch. This module centralizes those so examples
//! only carry their own scene setup and custom messages.

use crate::gpu::pipelines::planet::camera::Camera;
use iced::event::Event;
use iced::keyboard::{self, Key, key::Named};
use iced::mouse::{self, Button, ScrollDelta};

/// How the arrow keys drive the camera.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArrowAction {
    /// Orbit around the target (viewer examples).
    #[default]
    Orbit,
    /// Pan in the camera's view plane (shapes example).
    Pan,
}

/// Mouse/keyboard camera interaction shared by all viewer examples.
#[derive(Debug, Clone)]
pub struct CameraControl {
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
    /// How arrow keys act on the camera.
    pub arrow_action: ArrowAction,
    /// Keyboard rotation step per arrow press, in radians.
    pub rotate_angle: f32,
    /// Dolly distance for a `+`/`-` press.
    pub zoom_amount: f32,
    /// Fraction of the current camera distance applied per wheel "line".
    pub wheel_zoom_fraction: f32,
    /// Pan distance per arrow press (when `arrow_action` is `Pan`).
    pub pan_amount: f32,
    /// Orbit sensitivity while right-dragging, radians per pixel.
    pub drag_sensitivity: f32,
    /// Wheel `Pixels` deltas are scaled by this divisor (lines are not).
    pub wheel_pixel_divisor: f32,
    /// Multiply the wheel dolly by this sign.
    pub wheel_sign: f32,
}

impl Default for CameraControl {
    fn default() -> Self {
        Self {
            cursor_position: None,
            drag_start: None,
            right_button_down: false,
            arrow_action: ArrowAction::Orbit,
            rotate_angle: 5.0_f32.to_radians(),
            zoom_amount: 500.0,
            wheel_zoom_fraction: 0.05,
            pan_amount: 0.75,
            drag_sensitivity: 0.005,
            wheel_pixel_divisor: 100.0,
            wheel_sign: 1.0,
        }
    }
}

impl CameraControl {
    /// Handle an iced event, mutating the camera. Returns `true` if the event
    /// was consumed (mouse tracking, wheel, or an arrow/zoom key), so examples
    /// can chain additional key handling.
    pub fn handle_event(&mut self, event: &Event, camera: &mut Camera) -> bool {
        match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                self.handle_keyboard(key, camera)
            }
            Event::Mouse(event) => self.handle_mouse(event, camera),
            _ => false,
        }
    }

    fn handle_keyboard(&mut self, key: &Key, camera: &mut Camera) -> bool {
        match key {
            Key::Named(Named::ArrowLeft) => {
                match self.arrow_action {
                    ArrowAction::Orbit => camera.rotate_around_up(-self.rotate_angle),
                    ArrowAction::Pan => camera.pan(-self.pan_amount, 0.0),
                }
                true
            }
            Key::Named(Named::ArrowRight) => {
                match self.arrow_action {
                    ArrowAction::Orbit => camera.rotate_around_up(self.rotate_angle),
                    ArrowAction::Pan => camera.pan(self.pan_amount, 0.0),
                }
                true
            }
            Key::Named(Named::ArrowUp) => {
                match self.arrow_action {
                    ArrowAction::Orbit => camera.rotate_vertically(-self.rotate_angle),
                    ArrowAction::Pan => camera.pan(0.0, self.pan_amount),
                }
                true
            }
            Key::Named(Named::ArrowDown) => {
                match self.arrow_action {
                    ArrowAction::Orbit => camera.rotate_vertically(self.rotate_angle),
                    ArrowAction::Pan => camera.pan(0.0, -self.pan_amount),
                }
                true
            }
            Key::Character(ch) if ch == "+" || ch == "=" => {
                camera.dolly(-self.zoom_amount);
                true
            }
            Key::Character(ch) if ch == "-" || ch == "_" => {
                camera.dolly(self.zoom_amount);
                true
            }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, event: &mouse::Event, camera: &mut Camera) -> bool {
        match event {
            mouse::Event::CursorMoved { position } => {
                let (x, y) = (position.x, position.y);
                if self.right_button_down {
                    if let Some((px, py)) = self.drag_start {
                        camera.rotate_around_up(-(x - px) * self.drag_sensitivity);
                        camera.rotate_vertically(-(y - py) * self.drag_sensitivity);
                        self.drag_start = Some((x, y));
                    } else {
                        self.drag_start = Some((x, y));
                    }
                }
                self.cursor_position = Some((x, y));
                true
            }
            mouse::Event::ButtonPressed(Button::Right) => {
                self.right_button_down = true;
                self.drag_start = self.cursor_position;
                true
            }
            mouse::Event::ButtonReleased(Button::Right) => {
                self.right_button_down = false;
                self.drag_start = None;
                true
            }
            mouse::Event::WheelScrolled { delta } => {
                let distance = (camera.target - camera.eye).norm();
                let amount = match delta {
                    ScrollDelta::Lines { y, .. } => {
                        y * distance * self.wheel_zoom_fraction
                    }
                    ScrollDelta::Pixels { y, .. } => {
                        y * distance * self.wheel_zoom_fraction / self.wheel_pixel_divisor
                    }
                };
                camera.dolly(amount * self.wheel_sign);
                true
            }
            _ => false,
        }
    }
}

/// The standard `Tick` + `Event` subscription batch used by every viewer example.
///
/// Both mappings must be non-capturing (zero-sized) closures — pass `|_| Message::Tick`
/// and the `Message::Event` variant constructor directly.
pub fn subscription<M, F, G>(tick: F, on_event: G) -> iced::Subscription<M>
where
    F: Fn(std::time::Instant) -> M + Send + Sync + Clone + 'static,
    G: Fn(Event) -> M + Send + Sync + Clone + 'static,
    M: 'static,
{
    iced::Subscription::batch([
        iced::time::every(std::time::Duration::from_millis(16)).map(tick),
        iced::event::listen().map(on_event),
    ])
}