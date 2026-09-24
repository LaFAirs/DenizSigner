// Prevents an extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        use std::env;
        if env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_none() {
            // SAFETY: single-threaded at startup before any threads spawn.
            unsafe {
                env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
            }
        }
    }

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    isideload::init().expect("Failed to initialize");
    denizsigner_lib::run()
}
