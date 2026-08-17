use anyhow::*;
use iced::wgpu;
use image::{GenericImageView, RgbaImage};

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
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();
        let mip_count = dimensions.0.max(dimensions.1).ilog2() + 1;
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        write_image_level(queue, &texture, 0, &rgba);

        let mut level = 0;
        let mut mip = rgba;
        while mip.width() > 1 || mip.height() > 1 {
            level += 1;
            mip = image::imageops::resize(
                &mip,
                (mip.width() / 2).max(1),
                (mip.height() / 2).max(1),
                image::imageops::FilterType::Triangle,
            );
            write_image_level(queue, &texture, level, &mip);
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
