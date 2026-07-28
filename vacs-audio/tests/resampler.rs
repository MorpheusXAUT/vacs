//! Tests for the streaming `Async` resampler used by the capture and Opus playback paths.
//!
//! `StreamDevice::resampler` needs a real cpal device, so these tests rebuild the same
//! `Async` configuration directly and drive it with the chunked `process_into_buffer`
//! loop that `stream::capture` and `sources::opus` use.

use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{
    Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

const TARGET_SAMPLE_RATE: u32 = 48_000;
const CHUNK_SIZE: usize = 1024;

/// Mirrors `StreamDevice::resampler` for an input (capture) device.
fn capture_resampler(device_rate: u32) -> Async<f32> {
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: None,
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    Async::<f32>::new_sinc(
        TARGET_SAMPLE_RATE as f64 / device_rate as f64,
        2.0,
        &params,
        CHUNK_SIZE,
        1,
        FixedAsync::Input,
    )
    .expect("failed to construct resampler")
}

fn sine(freq: f32, rate: u32, secs: f32) -> Vec<f32> {
    let frames = (rate as f32 * secs) as usize;
    (0..frames)
        .map(|n| (std::f32::consts::TAU * freq * n as f32 / rate as f32).sin())
        .collect()
}

fn dominant_freq(samples: &[f32], rate: u32) -> f32 {
    let skip = samples.len() / 10;
    let body = &samples[skip..samples.len() - skip];
    let crossings = body
        .windows(2)
        .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
        .count();
    crossings as f32 * rate as f32 / body.len() as f32
}

/// Drive `resampler` over `input` using the same buffering strategy as the capture loop:
/// accumulate `input_frames_next()` frames, resample, repeat.
fn drive(resampler: &mut Async<f32>, input: &[f32]) -> Vec<f32> {
    let mut in_buf = vec![Vec::<f32>::with_capacity(CHUNK_SIZE * 2)];
    let mut out_buf = vec![vec![0.0f32; resampler.output_frames_max()]];
    let mut indexing = Indexing {
        input_offset: 0,
        output_offset: 0,
        active_channels_mask: None,
        partial_len: None,
    };

    let mut collected = Vec::new();
    let mut fed = 0usize;

    loop {
        let need = resampler.input_frames_next();
        if fed + need > input.len() {
            break;
        }
        in_buf[0].clear();
        in_buf[0].extend_from_slice(&input[fed..fed + need]);
        fed += need;

        let input_frames = in_buf[0].len();
        let max_out = out_buf[0].len();
        let input_adapter = SequentialSliceOfVecs::new(&in_buf, 1, input_frames).unwrap();
        let mut output_adapter = SequentialSliceOfVecs::new_mut(&mut out_buf, 1, max_out).unwrap();

        indexing.input_offset = 0;
        indexing.output_offset = 0;

        let (_frames_in, frames_out) = resampler
            .process_into_buffer(&input_adapter, &mut output_adapter, Some(&indexing))
            .expect("resampling failed");

        collected.extend_from_slice(&out_buf[0][..frames_out]);
    }

    collected
}

#[test]
fn capture_resampling_preserves_tone_across_device_rates() {
    for device_rate in [44_100, 32_000, 22_050, 88_200, 96_000, 192_000] {
        let mut resampler = capture_resampler(device_rate);
        let input = sine(1_000.0, device_rate, 2.0);
        let out = drive(&mut resampler, &input);

        assert!(
            !out.is_empty(),
            "{device_rate} Hz device produced no output"
        );
        assert!(
            out.iter().all(|s| s.is_finite()),
            "{device_rate} Hz device produced non-finite samples"
        );

        let freq = dominant_freq(&out, TARGET_SAMPLE_RATE);
        assert!(
            (freq - 1_000.0).abs() < 20.0,
            "{device_rate} Hz device shifted the tone to {freq} Hz"
        );
    }
}

#[test]
fn capture_resampling_output_rate_matches_target() {
    // 2 s of audio in should yield ~2 s at 48 kHz out, minus the frames left in the
    // partial chunk the capture loop never submits and the resampler's filter delay.
    for device_rate in [44_100, 96_000] {
        let mut resampler = capture_resampler(device_rate);
        let input = sine(1_000.0, device_rate, 2.0);
        let out = drive(&mut resampler, &input);

        let expected = 2.0 * TARGET_SAMPLE_RATE as f32;
        let slack = 2.0 * CHUNK_SIZE as f32 * TARGET_SAMPLE_RATE as f32 / device_rate as f32;
        assert!(
            (expected - out.len() as f32) < slack && out.len() as f32 <= expected,
            "{device_rate} Hz device produced {} frames, expected ~{expected}",
            out.len()
        );
    }
}

#[test]
fn capture_resampling_is_continuous_across_chunks() {
    // A discontinuity at a chunk boundary shows up as a sample-to-sample jump far
    // larger than a 1 kHz sine at 48 kHz can legitimately make (~0.13 per sample).
    let mut resampler = capture_resampler(44_100);
    let input = sine(1_000.0, 44_100, 2.0);
    let out = drive(&mut resampler, &input);

    let max_step = out
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0f32, f32::max);

    assert!(
        max_step < 0.3,
        "discontinuity across chunk boundary: max sample step {max_step}"
    );
}

#[test]
fn capture_resampling_handles_silence_and_full_scale() {
    let mut resampler = capture_resampler(44_100);

    let silence = vec![0.0f32; 44_100];
    let out = drive(&mut resampler, &silence);
    let peak = out.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(
        peak < 1e-6,
        "silence resampled to non-silence (peak {peak})"
    );

    // Full-scale input must not blow up beyond a small amount of filter overshoot.
    let mut resampler = capture_resampler(44_100);
    let out = drive(&mut resampler, &sine(1_000.0, 44_100, 1.0));
    let peak = out.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(peak < 1.1, "full-scale input overshot to {peak}");
}
