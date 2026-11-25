//! Launch a native PC game as a separate host process.
//!
//! Frame generation is an AetherX-wide policy: every launched game receives
//! the same request. The rendering backend must still be available in the
//! target process before generated frames can be produced.

use std::path::Path;
use std::process::Command;

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

    // PhyriadFG captures the game's final window from outside the process,
    // so it does not need game-engine hooks, Steam, or account access.
    // Install tools/setup-phyriadfg.ps1 places the pinned backend beside AetherX.
    let fg_path = std::env::current_exe()?
        .parent()
        .context("AetherX executable has no parent directory")?
        .join("phyriad_fg.exe");
    let mut framegen = if fg_path.is_file() {
        match Command::new(&fg_path)
            .args([
                "--window-pid",
                &game_pid.to_string(),
                "--fg-factor",
                "2",
                "--force-single-gpu",
            ])
            .spawn()
        {
            Ok(process) => {
                println!("frame gen      PhyriadFG started (2x, PID {game_pid})");
                Some(process)
            }
            Err(error) => {
                eprintln!("frame gen      could not start PhyriadFG: {error}");
                None
            }
        }
    } else {
        eprintln!(
            "frame gen      backend not installed; run tools\\setup-phyriadfg.ps1"
        );
        None
    };

    // Keep the capture/presenter alive exactly as long as the game. Closing
    // the game also terminates the external frame-generation process.
    std::thread::spawn(move || {
        let _ = child.wait();
        if let Some(process) = framegen.as_mut() {
            let _ = process.kill();
            let _ = process.wait();
        }
    });
    Ok(())
}
