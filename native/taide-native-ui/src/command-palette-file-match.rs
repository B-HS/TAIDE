use crate::fuzzy_match::utf16_len;

const PATH_SEPARATOR: char = '/';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMatchDisplay<'a> {
    pub file_name: &'a str,
    pub dir_path: Option<&'a str>,
    pub file_name_indices: Vec<usize>,
    pub dir_path_indices: Vec<usize>,
}

pub fn relative_path<'a>(root: &str, path: &'a str) -> &'a str {
    let Some(below_root) = path.strip_prefix(root) else {
        return path;
    };
    if root.ends_with(PATH_SEPARATOR) {
        return below_root;
    }
    below_root.strip_prefix(PATH_SEPARATOR).unwrap_or(path)
}

pub fn file_name(path: &str) -> &str {
    path.rfind(PATH_SEPARATOR).map_or(path, |separator| {
        &path[separator + PATH_SEPARATOR.len_utf8()..]
    })
}

pub fn split_for_display<'a>(relative_path: &'a str, indices: &[usize]) -> FileMatchDisplay<'a> {
    let Some(separator) = relative_path.rfind(PATH_SEPARATOR) else {
        return FileMatchDisplay {
            file_name: relative_path,
            dir_path: None,
            file_name_indices: indices.to_vec(),
            dir_path_indices: Vec::new(),
        };
    };
    let dir_path = &relative_path[..separator];
    let separator_index = utf16_len(dir_path);
    FileMatchDisplay {
        file_name: file_name(relative_path),
        dir_path: Some(dir_path),
        file_name_indices: indices
            .iter()
            .filter(|index| **index > separator_index)
            .map(|index| index - separator_index - PATH_SEPARATOR.len_utf16())
            .collect(),
        dir_path_indices: indices
            .iter()
            .copied()
            .filter(|index| *index < separator_index)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzy_match::Matcher;
    use icu_normalizer::DecomposingNormalizer;
    use std::borrow::Cow;

    #[test]
    fn relative_path는_루트_아래_경로만_상대_경로로_바꾼다() {
        assert_eq!(relative_path("/repo", "/repo/src/main.ts"), "src/main.ts");
        assert_eq!(relative_path("/repo/", "/repo/src/main.ts"), "src/main.ts");
        assert_eq!(
            relative_path("/repo", "/repository/a.ts"),
            "/repository/a.ts"
        );
        assert_eq!(relative_path("/repo", "/repo"), "/repo");
        assert_eq!(relative_path("/repo", "/other/a.ts"), "/other/a.ts");
    }

    #[test]
    fn file_name은_마지막_세그먼트를_돌려준다() {
        assert_eq!(
            file_name("/project/src/widgets/search-editor/search-editor-pane.tsx"),
            "search-editor-pane.tsx",
            "경로에서 마지막 세그먼트를 파일명으로 반환한다"
        );
        assert_eq!(
            file_name("package.json"),
            "package.json",
            "슬래시가 없으면 전체 문자열을 그대로 반환한다"
        );
        assert_eq!(
            file_name("/project/src/widgets/"),
            "",
            "디렉토리로 끝나면 빈 문자열을 반환한다"
        );
    }

    #[test]
    fn split_for_display는_파일명과_상위_경로로_나누고_인덱스를_다시_맞춘다() {
        let nested = split_for_display("src/widgets/command-palette/command-palette.tsx", &[]);
        assert_eq!(nested.file_name, "command-palette.tsx");
        assert_eq!(
            nested.dir_path,
            Some("src/widgets/command-palette"),
            "디렉토리가 있으면 파일명과 상위 경로로 나눈다"
        );
        let root = split_for_display("package.json", &[0, 1, 2]);
        assert_eq!(root.file_name, "package.json");
        assert_eq!(
            root.dir_path, None,
            "프로젝트 루트 직속 파일은 dir_path 가 없다"
        );
        assert_eq!(
            root.file_name_indices,
            [0, 1, 2],
            "루트 직속 파일의 매칭 인덱스는 그대로 file_name_indices 에 담긴다"
        );
        assert!(root.dir_path_indices.is_empty());
        let directory = split_for_display("src/x.ts", &[0]);
        assert_eq!(
            directory.dir_path_indices,
            [0],
            "디렉토리 부분의 매칭 인덱스는 dir_path_indices 에 그대로 담긴다"
        );
        assert!(directory.file_name_indices.is_empty());
        let name = split_for_display("src/x.ts", &[4]);
        assert_eq!(
            name.file_name_indices,
            [0],
            "파일명 부분의 매칭 인덱스는 구분자 길이만큼 당겨진다"
        );
        assert!(name.dir_path_indices.is_empty());
        let separator = split_for_display("src/x.ts", &[3]);
        assert!(
            separator.file_name_indices.is_empty() && separator.dir_path_indices.is_empty(),
            "구분자 자체가 매칭 인덱스면 양쪽 어디에도 포함되지 않는다"
        );
        let both = split_for_display("widgets/pane-node-view.tsx", &[0, 9, 17]);
        assert_eq!(both.dir_path, Some("widgets"));
        assert_eq!(both.file_name, "pane-node-view.tsx");
        assert_eq!(both.dir_path_indices, [0]);
        assert_eq!(
            both.file_name_indices,
            [1, 9],
            "디렉토리·파일명 양쪽에 걸친 매칭 인덱스를 각각 올바르게 분리한다"
        );
    }

    #[test]
    fn split_for_display는_nfd_경로를_nfc_질의로_찾아도_파일명_강조가_어긋나지_않는다() {
        let decomposed = DecomposingNormalizer::new_nfd()
            .normalize("src/한글/문서.ts")
            .into_owned();
        let ranked = Matcher::new("en-US")
            .unwrap()
            .filter(
                "문서",
                [decomposed.as_str()],
                |path| Cow::Borrowed(*path),
                None,
            )
            .remove(0);
        let display = split_for_display(&ranked.label, &ranked.matched.indices);
        assert_eq!(display.file_name, "문서.ts");
        assert_eq!(display.dir_path, Some("src/한글"));
        assert_eq!(display.file_name_indices, [0, 1]);
        assert!(display.dir_path_indices.is_empty());
    }
}
