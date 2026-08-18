//! Optional CPU-side asset preloading for the renderer.
//!
//! The heavy work (JPEG decode, mip-chain generation, star-catalog parsing) happens
//! synchronously inside `Pipelines::new`, which iced invokes lazily on the first
//! render of the shader widget. Consumers that want a loading screen can call
//! [`load_async`] in their boot task, show [`crate::ui::components::loading::loading_screen`]
//! until it completes, then call [`install`] before the shader widget first appears.
//!
//! When no assets are installed, the pipelines fall back to synchronous loading,
//! so using this module is entirely optional.

use std::sync::{Arc, OnceLock};

use anyhow::Result;
use image::RgbaImage;

use crate::gpu::pipelines::planet::{
    star_catalog::{self, StarInstance},
    texture,
};

/// Earth texture resolution to preload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureQuality {
    /// `earthmap1k.jpg` — fast decode, small footprint.
    LowRes,
    /// `earthmap4k.jpg` — highest visual quality.
    HighRes,
}

/// CPU-side render assets: the earth mip chain and the star catalog instances.
#[derive(Debug, Clone)]
pub struct PreloadedAssets {
    pub earth_mips: Vec<RgbaImage>,
    pub star_instances: Vec<StarInstance>,
}

static ASSETS: OnceLock<Arc<PreloadedAssets>> = OnceLock::new();

/// Preload the CPU-side assets for a given earth texture resolution.
pub fn load(quality: TextureQuality) -> Result<PreloadedAssets> {
    let bytes: &[u8] = match quality {
        TextureQuality::LowRes => include_bytes!("textures/earthmap1k.jpg"),
        TextureQuality::HighRes => include_bytes!("textures/earthmap4k.jpg"),
    };
    let earth_mips = texture::decode(bytes)?;
    let star_instances = star_catalog::load_star_instances();
    Ok(PreloadedAssets {
        earth_mips,
        star_instances,
    })
}

/// Load [`PreloadedAssets`] on a worker thread. Call from an iced `Task::perform`.
pub async fn load_async(quality: TextureQuality) -> Result<Arc<PreloadedAssets>> {
    let assets = smol::unblock(move || load(quality).map(Arc::new)).await?;
    Ok(assets)
}

/// Install preloaded assets for the renderer to consume. Idempotent: only the
/// first install takes effect.
pub fn install(assets: Arc<PreloadedAssets>) {
    let _ = ASSETS.set(assets);
}

/// The currently installed assets, if any.
pub fn get() -> Option<&'static Arc<PreloadedAssets>> {
    ASSETS.get()
}