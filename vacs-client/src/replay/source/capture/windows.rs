use crate::replay::source::capture::{LoopbackCapture, LoopbackEvent};
use crate::replay::{ReplayError, TapId};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use tokio::sync::mpsc;
use wasapi::{AudioClient, Direction, SampleType, StreamMode, WaveFormat};

// TODO: Tap Id should always merged, maybe not if we can get both streams separate
// TODO: We should handle errors, right?

/// Channel capacity for the capture-thread → async forwarder mpsc.
const CHANNEL_CAPACITY: usize = 1024;
const CHUNKSIZE: usize = 1024;

pub struct WindowsApplicationCapture {
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WindowsApplicationCapture {
    fn start_inner() -> Result<(Self, mpsc::Receiver<LoopbackEvent>), ReplayError> {
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let (shutdown, thread) = spawn_wasapi_thread(tx)?;
        Ok((
            Self {
                shutdown,
                thread: Some(thread),
            },
            rx,
        ))
    }

    fn stop_inner(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take()
            && let Err(err) = thread.join()
        {
            log::warn!("Windows loopback audio capture thread panicked: {err:?}");
        }
    }
}

impl LoopbackCapture for WindowsApplicationCapture {
    fn start() -> Result<(Self, mpsc::Receiver<LoopbackEvent>), ReplayError> {
        Self::start_inner()
    }

    fn stop(&mut self) {
        self.stop_inner();
    }
}

impl Drop for WindowsApplicationCapture {
    fn drop(&mut self) {
        self.stop_inner();
    }
}

fn spawn_wasapi_thread(
    tx: mpsc::Sender<LoopbackEvent>,
) -> Result<(Arc<AtomicBool>, JoinHandle<()>), ReplayError> {
    let refreshes = RefreshKind::nothing().with_processes(ProcessRefreshKind::everything());
    let system = System::new_with_specifics(refreshes);
    let process_ids = system.processes_by_name(OsStr::new("trackaudio.exe"));
    let mut process_id = 0;
    for process in process_ids {
        // Note: When capturing audio windows allows you to capture an app's entire process tree,
        // however you must ensure you use the parent as the target process ID
        process_id = process.parent().unwrap_or(process.pid()).as_u32();
    }

    let stop_requested = Arc::new(AtomicBool::new(false));
    let capture_stop_requested = Arc::clone(&stop_requested);

    let thread = std::thread::Builder::new()
        .name("vacs-replay-wasapi".to_owned())
        .spawn(move || run_main_loop(tx, process_id, capture_stop_requested))
        .map_err(ReplayError::Io)?;

    Ok((stop_requested, thread))
}

fn run_main_loop(
    tx: mpsc::Sender<LoopbackEvent>,
    process_id: u32,
    stop_requested: Arc<AtomicBool>,
) {
    let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 48000, 2, None);
    let blockalign = desired_format.get_blockalign();

    let mut audio_client = AudioClient::new_application_loopback_client(process_id, true).unwrap();
    audio_client
        .initialize_client(
            &desired_format,
            &Direction::Capture,
            &StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: 0,
            },
        )
        .unwrap();

    let h_event = audio_client.set_get_eventhandle().unwrap();

    let capture_client = audio_client.get_audiocaptureclient().unwrap();

    // just eat the reallocation because querying the buffer size gives massive values.
    let mut sample_queue: VecDeque<u8> = VecDeque::new();

    if let Err(err) = tx.try_send(LoopbackEvent::Opened {
        tap: TapId::Headset,
        channels: 2,
        sample_rate: 48000,
    }) {
        log::warn!("failed to send opened event: {err}");
    }

    audio_client.start_stream().unwrap();

    while !stop_requested.load(Ordering::Relaxed) {
        while sample_queue.len() > (blockalign as usize * CHUNKSIZE) {
            if stop_requested.load(Ordering::Relaxed) {
                break;
            }
            let mut chunk = vec![0u8; blockalign as usize * CHUNKSIZE];
            for element in chunk.iter_mut() {
                *element = sample_queue.pop_front().unwrap();
            }
            if let Err(err) = tx.try_send(LoopbackEvent::Frame {
                tap: TapId::Headset,
                samples: bytes_to_f32(&chunk).into(),
                captured_at: std::time::Instant::now(),
            }) {
                log::warn!("failed to send frame: {err}");
            };
        }

        let new_frames = capture_client.get_next_packet_size().unwrap().unwrap_or(0);
        let additional = (new_frames as usize * blockalign as usize)
            .saturating_sub(sample_queue.capacity() - sample_queue.len());
        sample_queue.reserve(additional);
        if new_frames > 0 {
            capture_client
                .read_from_device_to_deque(&mut sample_queue)
                .unwrap();
        }
        if h_event.wait_for_event(3000).is_err() {
            break;
        }
    }

    if let Err(err) = tx.try_send(LoopbackEvent::Closed {
        tap: TapId::Headset,
    }) {
        log::warn!("failed to send closed event: {err}");
    }

    audio_client.stop_stream().unwrap();

    log::info!("Windows loopback audio capture thread exiting");
}

fn bytes_to_f32(samples: &[u8]) -> Vec<f32> {
    samples
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
