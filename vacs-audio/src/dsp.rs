pub fn downmix_interleaved_to_mono(interleaved: &[f32], channels: usize, mono: &mut Vec<f32>) {
    debug_assert!(channels > 0);
    debug_assert_eq!(interleaved.len() % channels, 0);

    let frames = interleaved.len() / channels;
    mono.clear();
    mono.reserve(frames);
    for frame in interleaved.chunks(channels) {
        mono.push(downmix_frame_to_mono(frame));
    }
}

#[inline]
fn downmix_frame_to_mono(frame: &[f32]) -> f32 {
    match frame.len() {
        0 => 0.0f32,
        1 => frame[0],
        2 => {
            let (l, r) = (frame[0], frame[1]);
            if (l - r).abs() < 1e-4 {
                l
            } else {
                (l + r) * 0.5f32
            }
        }
        n => frame.iter().take(n).copied().sum::<f32>() / (n as f32),
    }
}
