//! Launch a native PC game as a separate host process.
//!
//! This is deliberately separate from `run`: it does not emulate a console
//! or translate a title. It asks the host operating system to start the
//! executable using Rust's process API.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Start a native executable with its install directory as the working
/// directory. Arguments are passed as individual arguments, never through a
/// shell.
pub fn run(exe: &Path, args: &[String]) -> Result<()> {
    run_with_frame_generation(exe, args, true)
}

/// Start a native executable and pass AetherX's frame-generation preference
/// to it. This is only a capability/configuration hand-off: a game needs a
/// compatible capture/presentation backend before any generated frames exist.
pub fn run_with_frame_generation(
    exe: &Path,
    args: &[String],
    frame_generation_2x: bool,
) -> Result<()> {
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

    let child = Command::new(&exe)
        .args(args)
        .env(
            "AETHERX_FRAME_GENERATION",
            if frame_generation_2x { "2x" } else { "off" },
        )
        .current_dir(directory)
        .spawn()
        .with_context(|| format!("starting native PC game {}", exe.display()))?;

    println!("launched       {}", exe.display());
    println!("working dir    {}", directory.display());
    println!("process id     {}", child.id());
    println!("mode           native Windows/host process (no console emulation)");
    println!("frame gen      {}", if frame_generation_2x { "2x requested (backend required)" } else { "off" });
    Ok(())
}
