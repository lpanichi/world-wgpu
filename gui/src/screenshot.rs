//! Optional auto-screenshot support for the README example images.
//!
//! Any example binary can be launched with `--screenshot[=out.png]` (or the
//! `WORLD_SCREENSHOT` environment variable) to render for a short settle
//! period, grab the window framebuffer through iced's `window::screenshot`
//! API, write the PNG to disk and exit. The `make screenshots` target drives
//! every example this way so the images in `docs/screenshots` can be
//! regenerated on demand.
//!
//! Wiring per example:
//!  1. add a `shot: gui::screenshot::AutoShot` field, initialised with
//!     `AutoShot::from_env()`;
//!  2. add a `Shot(gui::screenshot::ShotMessage)` variant to the app's
//!     `Message` enum;
//!  3. in `update`, return `iced::Task<Message>` and forward the screenshot
//!     flow: call `sim.shot.on_frame()` from the tick handler and
//!     `sim.shot.handle(msg)` from the `Message::Shot` arm;
//!  4. in `main`, prefer a fixed window size (e.g. 1024×768) while a capture
//!     is requested.

use std::path::PathBuf;

use iced::window;
use iced::Task;

/// Number of rendered frames (ticks) to wait before grabbing the framebuffer.
///
/// Long enough for the GPU pipeline to be built (assets decode synchronously
/// on the first render when not preloaded) while keeping the wait short.
pub const SETTLE_FRAMES: u32 = 120;

/// Logical window dimensions used for README captures.
pub const CAPTURE_SIZE: iced::Size = iced::Size::new(1024.0, 768.0);

/// The window size to apply while capturing screenshots, if enabled.
pub fn window_size() -> Option<iced::Size> {
    AutoShot::from_env().enabled().then_some(CAPTURE_SIZE)
}

/// Messages produced by the [`AutoShot`] state machine.
#[derive(Debug, Clone)]
pub enum ShotMessage {
    /// The resolved id of the main (oldest) window.
    WindowId(Option<window::Id>),
    /// A captured framebuffer that should be written to disk.
    Captured(window::Screenshot),
}

/// Auto-screenshot state machine.
#[derive(Debug)]
pub struct AutoShot {
    output: Option<PathBuf>,
    frames_left: u32,
    window_id: Option<window::Id>,
    requested: bool,
    done: bool,
}

impl AutoShot {
    /// A disabled instance that never captures.
    pub fn disabled() -> Self {
        Self {
            output: None,
            frames_left: 0,
            window_id: None,
            requested: false,
            done: false,
        }
    }

    /// Parses `--screenshot[=out.png]` from the CLI args, falling back to the
    /// `WORLD_SCREENSHOT` environment variable.
    pub fn from_env() -> Self {
        let output = std::env::args()
            .find_map(|arg| arg.strip_prefix("--screenshot=").map(PathBuf::from))
            .or_else(|| std::env::var("WORLD_SCREENSHOT").ok().map(PathBuf::from));

        match output {
            Some(path) => Self {
                output: Some(path),
                frames_left: SETTLE_FRAMES,
                window_id: None,
                requested: false,
                done: false,
            },
            None => Self::disabled(),
        }
    }

    /// Whether a capture has been requested.
    pub fn enabled(&self) -> bool {
        self.output.is_some()
    }

    /// The path the screenshot will be written to, if any.
    pub fn output(&self) -> Option<&PathBuf> {
        self.output.as_ref()
    }

    /// Restarts the settle countdown. Useful for examples that must wait for
    /// asynchronous boot work (e.g. asset preloading) before the scene is
    /// actually rendered.
    pub fn restart_countdown(&mut self) {
        if self.enabled() && !self.requested {
            self.frames_left = SETTLE_FRAMES;
        }
    }

    /// Call once per rendered tick. Returns a task to run once the settle
    /// period elapses.
    pub fn on_frame(&mut self) -> Option<Task<ShotMessage>> {
        if !self.enabled() || self.done || self.requested {
            return None;
        }
        if self.frames_left > 0 {
            self.frames_left -= 1;
        }
        if self.frames_left == 0 {
            self.requested = true;
            Some(window::oldest().map(ShotMessage::WindowId))
        } else {
            None
        }
    }

    /// Handle a [`ShotMessage`] produced by the tasks returned by this helper.
    pub fn handle(&mut self, message: ShotMessage) -> Option<Task<ShotMessage>> {
        match message {
            ShotMessage::WindowId(Some(id)) => {
                self.window_id = Some(id);
                Some(window::screenshot(id).map(ShotMessage::Captured))
            }
            ShotMessage::WindowId(None) => {
                eprintln!("screenshot: could not find the application window");
                self.done = true;
                None
            }
            ShotMessage::Captured(shot) => {
                self.done = true;
                match &self.output {
                    Some(path) => match save_png(path, &shot) {
                        Ok(()) => eprintln!("screenshot: wrote {}", path.display()),
                        Err(error) => {
                            eprintln!("screenshot: failed to write {}: {error}", path.display())
                        }
                    },
                    None => eprintln!("screenshot: captured but no output path configured"),
                }
                self.window_id.map(window::close::<ShotMessage>)
            }
        }
    }

    /// Whether the capture is complete and the app can exit.
    pub fn done(&self) -> bool {
        self.done
    }
}

/// Encodes a captured [`window::Screenshot`] to a PNG file.
fn save_png(path: &PathBuf, shot: &window::Screenshot) -> anyhow::Result<()> {
    let width = shot.size.width;
    let height = shot.size.height;
    let image = image::RgbaImage::from_raw(width, height, shot.rgba.to_vec())
        .ok_or_else(|| anyhow::anyhow!("invalid framebuffer size {width}x{height}"))?;
    image.save(path)?;
    Ok(())
}