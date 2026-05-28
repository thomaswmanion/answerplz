/// Linux/GTK can panic if click-through is enabled before the native window is shown.
/// Callers must `.show()` the window first (see `display_preview::create_preview_window`).
#[macro_export]
macro_rules! set_ignore_cursor_events_safe {
    ($window:expr, $ignore:expr) => {{
        let _ = $window.set_ignore_cursor_events($ignore);
    }};
}

/// AppIndicator/libayatana needs a DBus session (often missing in WSL without dbus-x11).
pub fn system_tray_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}
