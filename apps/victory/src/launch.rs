//! Launch a native Windows game as a separate host process.
//!
//! This path deliberately does not inject overlays or frame-generation
//! backends. AetherX's performance work focuses on measurable native-game
//! optimizations, frame pacing, graphics settings and audio reliability.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Start a native executable with its install directory as the working
/// directory. Arguments are passed directly, never through a shell.
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

    // Among Us ships as a Unity Windows player. Prefer its native D3D12
    // renderer unless the user has explicitly selected another backend.
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

    let child = command
        .current_dir(directory)
        .spawn()
        .with_context(|| format!("starting native PC game {}", exe.display()))?;

    println!("launched       {}", exe.display());
    println!("working dir    {}", directory.display());
    println!("process id     {}", child.id());
    println!("mode           native Windows process; no frame-generation overlay");

    // Keep the process handle alive and report its exit status without
    // blocking the launcher UI.
    std::thread::spawn(move || {
        let mut child = child;
        if let Err(error) = child.wait() {
            eprintln!("game process   wait failed: {error}");
        }
    });

    Ok(())
}
