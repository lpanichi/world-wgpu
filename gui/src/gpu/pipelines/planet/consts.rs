use iced::wgpu;

/// Shared MSAA sample count for all scene pipelines and their MSAA/depth targets.
///
/// Kept in one place so switching the MSAA target format or count (required for
/// HDR/bloom work) touches only this file.
pub const MSAA_SAMPLE_COUNT: u32 = 4;

/// Shared depth texture format used by the MSAA depth target and every scene pipeline.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

/// HDR intermediate render-target format used by the MSAA color texture.
///
/// Scene pipelines write linear HDR values into a floating-point target so that
/// highlight values can exceed `1.0`; the resolve pass then applies ACES tone
/// mapping and final gamma encoding before presenting to the (typically 8-bit)
/// surface.
pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;