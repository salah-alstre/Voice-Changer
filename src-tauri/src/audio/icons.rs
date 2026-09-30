//! Application icon extraction: exe path -> 32x32 PNG as a base64 data URL (cached).

use base64::Engine;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::mem::size_of;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

static CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

/// Returns a `data:image/png;base64,...` URL, or `None` if the executable has no extractable icon.
pub fn icon_data_url(exe_path: &str) -> Option<String> {
    if exe_path.is_empty() {
        return None;
    }
    {
        let g = CACHE.lock();
        if let Some(v) = g.as_ref().and_then(|m| m.get(exe_path)) {
            return v.clone();
        }
    }
    let result = extract(exe_path).map(|png| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    });
    CACHE
        .lock()
        .get_or_insert_with(HashMap::new)
        .insert(exe_path.to_string(), result.clone());
    result
}

fn extract(path: &str) -> Option<Vec<u8>> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut info = SHFILEINFOW::default();
        let r = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if r == 0 || info.hIcon.is_invalid() {
            return None;
        }
        let hicon = info.hIcon;
        let mut ii = ICONINFO::default();
        if GetIconInfo(hicon, &mut ii).is_err() {
            let _ = DestroyIcon(hicon);
            return None;
        }
        let out = (|| {
            let mut bm = BITMAP::default();
            if GetObjectW(
                ii.hbmColor.into(),
                size_of::<BITMAP>() as i32,
                Some(&mut bm as *mut _ as *mut _),
            ) == 0
            {
                return None;
            }
            let (w, h) = (bm.bmWidth, bm.bmHeight);
            if w <= 0 || h <= 0 || w > 256 || h > 256 {
                return None;
            }
            let mut bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut px = vec![0u8; (w * h * 4) as usize];
            let dc = GetDC(Some(HWND::default()));
            let lines = GetDIBits(
                dc,
                ii.hbmColor,
                0,
                h as u32,
                Some(px.as_mut_ptr() as *mut _),
                &mut bi,
                DIB_RGB_COLORS,
            );
            ReleaseDC(Some(HWND::default()), dc);
            if lines == 0 {
                return None;
            }
            // BGRA -> RGBA; icons without an alpha channel come back all-zero alpha, so make them opaque.
            let has_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
            for p in px.chunks_exact_mut(4) {
                p.swap(0, 2);
                if !has_alpha {
                    p[3] = 255;
                }
            }
            let mut buf = Vec::new();
            {
                let mut enc = png::Encoder::new(&mut buf, w as u32, h as u32);
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                let mut wr = enc.write_header().ok()?;
                wr.write_image_data(&px).ok()?;
            }
            Some(buf)
        })();
        let _ = DeleteObject(ii.hbmColor.into());
        let _ = DeleteObject(ii.hbmMask.into());
        let _ = DestroyIcon(hicon);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_path_has_no_icon() {
        assert!(icon_data_url("").is_none());
    }

    #[test]
    #[ignore = "needs a real Windows shell"]
    fn extracts_icon_for_explorer() {
        let url = icon_data_url("C:\\Windows\\explorer.exe").expect("icon");
        println!("{} bytes", url.len());
        assert!(url.starts_with("data:image/png;base64,"));
    }
}
