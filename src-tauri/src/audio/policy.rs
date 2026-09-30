#![allow(non_snake_case, dead_code)]
//! Undocumented Windows policy interfaces, isolated here with honest failure modes:
//!
//! * `IPolicyConfig::SetDefaultEndpoint` – change the default playback/recording device
//!   (the same call the Sound control panel uses).
//! * `IAudioPolicyConfigFactory` – per-application default device (the Settings >
//!   Volume mixer "Output device" override). The interface id differs between Windows
//!   builds, so both known ids are tried; if neither activates the feature reports
//!   `Unsupported` instead of pretending.
//!
//! Neither interface is part of the public SDK and Microsoft may change them.

use super::devices::{self, Flow};
use crate::error::{AppError, AppResult};
use std::ffi::c_void;
use windows::core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT, HSTRING, PCWSTR};
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, ERole,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::WinRT::RoGetActivationFactory;

const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    fn GetMixFormat(&self, device: PCWSTR, out: *mut *mut c_void) -> HRESULT;
    fn GetDeviceFormat(&self, device: PCWSTR, default: i32, out: *mut *mut c_void) -> HRESULT;
    fn ResetDeviceFormat(&self, device: PCWSTR) -> HRESULT;
    fn SetDeviceFormat(&self, device: PCWSTR, a: *mut c_void, b: *mut c_void) -> HRESULT;
    fn GetProcessingPeriod(
        &self,
        device: PCWSTR,
        default: i32,
        a: *mut i64,
        b: *mut i64,
    ) -> HRESULT;
    fn SetProcessingPeriod(&self, device: PCWSTR, a: *mut i64) -> HRESULT;
    fn GetShareMode(&self, device: PCWSTR, out: *mut c_void) -> HRESULT;
    fn SetShareMode(&self, device: PCWSTR, m: *mut c_void) -> HRESULT;
    fn GetPropertyValue(
        &self,
        device: PCWSTR,
        fx: i32,
        key: *const c_void,
        out: *mut c_void,
    ) -> HRESULT;
    fn SetPropertyValue(
        &self,
        device: PCWSTR,
        fx: i32,
        key: *const c_void,
        v: *mut c_void,
    ) -> HRESULT;
    fn SetDefaultEndpoint(&self, device: PCWSTR, role: ERole) -> HRESULT;
    fn SetEndpointVisibility(&self, device: PCWSTR, visible: i32) -> HRESULT;
}

/// Make `device_id` the default endpoint for all three roles of its flow.
pub fn set_default_device(device_id: &str) -> AppResult<()> {
    devices::ensure_com();
    // Validate the id refers to an existing endpoint before touching policy.
    devices::open_device(device_id)?;
    let cfg: IPolicyConfig =
        unsafe { CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL) }.map_err(
            |e| AppError::Unsupported(format!("Windows policy interface unavailable: {e}")),
        )?;
    let wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
    for role in [eConsole, eMultimedia, eCommunications] {
        unsafe { cfg.SetDefaultEndpoint(PCWSTR(wide.as_ptr()), role).ok()? };
    }
    Ok(())
}

// ---- per-application routing -------------------------------------------------------

// Methods 0..18 are unrelated volume/chat-context members we never call.
macro_rules! policy_factory {
    ($name:ident, $iid:literal) => {
        #[interface($iid)]
        unsafe trait $name: IUnknown {
            // IInspectable members, declared by hand so the macro only needs IUnknown.
            fn GetIids(&self, count: *mut u32, iids: *mut *mut GUID) -> HRESULT;
            fn GetRuntimeClassName(&self, name: *mut *mut c_void) -> HRESULT;
            fn GetTrustLevel(&self, level: *mut i32) -> HRESULT;
            fn m0(&self) -> HRESULT;
            fn m1(&self) -> HRESULT;
            fn m2(&self) -> HRESULT;
            fn m3(&self) -> HRESULT;
            fn m4(&self) -> HRESULT;
            fn m5(&self) -> HRESULT;
            fn m6(&self) -> HRESULT;
            fn m7(&self) -> HRESULT;
            fn m8(&self) -> HRESULT;
            fn m9(&self) -> HRESULT;
            fn m10(&self) -> HRESULT;
            fn m11(&self) -> HRESULT;
            fn m12(&self) -> HRESULT;
            fn m13(&self) -> HRESULT;
            fn m14(&self) -> HRESULT;
            fn m15(&self) -> HRESULT;
            fn m16(&self) -> HRESULT;
            fn m17(&self) -> HRESULT;
            fn m18(&self) -> HRESULT;
            fn SetPersistedDefaultAudioEndpoint(
                &self,
                pid: u32,
                flow: i32,
                role: i32,
                device: *mut c_void,
            ) -> HRESULT;
            fn GetPersistedDefaultAudioEndpoint(
                &self,
                pid: u32,
                flow: i32,
                role: i32,
                out: *mut *mut c_void,
            ) -> HRESULT;
            fn ClearAllPersistedApplicationDefaultEndpoints(&self) -> HRESULT;
        }
    };
}

policy_factory!(
    IAudioPolicyConfigFactoryNew,
    "2a59116d-6c4f-45e0-a74f-707e3fef9258"
);
policy_factory!(
    IAudioPolicyConfigFactoryOld,
    "ab3d4648-e242-459f-b02f-541c70306324"
);

const CLASS: &str = "Windows.Media.Internal.AudioPolicyConfig";

enum Factory {
    New(IAudioPolicyConfigFactoryNew),
    Old(IAudioPolicyConfigFactoryOld),
}

impl Factory {
    fn open() -> AppResult<Self> {
        devices::ensure_com();
        let class = HSTRING::from(CLASS);
        if let Ok(f) = unsafe { RoGetActivationFactory::<IAudioPolicyConfigFactoryNew>(&class) } {
            return Ok(Factory::New(f));
        }
        if let Ok(f) = unsafe { RoGetActivationFactory::<IAudioPolicyConfigFactoryOld>(&class) } {
            return Ok(Factory::Old(f));
        }
        Err(AppError::Unsupported(
            "per-application output routing is not available on this Windows build".into(),
        ))
    }

    fn set(&self, pid: u32, flow: i32, dev: &HSTRING) -> HRESULT {
        // The HSTRING is borrowed by the callee; we keep ownership.
        let raw: *mut c_void = unsafe { std::mem::transmute_copy(dev) };
        unsafe {
            match self {
                Factory::New(f) => {
                    let a = f.SetPersistedDefaultAudioEndpoint(pid, flow, eConsole.0, raw);
                    if a.is_err() {
                        return a;
                    }
                    f.SetPersistedDefaultAudioEndpoint(pid, flow, eMultimedia.0, raw)
                }
                Factory::Old(f) => {
                    let a = f.SetPersistedDefaultAudioEndpoint(pid, flow, eConsole.0, raw);
                    if a.is_err() {
                        return a;
                    }
                    f.SetPersistedDefaultAudioEndpoint(pid, flow, eMultimedia.0, raw)
                }
            }
        }
    }

    fn get(&self, pid: u32, flow: i32) -> AppResult<String> {
        let mut out: *mut c_void = std::ptr::null_mut();
        let hr = unsafe {
            match self {
                Factory::New(f) => {
                    f.GetPersistedDefaultAudioEndpoint(pid, flow, eConsole.0, &mut out)
                }
                Factory::Old(f) => {
                    f.GetPersistedDefaultAudioEndpoint(pid, flow, eConsole.0, &mut out)
                }
            }
        };
        // E_INVALIDARG is what Windows returns when the process has no override.
        if hr == HRESULT(0x80070057u32 as i32) {
            return Ok(String::new());
        }
        hr.ok()?;
        if out.is_null() {
            return Ok(String::new());
        }
        // Take ownership of the returned HSTRING.
        let h: HSTRING = unsafe { std::mem::transmute(out) };
        Ok(h.to_string())
    }
}

const RENDER_SWD: &str = "\\\\?\\SWD#MMDEVAPI#";
const RENDER_IFACE: &str = "#{e6327cad-dcec-4949-ae8a-991e976a79d2}";
const CAPTURE_IFACE: &str = "#{2eef81be-33fa-4800-9670-1cd474972c3f}";

/// MMDevice id like `{0.0.0.00000000}.{guid}` -> the long SWD form the policy API wants.
fn to_policy_id(mm_id: &str, flow: Flow) -> String {
    let iface = if flow == Flow::Render {
        RENDER_IFACE
    } else {
        CAPTURE_IFACE
    };
    format!("{RENDER_SWD}{mm_id}{iface}")
}

fn from_policy_id(s: &str) -> String {
    let t = s.strip_prefix(RENDER_SWD).unwrap_or(s);
    match t.rfind("#{") {
        Some(i) => t[..i].to_string(),
        None => t.to_string(),
    }
}

fn flow_i32(f: Flow) -> i32 {
    if f == Flow::Render {
        eRender.0
    } else {
        eCapture.0
    }
}

/// Route one process's audio to `device_id` (`None` = follow the system default again).
pub fn set_app_device(pid: u32, flow: Flow, device_id: Option<&str>) -> AppResult<()> {
    if pid == 0 {
        return Err(AppError::Invalid(
            "the system-sounds session cannot be routed".into(),
        ));
    }
    let f = Factory::open()?;
    let value = match device_id {
        Some(id) if !id.is_empty() => {
            devices::open_device(id)?;
            HSTRING::from(to_policy_id(id, flow))
        }
        _ => HSTRING::new(),
    };
    f.set(pid, flow_i32(flow), &value)
        .ok()
        .map_err(|e| AppError::Unsupported(format!("Windows refused the routing change: {e}")))
}

/// Current per-app override (`None` = follows the system default).
pub fn get_app_device(pid: u32, flow: Flow) -> AppResult<Option<String>> {
    let f = Factory::open()?;
    let s = f.get(pid, flow_i32(flow))?;
    Ok(if s.is_empty() {
        None
    } else {
        Some(from_policy_id(&s))
    })
}

/// Whether per-app routing can be activated on this machine.
pub fn routing_supported() -> bool {
    Factory::open().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_id_round_trip() {
        let mm = "{0.0.0.00000000}.{11111111-2222-3333-4444-555555555555}";
        let p = to_policy_id(mm, Flow::Render);
        assert!(p.starts_with("\\\\?\\SWD#MMDEVAPI#"));
        assert!(p.ends_with("e6327cad-dcec-4949-ae8a-991e976a79d2}"));
        assert_eq!(from_policy_id(&p), mm);
        assert_eq!(from_policy_id(mm), mm);
    }

    #[test]
    #[ignore = "needs real Windows audio stack"]
    fn per_app_routing_round_trip_on_this_process() {
        println!("routing supported: {}", routing_supported());
        let pid = std::process::id();
        let before = get_app_device(pid, Flow::Render);
        println!("before: {before:?}");
        let def = devices::default_device_id(Flow::Render, windows::Win32::Media::Audio::eConsole)
            .expect("default");
        set_app_device(pid, Flow::Render, Some(&def)).expect("set");
        let mid = get_app_device(pid, Flow::Render).expect("get");
        println!("after set: {mid:?}");
        assert_eq!(mid.as_deref(), Some(def.as_str()));
        set_app_device(pid, Flow::Render, None).expect("clear");
        assert_eq!(get_app_device(pid, Flow::Render).expect("get"), None);
    }
}
