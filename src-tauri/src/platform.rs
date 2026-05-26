/// Linux/GTK panics if click-through is enabled before the native window exists (tao unwrap).
#[macro_export]
macro_rules! set_ignore_cursor_events_safe {
    ($window:expr, $ignore:expr) => {{
        #[cfg(target_os = "linux")]
        {
            if $ignore {
                ();
            } else {
                let _ = $window.set_ignore_cursor_events(false);
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = $window.set_ignore_cursor_events($ignore);
        }
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
