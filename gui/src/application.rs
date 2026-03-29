// use std::sync::Arc;

// use log::{debug, error, info};
// use winit::event_loop::ActiveEventLoop;
// use winit::{
//     application::ApplicationHandler,
//     event::WindowEvent,
//     window::{Window, WindowId},
// };

// use crate::gpu::Gpu;
// use crate::simulation::simulation::{Simulation, SimulationUpdateResult};

// #[derive(Default)]
// pub struct App<'a> {
//     window: Option<Arc<Window>>,
//     gpu: Option<Gpu<'a>>,
//     simulation: Option<Simulation>,
// }

// impl App<'_> {
//     fn update_simulation(&mut self, event: &WindowEvent) -> SimulationUpdateResult {
//         self.simulation
//             .as_mut()
//             .unwrap()
//             .update(event, self.gpu.as_mut().unwrap())
//     }

//     fn redraw_simulation(&mut self) -> Result<(), wgpu::SurfaceError> {
//         self.simulation
//             .as_mut()
//             .unwrap()
//             .render(self.gpu.as_mut().unwrap())
//     }
// }

// impl<'a> ApplicationHandler for App<'a> {
//     fn resumed(&mut self, event_loop: &ActiveEventLoop) {
//         debug!("resumed called");

//         // Create a new window and store it in an Arc
//         let window = Arc::new(
//             event_loop
//                 .create_window(Window::default_attributes())
//                 .expect("Failed to create window"),
//         );

//         // Initialize GPU asynchronously
//         let gpu = pollster::block_on(Gpu::new_async(window.clone()));

//         // Store the window and GPU in the App struct first
//         self.window = Some(window);
//         self.gpu = Some(gpu);

//         // Initialize simulation
//         match self.simulation {
//             None => {
//                 let simulation = Simulation::new(self.gpu.as_ref().unwrap(), 5.0);
//                 self.simulation = Some(simulation);
//             }
//             Some(_) => {}
//         }
//     }

//     fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
//         debug!("Window event {event:?} for window id {_id:?}");
//         match event {
//             WindowEvent::CloseRequested => {
//                 info!("The close button was pressed; stopping");
//                 // Clear memory or you get a segfault
//                 self.simulation = None;
//                 self.gpu = None;
//                 self.window = None;
//                 event_loop.exit();
//             }
//             WindowEvent::Resized(physical_size) => {
//                 self.gpu.as_mut().unwrap().resize(physical_size);
//             }
//             WindowEvent::RedrawRequested => match self.redraw_simulation() {
//                 Ok(_) => {
//                     debug!("Redraw done")
//                 }
//                 Err(_) => {
//                     error!("Redraw error")
//                 }
//             },
//             WindowEvent::Destroyed => {}
//             _ => match self.update_simulation(&event) {
//                 SimulationUpdateResult::RequiresRedraw => self.redraw_simulation().unwrap(),
//                 _ => {}
//             },
//         }
//     }
// }
