use std::error::Error;
use std::io;
use std::process::Command;

use taide_migration_metrics::{is_product_source, nonblank_physical_lines, rust_product_lines};

const PERCENT_SCALE: f64 = 100.0;

fn git_output(arguments: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = Command::new("git").args(arguments).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!("git failed: {}", output.status)).into());
    }
    Ok(output.stdout)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let (revision, is_worktree) = match arguments.as_slice() {
        [] => ("HEAD", false),
        [mode] if mode == "--worktree" => ("HEAD", true),
        [mode, revision] if mode == "--revision" => (revision.as_str(), false),
        _ => {
            return Err(io::Error::other(
                "usage: taide-migration-metrics [--revision REF | --worktree]",
            )
            .into());
        }
    };
    let commit_expression = format!("{revision}^{{commit}}");
    let commit = String::from_utf8(git_output(&[
        "rev-parse",
        "--verify",
        "--end-of-options",
        &commit_expression,
    ])?)?
    .trim()
    .to_string();
    let paths_output = if is_worktree {
        git_output(&["ls-files", "-z"])?
    } else {
        git_output(&["ls-tree", "-r", "--name-only", "-z", &commit])?
    };
    let paths = std::str::from_utf8(&paths_output)?;
    let mut rust_lines = 0;
    let mut typescript_lines = 0;
    let mut rust_files = 0;
    let mut typescript_files = 0;
    for path in paths.split('\0').filter(|path| is_product_source(path)) {
        let source = if is_worktree {
            std::fs::read_to_string(path)?
        } else {
            String::from_utf8(git_output(&["show", &format!("{commit}:{path}")])?)?
        };
        if path.ends_with(".rs") {
            rust_lines += rust_product_lines(&source)
                .map_err(|error| io::Error::other(format!("{path}: {error}")))?;
            rust_files += 1;
        } else {
            typescript_lines += nonblank_physical_lines(&source);
            typescript_files += 1;
        }
    }
    let total = rust_lines + typescript_lines;
    if total == 0 {
        return Err(io::Error::other("no tracked product source found").into());
    }
    let rust_percent = rust_lines as f64 / total as f64 * PERCENT_SCALE;
    println!(
        "{{\"policy\":\"tracked-product-nonblank-physical-v1\",\"commit\":\"{commit}\",\"worktree\":{is_worktree},\"rust_files\":{rust_files},\"typescript_files\":{typescript_files},\"rust_lines\":{rust_lines},\"typescript_lines\":{typescript_lines},\"rust_percent\":{rust_percent:.4}}}"
    );
    Ok(())
}
