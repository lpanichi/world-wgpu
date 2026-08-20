pub mod assets;
pub mod maths;
pub mod pipelines;

#[cfg(test)]
mod tests {
    /// Parse every WGSL shader with naga so syntax/semantic errors are caught
    /// by `cargo test` instead of at pipeline creation time at runtime.
    #[test]
    fn all_shaders_parse() {
        let shaders = [
            "atmosphere_shader.wgsl",
            "bloom_shader.wgsl",
            "clear_quad_shader.wgsl",
            "cloud_shader.wgsl",
            "colored_line_shader.wgsl",
            "milky_way_shader.wgsl",
            "moon_shader.wgsl",
            "planet_shader.wgsl",
            "satellite_shader.wgsl",
            "star_catalog_shader.wgsl",
            "station_shader.wgsl",
            "sun_shader.wgsl",
            "text_shader.wgsl",
        ];
        for name in shaders {
            let path = format!("{}/src/gpu/shaders/{name}", env!("CARGO_MANIFEST_DIR"));
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
            let module = iced::wgpu::naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|e| panic!("{name} failed to parse: {}", e.emit_to_string(&source)));
            // Full validation (same as wgpu performs at pipeline creation):
            // catches uniformity-analysis violations around derivative calls,
            // type errors, invalid control flow, etc.
            let info = iced::wgpu::naga::valid::Validator::new(
                iced::wgpu::naga::valid::ValidationFlags::all(),
                iced::wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name} failed validation: {e:?}"));
            drop(info);
        }
    }
}
