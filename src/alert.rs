use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use notify_rust::Notification;

fn alert_message(
    window: std::num::NonZeroIsize,
    title: &str,
    message: &str,
    notification: bool,
) {
    if notification {
        Notification::new()
            .summary(title)
            .body(message)
            // .icon("thunderbird")
            .appname("Bambu Watcher")
            .timeout(0)
            .show()
            .unwrap();
    }

    let mut l_msg: Vec<u16> = title.encode_utf16().collect();
    let mut l_title: Vec<u16> = message.encode_utf16().collect();

    /// ensure null-terminated
    if l_msg.last() != Some(&0) {
        l_msg.push(0);
    }
    if l_title.last() != Some(&0) {
        l_title.push(0);
    }

    // let handle: isize = window.get()
    let handle = windows::Win32::Foundation::HWND(window.get());

    debug!("displaying alert");
    let result = unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            // None,
            // Some(windows::Win32::Foundation::HWND(window as _)),
            handle,
            windows::core::PCWSTR::from_raw(l_msg.as_ptr()),
            windows::core::PCWSTR::from_raw(l_title.as_ptr()),
            windows::Win32::UI::WindowsAndMessaging::MB_OK
                | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING
                | windows::Win32::UI::WindowsAndMessaging::MB_SETFOREGROUND
                // | windows::Win32::UI::WindowsAndMessaging::MB_APPLMODAL
                | windows::Win32::UI::WindowsAndMessaging::MB_TASKMODAL
                | windows::Win32::UI::WindowsAndMessaging::MB_TOPMOST,
        )
    };
    debug!("displayed alert");

    match result {
        _ => {
            debug!("done");
        }
    }
}
