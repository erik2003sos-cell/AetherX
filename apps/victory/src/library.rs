//! Discover installed native PC games and launch one from a numbered menu.

use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::launch;

/// Discover executables under the supplied roots, or common Windows game
/// folders when no roots are supplied. Scanning is intentionally bounded.
pub fn run(roots: Vec<PathBuf>) -> Result<()> {
    let roots = if roots.is_empty() {
        default_roots()
    } else {
        roots
    };

    let games = discover(&roots);
    anyhow::ensure!(
        !games.is_empty(),
        "no game executables found; pass one or more install folders, e.g. `victory library \"C:\\Games\"`"
    );

    println!("AetherX / VIctory — PC game library");
    println!("Found {} executable(s):\n", games.len());
    for (index, path) in games.iter().enumerate() {
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown");
        println!("{:>2}. {}\n    {}", index + 1, name, path.display());
    }

    print!("\nChoose a number to launch (or 0 to cancel): ");
    io::stdout().flush().context("flushing game selection prompt")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("reading game selection")?;
    let choice: usize = input.trim().parse().context("enter a game number")?;
    if choice == 0 {
        println!("Cancelled.");
        return Ok(());
    }
    let selected = games
        .get(choice - 1)
        .with_context(|| format!("selection {choice} is outside 1..={}", games.len()))?;
    launch::run(selected, &[])
}

fn default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(program_files_x86).join("Steam/steamapps/common"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(program_files).join("Steam/steamapps/common"));
    }
    roots.extend([PathBuf::from(r"C:\Games"), PathBuf::from(r"D:\Games")]);
    roots
}

fn discover(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = BTreeSet::new();
    for root in roots.iter().filter(|root| root.is_dir()) {
        for entry in WalkDir::new(root)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.path();
            if is_game_executable(path) {
                let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                found.insert(resolved);
            }
        }
    }
    found.into_iter().collect()
}

fn is_game_executable(path: &Path) -> bool {
    if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("exe")) {
        return false;
    }
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    ![
        "unitycrashhandler",
        "unitycrashhandler64",
        "crashpad_handler",
        "crashreportclient",
        "ue4prereqsetup",
        "easyanticheat_eos_setup",
        "easyanticheat_setup",
        "vc_redist",
        "vcredist",
        "unins",
        "uninstall",
        "setup",
        "install",
        "gamelaunchhelper",
        "mcplaceholderstub",
    ]
    .iter()
    .any(|excluded| name == *excluded || name.starts_with(&format!("{excluded}_")))
}

#[cfg(test)]
mod tests {
    use super::is_game_executable;
    use std::path::Path;

    #[test]
    fn filters_common_helpers_but_keeps_game_executables() {
        assert!(is_game_executable(Path::new("Among Us.exe")));
        assert!(is_game_executable(Path::new("ROB RUN.exe")));
        assert!(!is_game_executable(Path::new("UnityCrashHandler64.exe")));
        assert!(!is_game_executable(Path::new("gamelaunchhelper.exe")));
        assert!(!is_game_executable(Path::new("readme.txt")));
    }
}
