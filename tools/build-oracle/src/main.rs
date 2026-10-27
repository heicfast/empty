use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

fn source(value: u32) -> String {
    format!(
        "fn main() {{ println!(\"{{}}\", value()); }}\n#[inline(never)] fn value() -> u32 {{ {value} }}\n"
    )
}

fn prepare(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"oracle-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn run_builder(dir: &Path, builder: &str, value: u32) -> Result<u128, String> {
    fs::write(dir.join("src/main.rs"), source(value)).map_err(|e| e.to_string())?;
    let mut command = Command::new(builder);
    command.arg("build").arg("--offline").current_dir(dir);
    let start = Instant::now();
    let build = command
        .output()
        .map_err(|e| format!("starting {builder}: {e}"))?;
    let elapsed = start.elapsed().as_micros();
    if !build.status.success() {
        return Err(format!(
            "{builder} build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        ));
    }
    let binary = dir
        .join("target/debug")
        .join(format!("oracle-fixture{}", env::consts::EXE_SUFFIX));
    let output = Command::new(binary)
        .current_dir(dir)
        .output()
        .map_err(|e| e.to_string())?;
    let expected = format!("{value}\n");
    if !output.status.success() || output.stdout != expected.as_bytes() || !output.stderr.is_empty()
    {
        return Err(format!(
            "{builder} produced incorrect behavior for {value}: status={}, stdout={:?}, stderr={:?}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(elapsed)
}

fn main_inner() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: build-oracle-smoke <cargo executable> <candidate executable> <new empty working directory>".into());
    }
    let cargo = args[0].to_str().ok_or("cargo path must be valid UTF-8")?;
    let candidate = args[1]
        .to_str()
        .ok_or("candidate path must be valid UTF-8")?;
    let root = PathBuf::from(&args[2]);
    // Exclusive creation prevents overwriting or cleaning an existing directory.
    fs::create_dir(&root).map_err(|e| format!("working directory must not exist: {e}"))?;
    let a = root.join("cargo");
    let b = root.join("candidate");
    prepare(&a)?;
    prepare(&b)?;
    println!("iteration,builder,build_us,correct");
    for value in 0..6 {
        // Alternate order to reduce a simple order bias. The first iteration is cold.
        let entries = if value % 2 == 0 {
            [(&a, cargo, "cargo"), (&b, candidate, "candidate")]
        } else {
            [(&b, candidate, "candidate"), (&a, cargo, "cargo")]
        };
        for (dir, executable, label) in entries {
            let micros = run_builder(dir, executable, value)?;
            println!("{value},{label},{micros},true");
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match main_inner() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::source;
    #[test]
    fn edits_change_observable_behavior() {
        assert_ne!(source(1), source(2));
        assert!(source(42).contains("{ 42 }"));
    }
}
