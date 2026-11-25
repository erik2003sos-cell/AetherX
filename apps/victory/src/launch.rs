//! Launch a native PC game as a separate host process.
//!
//! Frame generation is an AetherX-wide policy: every launched game receives
//! the same request. The rendering backend must still be available in the
//! target process before generated frames can be produced.

use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

/// Start a native executable with its install directory as the working
/// directory. Arguments are passed as individual arguments, never through a
/// shell. 2x frame generation is always requested; there is no per-game
/// opt-out or user-facing toggle.
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
        .env("AETHERX_FRAME_GENERATION", "2x")
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

        let mut process = match Command::new(fg_path)
            .args([
                "--window-pid",
                game_pid_arg.as_str(),
                "--fg-factor",
                "2",
                // The default own-window flip plane yields whenever another
                // window takes foreground, which stopped presenting during
                // the previous Among Us test. Use the DComp overlay fallback
                // so frame presentation does not depend on foreground focus.
                "--no-present-own-window",
            ])
            .spawn()
        {
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
                    println!("frame gen      PhyriadFG initialized (2x, game PID {game_pid})");
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
