use iced::wgpu::{self, Buffer, BufferDescriptor};

/// Write `data` into a grow-only vertex buffer, reusing the allocation across
/// frames. The buffer is only re-created when it needs to be larger, so
/// per-frame updates become cheap `write_buffer` calls instead of allocation churn.
///
/// A buffer already present but empty is kept around (size is not shrunk),
/// avoiding re-allocation when data comes back.
pub fn write_or_grow(
    slot: &mut Option<Buffer>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[u8],
    label: &str,
) {
    let needed = data.len() as u64;
    let needs_grow = match slot {
        Some(buffer) => buffer.size() < needed,
        None => true,
    };

    if needs_grow {
        let size = if needed == 0 { 1 } else { needed };
        *slot = Some(device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }

    if !data.is_empty() {
        queue.write_buffer(slot.as_ref().expect("buffer must exist"), 0, data);
    }
}