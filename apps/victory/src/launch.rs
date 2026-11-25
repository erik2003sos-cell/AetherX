//! Launch a native PC game as a separate host process.
//!
//! Frame generation is an AetherX-wide policy: every launched game receives
//! the same request. The rendering backend must still be available in the
//! target process before generated frames can be produced.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

/// Start a native executable with its install directory as the working
/// directory. Arguments are passed as individual arguments, never through a
/// shell. The backend renders at up to 2x internally, while its fractional
/// output controller targets 45 FPS (about 1.5x for a 30 FPS source).
pub fn run(exe: &Path, args: &[String]) -> Result<()> {
    anyhow::ensure!(
        exe.is_file(),
        "game executable does not exist or is not a file: {}",
        exe.display()
    );

    let exe = exe
        .canonicalize()
        .with_context(|| format!("resolving game executable {}", exe.display()))?;
    let directory = exe
        .parent()
        .context("game executable has no parent directory")?;

    // Among Us ships as a Unity Windows player. Force its supported D3D12
    // backend from AetherX itself so the future DX12 frame-generation path
    // can attach without requiring Steam launch options or account access.
    let is_among_us = exe
        .file_stem()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("Among Us"));
    let mut command = Command::new(&exe);
    command.args(args);
    if is_among_us
        && !args.iter().any(|arg| {
            matches!(
                arg.to_ascii_lowercase().as_str(),
                "-force-d3d11" | "-force-d3d12" | "-force-vulkan"
            )
        })
    {
        command.arg("-force-d3d12");
    }

    let mut child = command
        .env("AETHERX_FRAME_GENERATION", "1.5x-target-45fps")
        .current_dir(directory)
        .spawn()
        .with_context(|| format!("starting native PC game {}", exe.display()))?;

    let game_pid = child.id();
    println!("launched       {}", exe.display());
    println!("working dir    {}", directory.display());
    println!("process id     {game_pid}");
    println!("mode           native Windows/host process (no console emulation)");

    // PhyriadFG resolves --window-pid immediately and exits if the game's
    // window does not exist yet. Unity games can take several seconds to
    // create that window, so retry short-lived startup failures instead of
    // racing the game's startup. The process is accepted once it survives
    // its initialization window.
    let fg_path = std::env::current_exe()?
        .parent()
        .context("AetherX executable has no parent directory")?
        .join("phyriad_fg.exe");
    let fg_available = fg_path.is_file();
    if !fg_available {
        eprintln!("frame gen      backend not installed; run tools\\setup-phyriadfg.ps1");
    }

    // Keep the game and capture/presenter lifecycle together without blocking
    // the launcher UI while the game creates its first visible window.
    std::thread::spawn(move || {
        let mut framegen = if fg_available {
            start_framegen_when_window_exists(&fg_path, game_pid, &mut child)
        } else {
            None
        };

        let _ = child.wait();
        if let Some(process) = framegen.as_mut() {
            let _ = process.kill();
            let _ = process.wait();
        }
    });
    Ok(())
}

/// Start PhyriadFG once the game has a capturable window. PhyriadFG reports a
/// missing window by exiting during startup; retry that case for up to 30 s.
/// A backend that remains alive for 3 s is considered initialized.
fn start_framegen_when_window_exists(
    fg_path: &Path,
    game_pid: u32,
    game: &mut Child,
) -> Option<Child> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let game_pid_arg = game_pid.to_string();

    while Instant::now() < deadline {
        if game.try_wait().ok().flatten().is_some() {
            return None;
        }

        let mut command = Command::new(fg_path);
        // Keep PhyriadFG's default own-window presenter. It yields its
        // presentation plane when another app (including the desktop) takes
        // focus, preventing the last game frame from remaining over the
        // desktop during Alt-Tab. The DComp fallback stays visible over other
        // windows on this setup, so do not pass --no-present-own-window.
        command.args([
            "--window-pid",
            game_pid_arg.as_str(),
            "--fg-factor",
            "2",
            // Fractional output controller: 45 FPS is ~1.5x for a 30 FPS source.
            // --no-decimate selects its fractional content quantizer instead of
            // snapping to an exact divisor of the monitor refresh rate.
            "--target-output-fps",
            "45",
            "--no-decimate",
        ]);
        if let Some(log) = framegen_log_file(game_pid) {
            if let Ok(stderr) = log.try_clone() {
                command.stdout(Stdio::from(log)).stderr(Stdio::from(stderr));
            }
        }
        let mut process = match command.spawn() {
            Ok(process) => process,
            Err(error) => {
                eprintln!("frame gen      could not start PhyriadFG: {error}");
                return None;
            }
        };

        let startup_deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if game.try_wait().ok().flatten().is_some() {
                let _ = process.kill();
                let _ = process.wait();
                return None;
            }

            match process.try_wait() {
                Ok(Some(status)) => {
                    eprintln!(
                        "frame gen      PhyriadFG startup exited ({status}); retrying while game window appears"
                    );
                    break;
                }
                Ok(None) if Instant::now() >= startup_deadline => {
                    println!("frame gen      PhyriadFG initialized (45 FPS target, ~1.5x at 30 source FPS, game PID {game_pid})");
                    return Some(process);
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(200)),
                Err(error) => {
                    eprintln!("frame gen      could not monitor PhyriadFG: {error}");
                    let _ = process.kill();
                    let _ = process.wait();
                    return None;
                }
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    eprintln!("frame gen      no capturable game window appeared within 30 s; game continues without frame generation");
    None
}


/// Store backend diagnostics where the user can retrieve them even though
/// the AetherX GUI itself has no console window.
fn framegen_log_file(game_pid: u32) -> Option<std::fs::File> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let directory = base.join("AetherX").join("logs");
    std::fs::create_dir_all(&directory).ok()?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join(format!("phyriadfg-{game_pid}.log")))
        .ok()
}
