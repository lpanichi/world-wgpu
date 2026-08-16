use iced::wgpu;

/// Shared MSAA sample count for all scene pipelines and their MSAA/depth targets.
///
/// Kept in one place so switching the MSAA target format or count (required for
/// HDR/bloom work) touches only this file.
pub const MSAA_SAMPLE_COUNT: u32 = 4;

/// Shared depth texture format used by the MSAA depth target and every scene pipeline.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;