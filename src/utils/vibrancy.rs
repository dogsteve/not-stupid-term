pub fn apply_vibrancy_to_window<T: raw_window_handle::HasWindowHandle>(window: &T) {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
        let _ = apply_vibrancy(
            window,
            NSVisualEffectMaterial::WindowBackground,
            Some(NSVisualEffectState::Active),
            Some(8.0),
        );
    }
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::apply_blur;
        let _ = apply_blur(window, Some((18, 18, 22, 125)));
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window;
    }
}

pub fn apply_window_vibrancy(cc: &eframe::CreationContext<'_>) {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        use raw_window_handle::HasWindowHandle;
        if let Ok(handle) = cc.window_handle() {
            apply_vibrancy_to_window(&handle);
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = cc;
    }
}
