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
