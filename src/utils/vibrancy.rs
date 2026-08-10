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
}

pub fn apply_window_vibrancy(cc: &eframe::CreationContext<'_>) {
    #[cfg(target_os = "macos")]
    {
        use raw_window_handle::HasWindowHandle;
        if let Ok(handle) = cc.window_handle() {
            apply_vibrancy_to_window(&handle);
        }
    }
}
