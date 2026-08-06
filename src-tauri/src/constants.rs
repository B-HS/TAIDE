pub const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
    ".idea",
    ".DS_Store",
];

pub const WATCH_DEBOUNCE_MS: u64 = 300;

pub const LARGE_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const LARGE_FILE_LINES: usize = 50_000;
pub const READ_ONLY_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const REFUSED_FILE_BYTES: u64 = 50 * 1024 * 1024;

const _: () = assert!(LARGE_FILE_BYTES < READ_ONLY_FILE_BYTES);
const _: () = assert!(READ_ONLY_FILE_BYTES < REFUSED_FILE_BYTES);

pub fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIR_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 무시_목록은_대표_디렉토리를_포함한다() {
        assert!(is_ignored_dir(".git"));
        assert!(is_ignored_dir("node_modules"));
        assert!(!is_ignored_dir("src"));
    }
}
