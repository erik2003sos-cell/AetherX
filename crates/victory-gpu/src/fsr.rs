//! AMD FidelityFX Frame Generation SDK runtime discovery.
//!
//! SDK availability is distinct from activation. FSR FG still requires a
//! D3D12 swapchain integration and per-frame rendering inputs.

#[cfg(windows)]
mod windows_runtime {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use windows::Win32::System::LibraryLoader::{FreeLibrary, GetProcAddress, LoadLibraryW};
    use windows::core::{PCSTR, PCWSTR};

    const LOADER_DLL: &str = "amd_fidelityfx_loader_dx12.dll";
    const FRAMEGEN_DLL: &str = "amd_fidelityfx_framegeneration_dx12.dll";
    const REQUIRED_EXPORTS: [&[u8]; 5] = [
        b"ffxCreateContext\0",
        b"ffxDestroyContext\0",
        b"ffxConfigure\0",
        b"ffxQuery\0",
        b"ffxDispatch\0",
    ];

    struct Module(windows::Win32::Foundation::HMODULE);

    impl Drop for Module {
        fn drop(&mut self) {
            // SAFETY: this handle was returned by LoadLibraryW and is owned here.
            let _ = unsafe { FreeLibrary(self.0) };
        }
    }

    pub struct FsrRuntime {
        _loader: Module,
        _framegen: Module,
        pub directory: PathBuf,
    }

    impl FsrRuntime {
        pub fn load() -> Result<Self, String> {
            let exe = std::env::current_exe()
                .map_err(|e| format!("cannot locate AetherX executable: {e}"))?;
            let directory = exe.parent()
                .ok_or_else(|| "AetherX executable has no parent directory".to_owned())?
                .to_path_buf();
            let loader_path = directory.join(LOADER_DLL);
            let framegen_path = directory.join(FRAMEGEN_DLL);

            if !loader_path.is_file() {
                return Err(format!("missing {LOADER_DLL} beside AetherX"));
            }
            if !framegen_path.is_file() {
                return Err(format!("missing {FRAMEGEN_DLL} beside AetherX"));
            }

            let loader = load_absolute(&loader_path)?;
            let framegen = load_absolute(&framegen_path)?;
            for export in REQUIRED_EXPORTS {
                // SAFETY: export names are NUL-terminated static byte strings;
                // the loaded module remains alive in the runtime.
                let found = unsafe { GetProcAddress(loader.0, PCSTR(export.as_ptr())) };
                if found.is_none() {
                    return Err(format!(
                        "FSR loader is missing export {}",
                        String::from_utf8_lossy(&export[..export.len() - 1])
                    ));
                }
            }

            Ok(Self { _loader: loader, _framegen: framegen, directory })
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
pub fn availability() -> Result<String, String> {
    windows_runtime::FsrRuntime::load()
        .map(|runtime| format!("SDK DLLs and API exports found in {}", runtime.directory.display()))
}

#[cfg(not(windows))]
pub fn availability() -> Result<String, String> {
    Err("FSR Frame Generation requires the Windows DirectX 12 backend".to_owned())
}
