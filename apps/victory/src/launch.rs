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
        .current_dir(directory)
        .spawn()
        .with_context(|| format!("starting native PC game {}", exe.display()))?;

    println!("launched       {}", exe.display());
    println!("working dir    {}", directory.display());
    println!("process id     {}", child.id());
    println!("mode           native Windows/host process (no console emulation)");
    Ok(())
}
