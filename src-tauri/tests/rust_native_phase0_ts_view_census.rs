use std::path::Path;

use serde_json::Value;

const TS_VIEW_CENSUS: &str = include_str!("fixtures/rust-native/ts-view-components-v1.json");

fn collect_tsx_paths(directory: &Path, repository_root: &Path, paths: &mut Vec<String>) {
    for entry in std::fs::read_dir(directory).expect("src 디렉터리 목록") {
        let entry = entry.expect("src 디렉터리 항목");
        let file_type = entry.file_type().expect("src 항목 종류");
        let path = entry.path();

        if file_type.is_dir() {
            collect_tsx_paths(&path, repository_root, paths);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.ends_with(".tsx") || file_name.ends_with(".test.tsx") {
            continue;
        }

        let relative = path.strip_prefix(repository_root).expect("repository 아래 src 항목");
        paths.push(relative.to_string_lossy().replace('\\', "/"));
    }
}

#[test]
fn ts_view_소스_목록은_전수_fixture와_일치한다() {
    let fixture: Value = serde_json::from_str(TS_VIEW_CENSUS).expect("TS view census fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    let expected = fixture["paths"]
        .as_array()
        .expect("fixture paths")
        .iter()
        .map(|path| path.as_str().expect("fixture path").to_owned())
        .collect::<Vec<_>>();

    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("repository root");
    let mut actual = Vec::new();
    collect_tsx_paths(&repository_root.join("src"), repository_root, &mut actual);
    actual.sort();

    assert_eq!(actual, expected);
}
