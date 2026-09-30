//! Event-driven WASAPI shared-mode capture and render streams.
//!
//! Both streams always run at 48 kHz 32-bit float (mono capture, stereo render); Windows
//! converts from/to the device format (`AUTOCONVERTPCM`). Each stream owns a dedicated thread
//! registered with MMCSS ("Pro Audio"). All COM objects are created on that thread, so nothing
//! non-`Send` crosses thread boundaries, and initialisation errors are reported synchronously
//! to the caller of `start_*`.

use super::devices;
use crate::error::{AppError, AppResult};
use crossbeam_channel::bounded;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Threading::{
    AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW, CreateEventW,
    WaitForSingleObject,
};

pub const SAMPLE_RATE: u32 = 48_000;
const BUFFERFLAGS_SILENT: u32 = 0x2;

#[derive(Clone, Debug, Default)]
pub struct StreamInfo {
    pub device_id: String,
    pub device_name: String,
    /// `IAudioClient::GetStreamLatency` (API-reported stream latency) in milliseconds.
    pub stream_latency_ms: f32,
    pub buffer_frames: u32,
}

fn float_format(channels: u16) -> WAVEFORMATEX {
    let bytes = 4 * channels;
    WAVEFORMATEX {
        wFormatTag: 3, // WAVE_FORMAT_IEEE_FLOAT
        nChannels: channels,
        nSamplesPerSec: SAMPLE_RATE,
        nAvgBytesPerSec: SAMPLE_RATE * bytes as u32,
        nBlockAlign: bytes,
        wBitsPerSample: 32,
        cbSize: 0,
    }
}

struct Mmcss(Option<HANDLE>);

impl Mmcss {
    fn enter() -> Self {
        let mut idx = 0u32;
        Mmcss(unsafe { AvSetMmThreadCharacteristicsW(w!("Pro Audio"), &mut idx).ok() })
    }
}

impl Drop for Mmcss {
    fn drop(&mut self) {
        if let Some(h) = self.0 {
            unsafe {
                let _ = AvRevertMmThreadCharacteristics(h);
            }
        }
    }
}

struct Opened {
    client: IAudioClient,
    event: HANDLE,
    info: StreamInfo,
}

fn open_client(device_id: Option<&str>, flow: devices::Flow, channels: u16) -> AppResult<Opened> {
    devices::ensure_com();
    let dev = devices::resolve(device_id, flow)?;
    let id = devices::device_id(&dev)?;
    let name = devices::friendly_name(&dev);
    let client =
        unsafe { dev.Activate::<IAudioClient>(windows::Win32::System::Com::CLSCTX_ALL, None)? };
    let fmt = float_format(channels);
    let flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK
        | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
        | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
    unsafe {
        client
            .Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 0, 0, &fmt, None)
            .map_err(|e| {
                AppError::Audio(format!(
                    "could not open '{name}' for low-latency streaming: {e}"
                ))
            })?;
    }
    let event = unsafe { CreateEventW(None, false, false, PCWSTR::null())? };
    unsafe { client.SetEventHandle(event)? };
    let buffer_frames = unsafe { client.GetBufferSize()? };
    let latency = unsafe { client.GetStreamLatency().unwrap_or(0) };
    Ok(Opened {
        client,
        event,
        info: StreamInfo {
            device_id: id,
            device_name: name,
            stream_latency_ms: latency as f32 / 10_000.0,
            buffer_frames,
        },
    })
}

fn close_event(h: HANDLE) {
    unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(h);
    }
}

pub struct StreamHandle {
    pub info: StreamInfo,
    join: Option<JoinHandle<()>>,
    /// Set by the stream thread when the device disappears or an API call fails.
    pub error: Arc<Mutex<Option<String>>>,
    pub finished: Arc<AtomicBool>,
}

impl StreamHandle {
    pub fn join(&mut self) {
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        self.join();
    }
}

/// Start capturing mono 48 kHz float. `on_samples` runs on the realtime capture thread:
/// it must not block, allocate or log.
pub fn start_capture(
    device_id: Option<String>,
    stop: Arc<AtomicBool>,
    mut on_samples: impl FnMut(&[f32]) + Send + 'static,
) -> AppResult<StreamHandle> {
    let (tx, rx) = bounded::<AppResult<StreamInfo>>(1);
    let error = Arc::new(Mutex::new(None));
    let finished = Arc::new(AtomicBool::new(false));
    let (err2, fin2) = (error.clone(), finished.clone());
    let join = std::thread::Builder::new()
        .name("auralis-capture".into())
        .spawn(move || {
            let opened = match open_client(device_id.as_deref(), devices::Flow::Capture, 1) {
                Ok(o) => o,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            let Opened {
                client,
                event,
                info,
            } = opened;
            let cap: IAudioCaptureClient = match unsafe { client.GetService() } {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(e.into()));
                    close_event(event);
                    return;
                }
            };
            if let Err(e) = unsafe { client.Start() } {
                let _ = tx.send(Err(e.into()));
                close_event(event);
                return;
            }
            let _ = tx.send(Ok(info));
            let _mm = Mmcss::enter();
            let silence = vec![0.0f32; SAMPLE_RATE as usize];
            'outer: while !stop.load(Ordering::Relaxed) {
                let w = unsafe { WaitForSingleObject(event, 100) };
                if w != WAIT_OBJECT_0 {
                    continue;
                }
                loop {
                    let n = match unsafe { cap.GetNextPacketSize() } {
                        Ok(n) => n,
                        Err(e) => {
                            *err2.lock() = Some(format!("capture device lost: {e}"));
                            break 'outer;
                        }
                    };
                    if n == 0 {
                        break;
                    }
                    let mut data: *mut u8 = std::ptr::null_mut();
                    let (mut frames, mut flags) = (0u32, 0u32);
                    if let Err(e) =
                        unsafe { cap.GetBuffer(&mut data, &mut frames, &mut flags, None, None) }
                    {
                        *err2.lock() = Some(format!("capture device lost: {e}"));
                        break 'outer;
                    }
                    let len = frames as usize;
                    if flags & BUFFERFLAGS_SILENT != 0 || data.is_null() {
                        on_samples(&silence[..len.min(silence.len())]);
                    } else {
                        let s = unsafe { std::slice::from_raw_parts(data as *const f32, len) };
                        on_samples(s);
                    }
                    let _ = unsafe { cap.ReleaseBuffer(frames) };
                }
            }
            unsafe {
                let _ = client.Stop();
            }
            close_event(event);
            fin2.store(true, Ordering::Release);
        })
        .map_err(|e| AppError::Internal(format!("could not spawn capture thread: {e}")))?;
    let info = rx
        .recv()
        .map_err(|_| AppError::Internal("capture thread exited during start".into()))??;
    Ok(StreamHandle {
        info,
        join: Some(join),
        error,
        finished,
    })
}

pub struct RenderExtras {
    /// Frames queued in the device buffer after the most recent write (live measured).
    pub padding_frames: Arc<AtomicU32>,
}

/// Start rendering stereo 48 kHz float. `fill` receives an interleaved stereo slice that it must
/// completely overwrite and returns `false` once the source is exhausted (stream then drains and
/// stops). It runs on the realtime render thread.
pub fn start_render(
    device_id: Option<String>,
    stop: Arc<AtomicBool>,
    mut fill: impl FnMut(&mut [f32]) -> bool + Send + 'static,
) -> AppResult<(StreamHandle, RenderExtras)> {
    let (tx, rx) = bounded::<AppResult<StreamInfo>>(1);
    let error = Arc::new(Mutex::new(None));
    let finished = Arc::new(AtomicBool::new(false));
    let padding_frames = Arc::new(AtomicU32::new(0));
    let (err2, fin2, pad2) = (error.clone(), finished.clone(), padding_frames.clone());
    let join = std::thread::Builder::new()
        .name("auralis-render".into())
        .spawn(move || {
            let opened = match open_client(device_id.as_deref(), devices::Flow::Render, 2) {
                Ok(o) => o,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            let Opened {
                client,
                event,
                info,
            } = opened;
            let render: IAudioRenderClient = match unsafe { client.GetService() } {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(e.into()));
                    close_event(event);
                    return;
                }
            };
            let total = info.buffer_frames;
            // Pre-fill with silence so the device never starts empty.
            if let Ok(p) = unsafe { render.GetBuffer(total) } {
                let _ = p;
                let _ = unsafe { render.ReleaseBuffer(total, BUFFERFLAGS_SILENT) };
            }
            if let Err(e) = unsafe { client.Start() } {
                let _ = tx.send(Err(e.into()));
                close_event(event);
                return;
            }
            let _ = tx.send(Ok(info));
            let _mm = Mmcss::enter();
            let mut draining = false;
            while !stop.load(Ordering::Relaxed) {
                let w = unsafe { WaitForSingleObject(event, 100) };
                if w != WAIT_OBJECT_0 {
                    continue;
                }
                let padding = match unsafe { client.GetCurrentPadding() } {
                    Ok(p) => p,
                    Err(e) => {
                        *err2.lock() = Some(format!("output device lost: {e}"));
                        break;
                    }
                };
                if draining {
                    if padding == 0 {
                        break;
                    }
                    continue;
                }
                let avail = total.saturating_sub(padding);
                if avail == 0 {
                    continue;
                }
                let ptr = match unsafe { render.GetBuffer(avail) } {
                    Ok(p) => p,
                    Err(e) => {
                        *err2.lock() = Some(format!("output device lost: {e}"));
                        break;
                    }
                };
                let slice =
                    unsafe { std::slice::from_raw_parts_mut(ptr as *mut f32, avail as usize * 2) };
                let more = fill(slice);
                let _ = unsafe { render.ReleaseBuffer(avail, 0) };
                pad2.store(padding + avail, Ordering::Relaxed);
                if !more {
                    draining = true;
                }
            }
            unsafe {
                let _ = client.Stop();
            }
            close_event(event);
            fin2.store(true, Ordering::Release);
        })
        .map_err(|e| AppError::Internal(format!("could not spawn render thread: {e}")))?;
    let info = rx
        .recv()
        .map_err(|_| AppError::Internal("render thread exited during start".into()))??;
    Ok((
        StreamHandle {
            info,
            join: Some(join),
            error,
            finished,
        },
        RenderExtras { padding_frames },
    ))
}
