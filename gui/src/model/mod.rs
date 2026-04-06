pub mod ground_station;
pub mod orbit;
pub mod satellite;
pub mod shapes;
pub mod system;
pub mod text_vertices;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameMode {
    Eci,
    Ecef,
}
