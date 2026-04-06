# MSAA in the Planet Shader Pipeline

This document explains how multisample antialiasing (MSAA) is implemented for the custom planet shader widget and why it uses two render passes.

## Goal

The implementation needs to satisfy both constraints:

- Keep edge antialiasing for the 3D scene.
- Avoid corrupting Iced UI colors and backgrounds outside the shader clip region.

## Where It Lives

Main orchestration is in:

- `gui/src/gpu/pipelines/planet/pipelines.rs`

Resolve helper pipeline and shader are in:

- `gui/src/gpu/pipelines/planet/resolve_msaa.rs`
- `gui/src/gpu/shaders/resolve_msaa_shader.wgsl`

## Render Flow

### 1) Resource setup in prepare

When viewport size changes, `Pipelines::prepare` recreates:

- A multisampled color texture (`MSAA Color Texture`) with `sample_count = MSAA_SAMPLE_COUNT`.
- A multisampled depth texture (`Depth Texture`) with the same sample count.

It then binds the MSAA color texture view as the source for resolve/composite:

- `self.resolve_msaa.set_source(device, &msaa_view)`

### 2) Scene pass (MSAA render target)

In `Pipelines::render`, first pass:

- Target: multisampled color texture view.
- Depth: multisampled depth texture.
- `resolve_target: None`.
- Viewport and scissor are set from Iced `clip_bounds`.

All scene pipelines (planet, stars, satellites, shapes, atmosphere, etc.) render into this MSAA buffer.

### 3) Resolve/composite pass (single-sample final target)

Second pass:

- Target: final Iced surface `target`.
- No depth.
- Viewport and scissor again set from `clip_bounds`.

`ResolveMsaaPipeline` draws one full-screen triangle and runs `resolve_msaa_shader.wgsl`.

The fragment shader reads all 4 MSAA samples for the current pixel with `textureLoad` and writes the average:

- `s0 + s1 + s2 + s3` then multiply by `0.25`.

This is a manual resolve that is clip-aware because the pass is scissored by Iced clip bounds.

## Why Two Passes Instead of resolve_target

Using a direct MSAA `resolve_target` on the first pass can produce writes outside the intended widget clip in this integration path.

The explicit second pass allows strict clipping of final writes to the shader region, preserving surrounding Iced UI rendering.

## Important Constraints

- `MSAA_SAMPLE_COUNT` is currently `4`.
- The resolve shader currently assumes exactly 4 samples.

If you change `MSAA_SAMPLE_COUNT`, update the resolve shader sampling logic accordingly.

## Performance Notes

- This design adds one extra pass and one fullscreen draw call per shader primitive render.
- The resolve pass is scissored to clip bounds, so effective work is constrained to the visible shader area.

## Debug Checklist

If UI colors/background regress again:

1. Confirm the first pass still uses `resolve_target: None`.
2. Confirm the second pass exists and is scissored by `clip_bounds`.
3. Confirm all scene pipelines use the same multisample count as the MSAA textures.
4. Confirm the resolve shader sample count matches `MSAA_SAMPLE_COUNT`.
