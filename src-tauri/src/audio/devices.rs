//! Core Audio (MMDevice) endpoint enumeration and endpoint volume/mute/peak access.
//! All functions are stateless: they initialise COM (MTA) for the calling thread
//! and create short-lived interface pointers, so they can be called from any thread.

use crate::error::{AppError, AppResult};
use serde::Serialize;
use windows::core::PCWSTR;
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{PROPERTYKEY, RPC_E_CHANGED_MODE};
use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation};
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::StructuredStorage::{
    PropVariantClear, PropVariantToStringAlloc, PropVariantToUInt32,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Flow {
    Render,
    Capture,
}

impl Flow {
    fn raw(self) -> EDataFlow {
        match self {
            Flow::Render => eRender,
            Flow::Capture => eCapture,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub flow: Flow,
    pub state: String,
    pub is_default: bool,
    pub is_default_communications: bool,
    pub form_factor: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits: u16,
    pub volume: f32,
    pub muted: bool,
    /// True for devices that look like a virtual audio cable (VB-Cable, VoiceMeeter, ...).
    pub is_virtual: bool,
}

thread_local! {
    static COM_READY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Initialise COM as MTA for the calling thread (idempotent; never uninitialised so
/// interface pointers stay valid for the thread's life).
pub fn ensure_com() {
    COM_READY.with(|c| {
        if !c.get() {
            // RPC_E_CHANGED_MODE means the thread is already STA (e.g. a UI thread); usable anyway.
            let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if hr.is_ok() || hr == RPC_E_CHANGED_MODE {
                c.set(true);
            }
        }
    });
}

pub fn enumerator() -> AppResult<IMMDeviceEnumerator> {
    ensure_com();
    Ok(unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? })
}

fn pwstr_to_string(p: windows::core::PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    let s = unsafe { p.to_string().unwrap_or_default() };
    unsafe { CoTaskMemFree(Some(p.0 as *const _)) };
    s
}

pub fn device_id(dev: &IMMDevice) -> AppResult<String> {
    Ok(pwstr_to_string(unsafe { dev.GetId()? }))
}

fn prop_string(dev: &IMMDevice, key: &PROPERTYKEY) -> Option<String> {
    unsafe {
        let store = dev.OpenPropertyStore(STGM_READ).ok()?;
        let mut pv = store.GetValue(key).ok()?;
        let s = PropVariantToStringAlloc(&pv).ok().map(pwstr_to_string);
        let _ = PropVariantClear(&mut pv);
        s
    }
}

fn prop_u32(dev: &IMMDevice, key: &PROPERTYKEY) -> Option<u32> {
    unsafe {
        let store = dev.OpenPropertyStore(STGM_READ).ok()?;
        let mut pv = store.GetValue(key).ok()?;
        let v = PropVariantToUInt32(&pv).ok();
        let _ = PropVariantClear(&mut pv);
        v
    }
}

pub fn friendly_name(dev: &IMMDevice) -> String {
    prop_string(dev, &PKEY_Device_FriendlyName).unwrap_or_else(|| "Unknown device".into())
}

fn form_factor_name(v: Option<u32>) -> &'static str {
    match v {
        Some(0) => "Network",
        Some(1) => "Speakers",
        Some(2) => "Line level",
        Some(3) => "Headphones",
        Some(4) => "Microphone",
        Some(5) => "Headset",
        Some(6) => "Handset",
        Some(7) => "Digital passthrough",
        Some(8) => "SPDIF",
        Some(9) => "HDMI",
        Some(10) => "Unknown",
        _ => "Unknown",
    }
}

/// Heuristic: does this endpoint name belong to a virtual audio cable / routing driver that can
/// act as a virtual microphone? Headset-vendor and VR virtual devices are deliberately excluded.
pub fn looks_virtual(name: &str) -> bool {
    let n = name.to_lowercase();
    const EXCLUDE: [&str; 5] = [
        "oculus",
        "steam streaming",
        "meta quest",
        "remote audio",
        "virtual surround",
    ];
    if EXCLUDE.iter().any(|k| n.contains(k)) {
        return false;
    }
    [
        "cable",
        "vb-audio",
        "voicemeeter",
        "virtual audio cable",
        "virtual cable",
        "vac ",
        "hi-fi",
        "vdmic",
        "voicemod",
        "obs virtual",
        "virtual mic",
    ]
    .iter()
    .any(|k| n.contains(k))
}

/// Event-driven device-change notifications (add/remove/state/default changes). Holds the
/// enumerator so the registration stays valid; dropping it unregisters the callback.
#[windows::core::implement(IMMNotificationClient)]
struct ChangeClient(std::sync::Arc<std::sync::atomic::AtomicU64>);

impl ChangeClient {
    fn bump(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Release);
    }
}

#[allow(non_snake_case)]
impl IMMNotificationClient_Impl for ChangeClient_Impl {
    fn OnDeviceStateChanged(
        &self,
        _id: &PCWSTR,
        _state: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        self.bump();
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        self.bump();
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        self.bump();
        Ok(())
    }
    fn OnDefaultDeviceChanged(
        &self,
        _flow: EDataFlow,
        _role: ERole,
        _id: &PCWSTR,
    ) -> windows::core::Result<()> {
        self.bump();
        Ok(())
    }
    fn OnPropertyValueChanged(
        &self,
        _id: &PCWSTR,
        _key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

pub struct DeviceWatcher {
    enumerator: IMMDeviceEnumerator,
    client: IMMNotificationClient,
}

impl DeviceWatcher {
    /// `counter` is incremented on every relevant change; poll it from any thread.
    pub fn start(counter: std::sync::Arc<std::sync::atomic::AtomicU64>) -> AppResult<Self> {
        let enumerator = enumerator()?;
        let client: IMMNotificationClient = ChangeClient(counter).into();
        unsafe { enumerator.RegisterEndpointNotificationCallback(&client)? };
        Ok(Self { enumerator, client })
    }
}

impl Drop for DeviceWatcher {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.client);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::looks_virtual;

    #[test]
    fn virtual_cable_heuristic() {
        assert!(looks_virtual("CABLE Output (VB-Audio Virtual Cable)"));
        assert!(looks_virtual(
            "VoiceMeeter Output (VB-Audio VoiceMeeter VAIO)"
        ));
        assert!(looks_virtual("Line 1 (Virtual Audio Cable)"));
        assert!(!looks_virtual("Oculus Virtual Audio Device"));
        assert!(!looks_virtual("Speakers (Steam Streaming Speakers)"));
        assert!(!looks_virtual("Microphone (HyperX Cloud Alpha Wireless)"));
    }
}

fn state_name(s: DEVICE_STATE) -> &'static str {
    match s {
        DEVICE_STATE_ACTIVE => "active",
        DEVICE_STATE_DISABLED => "disabled",
        DEVICE_STATE_NOTPRESENT => "notPresent",
        DEVICE_STATE_UNPLUGGED => "unplugged",
        _ => "unknown",
    }
}

pub fn default_device_id(flow: Flow, role: ERole) -> Option<String> {
    let e = enumerator().ok()?;
    let d = unsafe { e.GetDefaultAudioEndpoint(flow.raw(), role).ok()? };
    device_id(&d).ok()
}

pub fn open_device(id: &str) -> AppResult<IMMDevice> {
    let e = enumerator()?;
    let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { e.GetDevice(PCWSTR(wide.as_ptr())) }
        .map_err(|_| AppError::NotFound(format!("audio device not found: {id}")))
}

pub fn default_device(flow: Flow) -> AppResult<IMMDevice> {
    let e = enumerator()?;
    unsafe { e.GetDefaultAudioEndpoint(flow.raw(), eConsole) }
        .map_err(|_| AppError::NotFound("no default audio device".into()))
}

/// Resolve an optional id (None = system default) to a device.
pub fn resolve(id: Option<&str>, flow: Flow) -> AppResult<IMMDevice> {
    match id {
        Some(i) if !i.is_empty() => open_device(i),
        _ => default_device(flow),
    }
}

pub fn endpoint_volume(dev: &IMMDevice) -> AppResult<IAudioEndpointVolume> {
    Ok(unsafe { dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)? })
}

pub fn mix_format(dev: &IMMDevice) -> Option<(u32, u16, u16)> {
    unsafe {
        let client = dev.Activate::<IAudioClient>(CLSCTX_ALL, None).ok()?;
        let p = client.GetMixFormat().ok()?;
        let f = std::ptr::read_unaligned(p);
        let out = (f.nSamplesPerSec, f.nChannels, f.wBitsPerSample);
        CoTaskMemFree(Some(p as *const _));
        Some(out)
    }
}

fn describe(
    dev: &IMMDevice,
    flow: Flow,
    def: &Option<String>,
    def_comm: &Option<String>,
) -> Option<DeviceInfo> {
    let id = device_id(dev).ok()?;
    let name = friendly_name(dev);
    let state = unsafe { dev.GetState().ok()? };
    let active = state == DEVICE_STATE_ACTIVE;
    let (sample_rate, channels, bits) = if active {
        mix_format(dev).unwrap_or((0, 0, 0))
    } else {
        (0, 0, 0)
    };
    let (volume, muted) = if active {
        match endpoint_volume(dev) {
            Ok(v) => unsafe {
                (
                    v.GetMasterVolumeLevelScalar().unwrap_or(0.0),
                    v.GetMute().map(|b| b.as_bool()).unwrap_or(false),
                )
            },
            Err(_) => (0.0, false),
        }
    } else {
        (0.0, false)
    };
    let ff = prop_u32(dev, &crate::audio::devices::FORM_FACTOR_KEY);
    Some(DeviceInfo {
        is_default: def.as_deref() == Some(id.as_str()),
        is_default_communications: def_comm.as_deref() == Some(id.as_str()),
        is_virtual: looks_virtual(&name),
        id,
        name,
        flow,
        state: state_name(state).into(),
        form_factor: form_factor_name(ff).into(),
        sample_rate,
        channels,
        bits,
        volume,
        muted,
    })
}

pub const FORM_FACTOR_KEY: PROPERTYKEY = PKEY_AudioEndpoint_FormFactor;

/// All endpoints of one flow that are not "not present" (active, disabled, unplugged).
pub fn list_devices(flow: Flow) -> AppResult<Vec<DeviceInfo>> {
    let e = enumerator()?;
    let def = default_device_id(flow, eConsole);
    let def_comm = default_device_id(flow, eCommunications);
    let state_mask =
        DEVICE_STATE(DEVICE_STATE_ACTIVE.0 | DEVICE_STATE_DISABLED.0 | DEVICE_STATE_UNPLUGGED.0);
    let coll = unsafe { e.EnumAudioEndpoints(flow.raw(), state_mask)? };
    let n = unsafe { coll.GetCount()? };
    let mut out = Vec::with_capacity(n as usize);
    for i in 0..n {
        if let Ok(d) = unsafe { coll.Item(i) } {
            if let Some(info) = describe(&d, flow, &def, &def_comm) {
                out.push(info);
            }
        }
    }
    out.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

pub fn set_endpoint_volume(id: &str, scalar: f32) -> AppResult<()> {
    if !scalar.is_finite() {
        return Err(AppError::Invalid("volume must be a finite number".into()));
    }
    let v = endpoint_volume(&open_device(id)?)?;
    unsafe { v.SetMasterVolumeLevelScalar(scalar.clamp(0.0, 1.0), std::ptr::null())? };
    Ok(())
}

pub fn set_endpoint_mute(id: &str, mute: bool) -> AppResult<()> {
    let v = endpoint_volume(&open_device(id)?)?;
    unsafe { v.SetMute(mute, std::ptr::null())? };
    Ok(())
}

pub fn get_endpoint_mute(id: &str) -> AppResult<bool> {
    let v = endpoint_volume(&open_device(id)?)?;
    Ok(unsafe { v.GetMute()?.as_bool() })
}

/// Holds a meter interface for one endpoint so the poller does not re-activate each tick.
pub struct EndpointMeter {
    meter: IAudioMeterInformation,
}

impl EndpointMeter {
    pub fn open(id: &str) -> AppResult<Self> {
        let dev = open_device(id)?;
        Ok(Self {
            meter: unsafe { dev.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None)? },
        })
    }
    pub fn peak(&self) -> f32 {
        unsafe { self.meter.GetPeakValue().unwrap_or(0.0) }
    }
}
