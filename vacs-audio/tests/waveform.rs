use std::time::Duration;
use vacs_audio::sources::AudioSource;
use vacs_audio::sources::waveform::{
    Waveform, WaveformSegment, WaveformSequence, WaveformSource, WaveformTone,
};

#[test]
fn test_waveform_sequence_transitions() {
    let sample_rate = 100.0; // Low sample rate for easier reasoning
    let tone1_dur = Duration::from_millis(20); // 2 samples at 100Hz
    let tone2_dur = Duration::from_millis(30); // 3 samples at 100Hz

    // Tone 1: 25Hz Square wave (period 4 samples) -> 1.0, 1.0, -1.0, -1.0
    let tone1 = WaveformTone::new(25.0, Waveform::Square, 1.0);

    // Tone 2: 50Hz Square wave (period 2 samples) -> 1.0, -1.0
    let tone2 = WaveformTone::new(50.0, Waveform::Square, 0.5);

    let sequence = WaveformSequence::from(vec![(tone1, tone1_dur), (tone2, tone2_dur)]);

    let mut source = WaveformSource::new(
        sequence,
        None,
        Duration::from_millis(0), // No fade
        sample_rate,
        1,
        1.0,
    );

    source.start();

    // Total duration: 20ms + 30ms = 50ms = 5 samples
    // 0-20ms (0-2 samples): Tone 1
    // 20-50ms (2-5 samples): Tone 2

    let mut output = vec![0.0; 5];
    source.mix_into(&mut output);

    // Expected Output:
    // Sample 0 (0ms): Tone 1 start. Phase 0. -> 1.0
    // Sample 1 (10ms): Tone 1 continue. Phase 0.25 (25Hz * 0.01s = 0.25). -> 1.0
    // Sample 2 (20ms): Tone 2 start. Phase 0. -> 0.5
    // Sample 3 (30ms): Tone 2 continue. Phase 0.5 (50Hz * 0.01s = 0.5). -> -0.5 (Square wave at 0.5 is -1.0?) Wait.
    // Square wave: if phase < 0.5 { 1.0 } else { -1.0 }

    let expected = [1.0, 1.0, 0.5, -0.5, 0.5];

    // Allow small float error
    for (i, (a, b)) in output.iter().zip(expected.iter()).enumerate() {
        assert!(
            (a - b).abs() < 0.001,
            "Sample {}: expected {}, got {}",
            i,
            b,
            a
        );
    }
}

#[test]
fn test_waveform_pause() {
    let sample_rate = 100.0;

    // Tone 1: 50Hz Square (period 2 samples) -> 1.0, -1.0
    let tone1 = WaveformTone::new(50.0, Waveform::Square, 1.0);
    // Tone 2: Same
    let tone2 = WaveformTone::new(50.0, Waveform::Square, 1.0);

    let sequence = WaveformSequence::new(vec![
        WaveformSegment::new(tone1, Duration::from_millis(20)), // 2 samples
        WaveformSegment::pause(Duration::from_millis(20)),      // 2 samples silence
        WaveformSegment::new(tone2, Duration::from_millis(20)), // 2 samples
    ]);

    let mut source = WaveformSource::new(
        sequence,
        None,
        Duration::from_millis(0),
        sample_rate,
        1,
        1.0,
    );

    source.start();

    // Total: 6 samples
    let mut output = vec![0.0; 6];
    source.mix_into(&mut output);

    // Expected:
    // S0, S1: Tone 1 (1.0, -1.0)
    // S2, S3: Silence (0.0, 0.0)
    // S4, S5: Tone 2 (start phase 0 -> 1.0, phase 0.5 -> -1.0)

    let expected = [1.0, -1.0, 0.0, 0.0, 1.0, -1.0];

    for (i, (a, b)) in output.iter().zip(expected.iter()).enumerate() {
        assert!(
            (a - b).abs() < 0.001,
            "Sample {}: expected {}, got {}",
            i,
            b,
            a
        );
    }
}

#[test]
fn test_waveform_smooth_transitions() {
    let sample_rate = 100.0;
    // 1 sample fade (10ms at 100Hz)
    let fade_dur = Duration::from_millis(10);

    // Tone 1: 50Hz Square -> 1.0, -1.0. Duration 40ms (4 samples)
    let tone1 = WaveformTone::new(50.0, Waveform::Square, 1.0);

    // Tone 2: 50Hz Square -> 1.0, -1.0. Duration 40ms (4 samples)
    let tone2 = WaveformTone::new(50.0, Waveform::Square, 1.0);

    let sequence = WaveformSequence::new(vec![
        WaveformSegment::new(tone1, Duration::from_millis(40)),
        WaveformSegment::new(tone2, Duration::from_millis(40)),
    ]);

    let mut source = WaveformSource::new(
        sequence,
        None,
        fade_dur, // 10ms fade -> 1 sample attack/release
        sample_rate,
        1,
        1.0,
    );

    source.start();

    // Total: 8 samples
    let mut output = vec![0.0; 8];
    source.mix_into(&mut output);

    // Verify smoothing at start of first segment (S0) and start of second segment (S4)

    // S0: Attack logic: (elapsed / attack) -> (0 / 1) -> 0.0
    assert!(
        output[0].abs() < 0.001,
        "Expected silence/fade at start, got {}",
        output[0]
    );

    // S4: Start of second segment.
    // Elapsed resets to 0.
    // Attack logic: (0 / 1) -> 0.0
    assert!(
        output[4].abs() < 0.001,
        "Expected silence/fade at segment boundary (sample 4), got {}",
        output[4]
    );

    // Verify full amplitude in middle (e.g., S1, S2, S5, S6)
    // S1: elapsed=1. 1/1 -> 1.0.
    assert!(
        output[1].abs() > 0.9,
        "Expected full amplitude after fade-in, got {}",
        output[1]
    );
}

#[test]
fn test_waveform_repeat() {
    let tone = WaveformTone::new(100.0, Waveform::Square, 1.0);
    let dur = Duration::from_millis(10);

    let seq = WaveformSequence::single(tone, dur).repeat(3);

    assert_eq!(seq.segments.len(), 3);

    // Verify content
    for seg in seq.segments {
        match seg {
            WaveformSegment::Tone(t, d) => {
                assert_eq!(t.freq, 100.0);
                assert_eq!(d, dur);
            }
            _ => panic!("Expected tone segment"),
        }
    }
}
