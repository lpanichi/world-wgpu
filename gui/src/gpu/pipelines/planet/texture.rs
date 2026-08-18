use anyhow::*;
use iced::wgpu;
use image::RgbaImage;

#[derive(Debug)]
pub struct Texture {
    #[allow(unused)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let mips = decode(bytes)?;
        Self::from_preloaded(device, queue, &mips, label)
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let mips = mip_chain(&img.to_rgba8());
        Self::from_preloaded(device, queue, &mips, label.unwrap_or("Texture"))
    }

    /// Upload a pre-decoded mip chain (level 0 is the full-res image).
    pub fn from_preloaded(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mips: &[RgbaImage],
        label: &str,
    ) -> Result<Self> {
        let (width, height) = mips[0].dimensions();
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: mips.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (level, mip) in mips.iter().enumerate() {
            write_image_level(queue, &texture, level as u32, mip);
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}

/// Decode an encoded image (JPEG/PNG/…) into a full RGBA mip chain. Pure CPU.
pub fn decode(bytes: &[u8]) -> Result<Vec<RgbaImage>> {
    let img = image::load_from_memory(bytes)?;
    Ok(mip_chain(&img.to_rgba8()))
}

/// Build a successive-halving mip chain down to 1x1 from a full-res RGBA image. Pure CPU.
pub fn mip_chain(rgba: &RgbaImage) -> Vec<RgbaImage> {
    let mut levels = vec![rgba.clone()];
    let mut mip = rgba.clone();
    while mip.width() > 1 || mip.height() > 1 {
        let (w, h) = ((mip.width() / 2).max(1), (mip.height() / 2).max(1));
        mip = image::imageops::resize(&mip, w, h, image::imageops::FilterType::Triangle);
        levels.push(mip.clone());
    }
    levels
}

/// Write an RGBA image to a mip level, padding each row to wgpu's alignment
/// requirement for buffer-to-texture copies.
fn write_image_level(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    image: &RgbaImage,
) {
    let (width, height) = image.dimensions();
    let row_bytes = (width as usize) * 4;
    let aligned_row_bytes =
        row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);

    let raw = image.as_raw();
    let mut padded = vec![0u8; aligned_row_bytes * height as usize];
    for row in 0..height as usize {
        let src = &raw[row * row_bytes..(row + 1) * row_bytes];
        let start = row * aligned_row_bytes;
        padded[start..start + src.len()].copy_from_slice(src);
    }

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture,
            mip_level,
            origin: wgpu::Origin3d::ZERO,
        },
        &padded,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(aligned_row_bytes as u32),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}
