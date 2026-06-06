use std::time::Duration;

pub mod opus;
pub mod wav;
pub mod waveform;

pub type AudioSourceId = usize;

/// Represents an audio source that can be mixed into an output stream.
///
/// Types implementing this trait can be fed into the [`crate::mixer::Mixer`] and contribute samples
/// to the final audio buffer on each cpal output device data callback.
pub trait AudioSource: Send {
    /// Add the [`AudioSource`]'s samples to the given output buffer, mixing it in-place.
    ///
    /// This function must **add** its samples onto the current buffer's content as replacing them
    /// would discard any other data already mixed in by other sources.
    /// The output buffer will initially be filled with [`cpal::Sample::EQUILIBRIUM`] to provide a
    /// consistent baseline for mixing and will automatically be clamped to `-1.0;1.0` by the mixer
    /// to avoid clipping.
    ///
    /// Implementations should advance their internal playback state (decode frames, progress waveform
    /// phase, etc.) accordingly as needed. The [`crate::mixer::Mixer`] will call this function for
    /// each registered source on each data callback triggered by the cpal output device.
    fn mix_into(&mut self, output: &mut [f32]);

    /// Begin playback of this source.
    ///
    /// The source should start producing audio samples and mix them into the output buffer whenever
    /// [`AudioSource::mix_into`] is called.
    /// - For continuous sources (like Opus), this may unmute or reset state, if required.
    /// - For one-shot waveforms, this typically starts the playback from the beginning.
    fn start(&mut self);

    /// Stop playback of this source.
    ///
    /// The source should stop producing audio samples, fading out gracefully if possible (e.g., by
    /// employing an envelope release) and reset its internal state as appropriate.
    fn stop(&mut self);

    /// Restart playback of this source.
    ///
    /// The default implementation of this function is simply a convenience wrapper for [`AudioSource::stop`]
    /// followed by [`AudioSource::start`]; however, sources might choose to override this method to
    /// change their reset behavior (e.g., by pausing briefly after playback was stopped before restarting
    /// their output).
    fn restart(&mut self) {
        self.stop();
        self.start();
    }

    /// Adjust the per-source playback volume.
    ///
    /// Volume is a linear gain multiplier (1.0 = unity gain, 0.0 = silent).
    /// Implementations should apply this scaling during their internal mixing in [`AudioSource::mix_in`],
    /// not destructively to their sample data. The volume should not be applied to the rest of the
    /// data already present in the output buffer.
    fn set_volume(&mut self, volume: f32);

    /// Skip forward in playback by the given duration.
    ///
    /// The source should advance its internal playback state by `duration`, as if that amount of
    /// audio had already been played. For finite sources (e.g., a file), seeking past the end
    /// should be handled gracefully (e.g., by stopping playback or clamping to the end).
    /// Implementations that do not support seeking may choose to treat this as a no-op.
    fn skip(&mut self, duration: Duration);

    /// Rewind playback by the given duration.
    ///
    /// The source should move its internal playback state backwards by `duration`. For sources
    /// that do not support backward seeking (e.g., a live stream or a non-seekable decoder),
    /// implementations may treat this as a no-op or clamp to the beginning of the available
    /// playback range.
    fn rewind(&mut self, duration: Duration);
}
