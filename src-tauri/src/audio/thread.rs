use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::RingBuffer;

use crate::audio::command::AudioCommand;
use crate::audio::engine::AudioEngine;

const RING_BUFFER_CAPACITY: usize = 1024;

// SAFETY: cpal::Stream on Linux/ALSA contains a *mut () PhantomData that marks
// it !Send to prevent accidental cross-thread access to the underlying ALSA handle.
// After AudioThread::new() returns, _stream is only ever held alive (never accessed).
// All mutable interaction goes through the Send+Sync ring buffer producer and an
// AtomicBool.  The Mutex<AudioThread> in AudioState serialises all callers, so
// there is no concurrent access to the stream handle.
unsafe impl Send for AudioThread {}
unsafe impl Sync for AudioThread {}

pub struct AudioThread {
    producer: rtrb::Producer<AudioCommand>,
    _stream: cpal::Stream,
    running: Arc<AtomicBool>,
    position_tick: Arc<AtomicU64>,
}

impl AudioThread {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or("No default output device available")?;

        let supported_config = device.default_output_config()?;
        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels() as usize;

        // Use the device's native config but force f32 sample format.
        // cpal will handle any necessary conversion if the device doesn't natively support f32.
        let stream_config = cpal::StreamConfig {
            channels: supported_config.channels(),
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let (producer, mut consumer) = RingBuffer::<AudioCommand>::new(RING_BUFFER_CAPACITY);

        let running = Arc::new(AtomicBool::new(true));
        let running_cb = Arc::clone(&running);

        // AudioEngine is created here; capture the position atomic before moving into the closure.
        let mut engine = AudioEngine::new(sample_rate);
        let position_tick = engine.position_tick();

        let err_fn = |err| eprintln!("[AudioThread] stream error: {err}");

        let stream = match supported_config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_output_stream::<f32, _, _>(
                    &stream_config,
                    move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                        audio_callback(data, &mut consumer, &mut engine, &running_cb, channels);
                    },
                    err_fn,
                    None,
                )?
            }
            _ => {
                // For non-f32 native formats, request f32 anyway — on Linux/ALSA cpal will
                // resample/convert. We use build_output_stream which handles FromSample<f32>.
                // This path is unusual on desktop but handled for robustness.
                device.build_output_stream::<f32, _, _>(
                    &stream_config,
                    move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                        audio_callback(data, &mut consumer, &mut engine, &running_cb, channels);
                    },
                    err_fn,
                    None,
                )?
            }
        };

        stream.play()?;

        Ok(Self {
            producer,
            _stream: stream,
            running,
            position_tick,
        })
    }

    /// Push a command to the audio thread's ring buffer.
    ///
    /// Returns `true` if the command was enqueued, `false` if the buffer is full.
    /// This method is non-blocking and real-time safe on the caller side.
    pub fn send(&mut self, cmd: AudioCommand) -> bool {
        self.producer.push(cmd).is_ok()
    }

    pub fn position_tick(&self) -> &Arc<AtomicU64> {
        &self.position_tick
    }

    /// Signal the audio callback to stop rendering (output silence instead).
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// The audio callback — runs on the audio thread at real-time priority.
///
/// Drains all pending commands, processes them, then renders a stereo frame.
fn audio_callback(
    data: &mut [f32],
    consumer: &mut rtrb::Consumer<AudioCommand>,
    engine: &mut AudioEngine,
    running: &AtomicBool,
    channels: usize,
) {
    if !running.load(Ordering::Relaxed) {
        // Stopped: fill with silence and return.
        for s in data.iter_mut() {
            *s = 0.0;
        }
        return;
    }

    // Drain all pending commands before rendering this frame.
    while let Ok(cmd) = consumer.pop() {
        engine.process_command(cmd);
    }

    if channels == 2 {
        // Fast path: the cpal buffer is already interleaved stereo f32.
        // AudioEngine::render writes [L0, R0, L1, R1, ...] directly.
        engine.render(data);
    } else {
        // Non-stereo fallback: render into a stereo temp buffer, then distribute.
        // Phase 1 note: this allocates — acceptable since stereo is the desktop norm.
        let frame_count = data.len() / channels;
        let mut stereo = vec![0.0f32; frame_count * 2];
        engine.render(&mut stereo);

        for (frame_idx, frame) in data.chunks_mut(channels).enumerate() {
            let l = stereo[frame_idx * 2];
            let r = stereo[frame_idx * 2 + 1];
            // Distribute L/R across available channels; mono gets (L+R)/2.
            match channels {
                1 => frame[0] = (l + r) * 0.5,
                _ => {
                    // For > 2 channels: fill first two with L/R, silence the rest.
                    frame[0] = l;
                    frame[1] = r;
                    for s in frame[2..].iter_mut() {
                        *s = 0.0;
                    }
                }
            }
        }
    }
}
