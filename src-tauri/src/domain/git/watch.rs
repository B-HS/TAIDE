use std::path::Path;

const GIT_DIR_NAME: &str = ".git";
const GIT_INDEX_FILE: &str = "index";
const GIT_HEAD_FILE: &str = "HEAD";
const GIT_REFS_DIR: &str = "refs";
const GIT_OBJECTS_DIR: &str = "objects";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitInvalidation {
    Status,
    Refs,
}

pub fn classify_git_change(path: &Path) -> Option<GitInvalidation> {
    let components: Vec<&str> = path.components().filter_map(|component| component.as_os_str().to_str()).collect();
    let git_dir_index = components.iter().rposition(|&component| component == GIT_DIR_NAME)?;
    let inside_git_dir = &components[git_dir_index + 1..];

    match inside_git_dir {
        [GIT_OBJECTS_DIR, ..] => None,
        [GIT_INDEX_FILE] => Some(GitInvalidation::Status),
        [GIT_HEAD_FILE] => Some(GitInvalidation::Refs),
        [GIT_REFS_DIR, ..] => Some(GitInvalidation::Refs),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_변경은_status_무효화다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git/index")), Some(GitInvalidation::Status));
    }

    #[test]
    fn head_변경은_refs_무효화다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git/HEAD")), Some(GitInvalidation::Refs));
    }

    #[test]
    fn refs_하위_변경은_refs_무효화다() {
        assert_eq!(
            classify_git_change(Path::new("/repo/.git/refs/heads/main")),
            Some(GitInvalidation::Refs)
        );
        assert_eq!(
            classify_git_change(Path::new("/repo/.git/refs/remotes/origin/main")),
            Some(GitInvalidation::Refs)
        );
    }

    #[test]
    fn objects_하위_변경은_무시한다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git/objects/ab/cd1234")), None);
        assert_eq!(classify_git_change(Path::new("/repo/.git/objects/pack/pack-abc.idx")), None);
    }

    #[test]
    fn 알수없는_git_내부_경로는_무시한다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git/logs/HEAD")), None);
        assert_eq!(classify_git_change(Path::new("/repo/.git/COMMIT_EDITMSG")), None);
    }

    #[test]
    fn git_디렉토리_밖의_경로는_무시한다() {
        assert_eq!(classify_git_change(Path::new("/repo/src/main.rs")), None);
    }

    #[test]
    fn git_디렉토리_자체는_무시한다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git")), None);
    }

    #[test]
    fn 경로에_git_디렉토리가_여러번_등장하면_가장_안쪽_기준으로_분류한다() {
        assert_eq!(
            classify_git_change(Path::new("/workspace/.git/nested-repo/.git/refs/heads/main")),
            Some(GitInvalidation::Refs)
        );
    }
}
