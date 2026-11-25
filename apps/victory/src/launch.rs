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

    let child = Command::new(&exe)
        .args(args)
        .env("AETHERX_FRAME_GENERATION", "2x")
        .current_dir(directory)
        .spawn()
        .with_context(|| format!("starting native PC game {}", exe.display()))?;

    println!("launched       {}", exe.display());
    println!("working dir    {}", directory.display());
    println!("process id     {}", child.id());
    println!("mode           native Windows/host process (no console emulation)");
    println!("frame gen      2x requested (rendering backend integration required)");
    Ok(())
}
