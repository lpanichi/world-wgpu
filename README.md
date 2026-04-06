# world-wgpu

This workspace contains a Rust + WGPU + Iced demo project for drawing a textured Earth, stars, orbit trajectory, and an animated satellite.

## Disclaimer

All work in this branch has been authored by the AI assistant (GitHub Copilot) iteration on March 29, 2026.
It includes:
- A textured Earth pipeline
- Starfield background pipeline
- Circular orbit trajectory pipeline
- Depth buffer support (`Depth24Plus`) for correct occlusion
- Camera controls (keyboard arrows + mouse scroll dolly)
- Animated satellite as a small cube moving along the orbital path

## Version

- `iced` version: `0.14.0` (workspace local dependency)
- `wgpu` within `iced_wgpu` from the `iced` 0.14 branch
- Project crate version: `0.1.0`

## Usage

Run example:

```bash
cargo run --example simulation
```

This project primarily works with the `smol` executor for timing subscriptions.

## Documentation

- MSAA implementation details: [docs/msaa.md](docs/msaa.md)
