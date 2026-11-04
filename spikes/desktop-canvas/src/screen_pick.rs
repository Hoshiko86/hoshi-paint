//! One-shot native screen sampling. Results are sRGB and opaque: the desktop
//! compositor has already flattened source alpha. No image is stored or uploaded.
use hoshi_controller::Color;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
};

pub type PickResult = Result<Option<Color>, String>;

pub struct ScreenPick {
    pub receiver: Receiver<PickResult>,
    cancelled: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    _sampler: Option<objc2::rc::Retained<objc2_app_kit::NSColorSampler>>,
}

impl Drop for ScreenPick {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Call on the application main thread (AppKit requires this on macOS).
/// Escape cancels in the native picker; dropping the handle cancels where the
/// platform exposes cancellation. All platform errors are returned to the UI.
pub fn start() -> ScreenPick {
    let (sender, receiver) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    #[cfg(target_os = "macos")]
    let sampler = macos::start(sender, cancelled.clone());
    #[cfg(not(target_os = "macos"))]
    {
        let flag = cancelled.clone();
        std::thread::spawn(move || {
            #[cfg(target_os = "linux")]
            let result = linux::pick(flag);
            #[cfg(target_os = "windows")]
            let result = windows::pick(flag);
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            let result = {
                let _ = flag;
                Err("Screen sampling is unavailable on this platform.".into())
            };
            let _ = sender.send(result);
        });
    }
    ScreenPick {
        receiver,
        cancelled,
        #[cfg(target_os = "macos")]
        _sampler: sampler,
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn from_rgb(rgb: (f64, f64, f64)) -> PickResult {
    let (r, g, b) = rgb;
    if [r, g, b]
        .iter()
        .any(|c| !c.is_finite() || !(0.0..=1.0).contains(c))
    {
        return Err("Screen picker returned an invalid color.".into());
    }
    let byte = |c: f64| (c * 255.0).round() as u8;
    Ok(Some(Color::rgba(byte(r), byte(g), byte(b), 255)))
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use futures_lite::{StreamExt, future};
    use std::{
        collections::HashMap,
        time::{Duration, Instant},
    };
    use zbus::{
        MatchRule, MessageStream,
        zvariant::{OwnedObjectPath, OwnedValue, Value},
    };

    pub fn pick(cancelled: Arc<AtomicBool>) -> PickResult {
        future::block_on(run(cancelled)).map_err(|e| format!("Screen picker: {e}. A desktop portal with PickColor support is required (GNOME/KDE)."))
    }

    async fn wait_for_cancel(cancelled: &AtomicBool, deadline: Instant) {
        while !cancelled.load(Ordering::Relaxed) && Instant::now() < deadline {
            async_io::Timer::after(Duration::from_millis(50)).await;
        }
    }

    async fn close_request(connection: &zbus::Connection, path: &str) {
        // A malfunctioning portal must not keep our worker alive on Close.
        future::race(
            async {
                let _ = connection
                    .call_method(
                        Some("org.freedesktop.portal.Desktop"),
                        path,
                        Some("org.freedesktop.portal.Request"),
                        "Close",
                        &(),
                    )
                    .await;
            },
            async {
                async_io::Timer::after(Duration::from_secs(2)).await;
            },
        )
        .await;
    }

    async fn run(cancelled: Arc<AtomicBool>) -> PickResult {
        let connection = zbus::Connection::session()
            .await
            .map_err(|e| e.to_string())?;
        let sender = connection
            .unique_name()
            .ok_or("No desktop bus identity")?
            .as_str()
            .trim_start_matches(':')
            .replace('.', "_");
        let token = format!("hoshi_{}", std::process::id());
        let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
        // Install the signal match before issuing PickColor: a quick portal
        // response must not be lost between the method reply and subscription.
        let rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender("org.freedesktop.portal.Desktop")
            .map_err(|e| e.to_string())?
            .interface("org.freedesktop.portal.Request")
            .map_err(|e| e.to_string())?
            .member("Response")
            .map_err(|e| e.to_string())?
            .path(path.as_str())
            .map_err(|e| e.to_string())?
            .build();
        let mut stream = MessageStream::for_match_rule(rule, &connection, Some(1))
            .await
            .map_err(|e| e.to_string())?;
        let options = HashMap::from([("handle_token", Value::from(token.as_str()))]);
        let deadline = Instant::now() + Duration::from_secs(60);
        let call_body = ("", options);
        let reply = future::race(
            async {
                Some(
                    connection
                        .call_method(
                            Some("org.freedesktop.portal.Desktop"),
                            "/org/freedesktop/portal/desktop",
                            Some("org.freedesktop.portal.Screenshot"),
                            "PickColor",
                            &call_body,
                        )
                        .await,
                )
            },
            async {
                wait_for_cancel(&cancelled, deadline).await;
                None
            },
        )
        .await;
        let Some(reply) = reply else {
            close_request(&connection, &path).await;
            return Ok(None);
        };
        let reply = reply.map_err(|e| e.to_string())?;
        let returned: OwnedObjectPath = reply.body().deserialize().map_err(|e| e.to_string())?;
        if returned.as_str() != path {
            close_request(&connection, returned.as_str()).await;
            return Err("Desktop portal returned an unexpected request identity".into());
        }
        let response = future::race(
            async {
                stream
                    .next()
                    .await
                    .ok_or("Desktop portal disconnected".to_string())?
                    .map_err(|e| e.to_string())
            },
            async {
                wait_for_cancel(&cancelled, deadline).await;
                Err("cancelled".into())
            },
        )
        .await;
        let message = match response {
            Ok(message) => message,
            Err(error) => {
                close_request(&connection, &path).await;
                return if error == "cancelled" {
                    Ok(None)
                } else {
                    Err(error)
                };
            }
        };
        let (response, mut values): (u32, HashMap<String, OwnedValue>) =
            message.body().deserialize().map_err(|e| e.to_string())?;
        match response {
            0 => {
                let value = values
                    .remove("color")
                    .ok_or("Desktop portal omitted color")?;
                let rgb = <(f64, f64, f64)>::try_from(value).map_err(|e| e.to_string())?;
                from_rgb(rgb)
            }
            1 => Ok(None),
            _ => Err("Desktop denied or failed screen sampling".into()),
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSColor, NSColorSampler, NSColorSpace};
    use std::sync::mpsc::Sender;

    pub fn start(
        sender: Sender<PickResult>,
        cancelled: Arc<AtomicBool>,
    ) -> Option<objc2::rc::Retained<NSColorSampler>> {
        if MainThreadMarker::new().is_none() {
            let _ = sender.send(Err(
                "The macOS screen picker must start on the main thread.".into(),
            ));
            return None;
        }
        let callback = block2::RcBlock::new(move |color: *mut NSColor| {
            if cancelled.load(Ordering::Relaxed) {
                return;
            }
            // AppKit owns this nullable object for the duration of the callback.
            let result = unsafe { color.as_ref() }.map_or(Ok(None), |color| {
                color
                    .colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())
                    .ok_or("Unable to convert sampled color to sRGB.".into())
                    .and_then(|color| {
                        from_rgb((
                            color.redComponent().clamp(0.0, 1.0),
                            color.greenComponent().clamp(0.0, 1.0),
                            color.blueComponent().clamp(0.0, 1.0),
                        ))
                    })
            });
            let _ = sender.send(result);
        });
        // Keep the sampler alive in ScreenPick until completion. AppKit copies
        // the completion block and supplies the native loupe and cursor.
        let sampler = NSColorSampler::new();
        unsafe {
            sampler.showSamplerWithSelectionHandler(&callback);
        }
        Some(sampler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn screen_rgb_is_rounded_and_opaque() {
        assert_eq!(
            from_rgb((0.0, 0.5, 1.0)).unwrap(),
            Some(Color::rgba(0, 128, 255, 255))
        );
    }
    #[test]
    fn invalid_screen_components_are_rejected() {
        for rgb in [
            (f64::NAN, 0.0, 0.0),
            (0.0, f64::INFINITY, 0.0),
            (-0.1, 0.0, 0.0),
            (0.0, 1.1, 0.0),
        ] {
            assert!(from_rgb(rgb).is_err());
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::{
        ptr::{null, null_mut},
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::{
            Dwm::DwmFlush,
            Gdi::{BLACK_BRUSH, GetDC, GetPixel, GetStockObject, ReleaseDC},
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON},
            WindowsAndMessaging::*,
        },
    };

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        w: WPARAM,
        l: LPARAM,
    ) -> LRESULT {
        match message {
            WM_LBUTTONUP => {
                unsafe {
                    PostQuitMessage(1);
                }
                0
            }
            WM_KEYDOWN if w == VK_ESCAPE as usize => {
                unsafe {
                    PostQuitMessage(0);
                }
                0
            }
            WM_CLOSE => {
                unsafe {
                    PostQuitMessage(0);
                }
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, message, w, l) },
        }
    }

    pub fn pick(cancelled: Arc<AtomicBool>) -> PickResult {
        // This dedicated worker owns a temporary, input-catching overlay. A
        // selection click never leaks into the application under the pointer.
        // Coordinates remain signed physical virtual-desktop pixels, including
        // monitors left of / above the primary monitor.
        unsafe {
            // This worker's coordinate system must match physical pixels on
            // mixed-DPI monitor layouts. Its context disappears on thread exit.
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let deadline = Instant::now() + Duration::from_secs(60);
            while GetAsyncKeyState(VK_LBUTTON as i32) < 0 {
                if cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
                    return Ok(None);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let name: Vec<u16> = "HoshiScreenPicker\0".encode_utf16().collect();
            let instance = GetModuleHandleW(null());
            // CUR directory stores hotspot separately. A Win32 cursor resource
            // contains that hotspot followed by the image payload.
            let cur = include_bytes!("../icons/eye-dropper.cur");
            let size = u32::from_le_bytes(cur[14..18].try_into().expect("cursor length")) as usize;
            let offset =
                u32::from_le_bytes(cur[18..22].try_into().expect("cursor offset")) as usize;
            let mut resource = Vec::with_capacity(size + 4);
            resource.extend_from_slice(&cur[10..14]);
            resource.extend_from_slice(&cur[offset..offset + size]);
            let cursor = CreateIconFromResourceEx(
                resource.as_ptr(),
                resource.len() as u32,
                0,
                0x00030000,
                32,
                32,
                0,
            );
            if cursor.is_null() {
                return Err("Unable to create screen eyedropper cursor.".into());
            }
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: name.as_ptr(),
                hCursor: cursor,
                hbrBackground: GetStockObject(BLACK_BRUSH) as _,
                ..std::mem::zeroed()
            };
            if RegisterClassW(&class) == 0 {
                DestroyCursor(cursor);
                return Err("Unable to register screen picker window.".into());
            }
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                name.as_ptr(),
                name.as_ptr(),
                WS_POPUP,
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            if hwnd.is_null() {
                UnregisterClassW(name.as_ptr(), instance);
                DestroyCursor(cursor);
                return Err("Unable to open screen picker overlay.".into());
            }
            // Nonzero alpha keeps the overlay hit-testable. It is destroyed
            // before reading the desktop pixel, so its tint is never sampled.
            SetLayeredWindowAttributes(hwnd, 0, 1, LWA_ALPHA);
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            let mut message: MSG = std::mem::zeroed();
            let mut selected = false;
            let mut point: POINT = std::mem::zeroed();
            'events: loop {
                if cancelled.load(Ordering::Relaxed)
                    || Instant::now() >= deadline
                    || GetAsyncKeyState(VK_ESCAPE as i32) < 0
                {
                    break;
                }
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if message.message == WM_QUIT {
                        selected = message.wParam == 1;
                        break 'events;
                    }
                    if message.message == WM_LBUTTONUP {
                        point = message.pt;
                    }
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                std::thread::sleep(Duration::from_millis(8));
            }
            DestroyWindow(hwnd);
            UnregisterClassW(name.as_ptr(), instance);
            DestroyCursor(cursor);
            if !selected {
                return Ok(None);
            }
            // Flush desktop composition after destroying our overlay so its
            // translucent hit-test surface cannot contaminate the chosen RGB.
            DwmFlush();
            let dc = GetDC(null_mut());
            if dc.is_null() {
                return Err("Unable to read desktop pixels.".into());
            }
            let pixel = GetPixel(dc, point.x, point.y);
            ReleaseDC(null_mut(), dc);
            if pixel == u32::MAX {
                return Err("The selected screen pixel is unavailable.".into());
            }
            Ok(Some(Color::rgba(
                pixel as u8,
                (pixel >> 8) as u8,
                (pixel >> 16) as u8,
                255,
            )))
        }
    }
}
