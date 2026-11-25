//! AMD FidelityFX Frame Generation SDK runtime discovery.
//!
//! AMD ships the DX12 frame-generation provider as one signed DLL. The SDK's
//! ffx_api_loader.h resolves the five API entry points from that module.
//! Finding those entry points is only runtime availability; a game swapchain
//! still needs to be wrapped and supplied with an interpolation dispatch.

#[cfg(windows)]
mod windows_runtime {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use windows::Win32::System::LibraryLoader::{FreeLibrary, GetProcAddress, LoadLibraryW};
    use windows::core::{PCSTR, PCWSTR};

    const PROVIDER_DLL: &str = "amd_fidelityfx_framegeneration_dx12.dll";
    const REQUIRED_EXPORTS: [&[u8]; 5] = [
        b"ffxCreateContext\0",
        b"ffxDestroyContext\0",
        b"ffxConfigure\0",
        b"ffxQuery\0",
        b"ffxDispatch\0",
    ];

    struct Module(windows::Win32::Foundation::HMODULE);

    // The module handle is immutable after LoadLibraryW; the cached runtime
    // is process-wide and the SDK entry points are safe to resolve concurrently.
    unsafe impl Send for Module {}
    unsafe impl Sync for Module {}

    impl Drop for Module {
        fn drop(&mut self) {
            // SAFETY: this handle was returned by LoadLibraryW and is owned here.
            let _ = unsafe { FreeLibrary(self.0) };
        }
    }

    pub struct FsrRuntime {
        _provider: Module,
        pub directory: PathBuf,
    }

    impl FsrRuntime {
        pub fn load() -> Result<Self, String> {
            let exe = std::env::current_exe()
                .map_err(|e| format!("cannot locate AetherX executable: {e}"))?;
            let directory = exe.parent()
                .ok_or_else(|| "AetherX executable has no parent directory".to_owned())?
                .to_path_buf();
            let provider_path = directory.join(PROVIDER_DLL);
            if !provider_path.is_file() {
                return Err(format!("missing {PROVIDER_DLL} beside AetherX"));
            }

            let provider = load_absolute(&provider_path)?;
            for export in REQUIRED_EXPORTS {
                // SAFETY: export names are NUL-terminated static byte strings;
                // the loaded provider remains alive in FsrRuntime.
                let found = unsafe { GetProcAddress(provider.0, PCSTR(export.as_ptr())) };
                if found.is_none() {
                    return Err(format!(
                        "FSR provider is missing export {}",
                        String::from_utf8_lossy(&export[..export.len() - 1])
                    ));
                }
            }
            Ok(Self { _provider: provider, directory })
        }
    }

    fn load_absolute(path: &Path) -> Result<Module, String> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: wide is NUL-terminated and remains alive for this call.
        unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) }
            .map(Module)
            .map_err(|e| format!("could not load {}: {e}", path.display()))
    }
}

#[cfg(windows)]
pub fn status() -> &'static str {
    use std::sync::OnceLock;
    static RUNTIME: OnceLock<Result<windows_runtime::FsrRuntime, String>> = OnceLock::new();
    match RUNTIME.get_or_init(windows_runtime::FsrRuntime::load) {
        Ok(runtime) => {
            let _ = &runtime.directory;
            "FSR DX12 provider detected — swapchain/dispatch integration pending"
        }
        Err(reason) => {
            let _ = reason;
            "FSR DX12 provider not installed — safe fallback"
        }
    }
}

#[cfg(not(windows))]
pub fn status() -> &'static str {
    "FSR Frame Generation requires the Windows DirectX 12 backend"
}
