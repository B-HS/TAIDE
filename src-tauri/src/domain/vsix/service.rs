use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

use super::types::{
    VsixExtensionInfo, VsixExtractedTheme, VsixThemeExtractionResult, VsixThemeIncludeEntry, VSIX_ENTRY_MAX_BYTES, VSIX_EXTENSION_ROOT,
    VSIX_INCLUDE_CHAIN_MAX_DEPTH, VSIX_MANIFEST_ENTRY, VSIX_MAX_THEME_CONTRIBUTIONS, VSIX_TOTAL_MAX_EXTRACTED_BYTES,
};
use crate::error::{AppError, AppResult};

const DEFAULT_UI_THEME: &str = "vs-dark";
const VSIX_NLS_ENTRY: &str = "extension/package.nls.json";

type NlsTable = HashMap<String, String>;

#[derive(Debug, Deserialize)]
struct PackageJsonManifest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "displayName")]
    display_name: Option<String>,
    #[serde(default)]
    publisher: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    contributes: Option<PackageJsonContributes>,
}

#[derive(Debug, Default, Deserialize)]
struct PackageJsonContributes {
    #[serde(default)]
    themes: Vec<PackageJsonThemeContribution>,
}

#[derive(Debug, Deserialize)]
struct PackageJsonThemeContribution {
    #[serde(default)]
    label: Option<String>,
    #[serde(default, rename = "uiTheme")]
    ui_theme: Option<String>,
    path: String,
}

struct ExtractionBudget {
    remaining: u64,
}

impl ExtractionBudget {
    fn new(total: u64) -> Self {
        Self { remaining: total }
    }

    fn consume(&mut self, bytes: u64) -> AppResult<()> {
        if bytes > self.remaining {
            return Err(AppError::InvalidArgument("vsix 테마 추출 용량 상한을 초과했습니다".to_string()));
        }
        self.remaining -= bytes;
        Ok(())
    }
}

pub fn extract_themes(vsix_path: &Path) -> AppResult<VsixThemeExtractionResult> {
    let file = File::open(vsix_path).map_err(|error| AppError::InvalidArgument(format!("vsix 파일을 열 수 없습니다: {error}")))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| AppError::InvalidArgument(format!("vsix 압축을 해제할 수 없습니다: {error}")))?;

    let mut budget = ExtractionBudget::new(VSIX_TOTAL_MAX_EXTRACTED_BYTES);
    let manifest_text = read_zip_entry_string(&mut archive, VSIX_MANIFEST_ENTRY, &mut budget)?;
    let manifest: PackageJsonManifest = serde_json::from_str(&manifest_text)
        .map_err(|error| AppError::InvalidArgument(format!("extension/package.json 파싱 실패: {error}")))?;
    let nls_table = read_nls_table(&mut archive, &mut budget);

    let extension = extension_info_from_manifest(&manifest, &nls_table);
    let contributions = manifest.contributes.map(|value| value.themes).unwrap_or_default();

    if contributions.len() > VSIX_MAX_THEME_CONTRIBUTIONS {
        log::warn!(
            "vsix 테마 기여가 상한({VSIX_MAX_THEME_CONTRIBUTIONS})을 초과해 나머지 {}개를 건너뜁니다",
            contributions.len() - VSIX_MAX_THEME_CONTRIBUTIONS
        );
    }

    let themes = contributions
        .into_iter()
        .take(VSIX_MAX_THEME_CONTRIBUTIONS)
        .filter_map(
            |contribution| match extract_one_theme(&mut archive, &contribution, &nls_table, &mut budget) {
                Ok(theme) => Some(theme),
                Err(error) => {
                    log::warn!("vsix 테마 항목을 건너뜁니다 ({}): {error}", contribution.path);
                    None
                }
            },
        )
        .collect();

    Ok(VsixThemeExtractionResult { extension, themes })
}

fn read_nls_table(archive: &mut zip::ZipArchive<File>, budget: &mut ExtractionBudget) -> NlsTable {
    let Ok(text) = read_zip_entry_string(archive, VSIX_NLS_ENTRY, budget) else {
        return NlsTable::new();
    };
    let Ok(serde_json::Value::Object(entries)) = serde_json::from_str::<serde_json::Value>(&text) else {
        return NlsTable::new();
    };

    entries
        .into_iter()
        .filter_map(|(key, value)| {
            let message = match value {
                serde_json::Value::String(text) => Some(text),
                serde_json::Value::Object(fields) => fields.get("message").and_then(|inner| inner.as_str()).map(str::to_string),
                _ => None,
            };
            message.map(|message| (key, message))
        })
        .collect()
}

fn resolve_nls_placeholders(text: &str, table: &NlsTable) -> String {
    if table.is_empty() {
        return text.to_string();
    }
    static NLS_PLACEHOLDER_PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = NLS_PLACEHOLDER_PATTERN.get_or_init(|| Regex::new(r"%([A-Za-z0-9_.]+)%").expect("valid nls placeholder regex"));
    pattern
        .replace_all(text, |captures: &regex::Captures| {
            table.get(&captures[1]).cloned().unwrap_or_else(|| captures[0].to_string())
        })
        .into_owned()
}

fn extension_info_from_manifest(manifest: &PackageJsonManifest, nls_table: &NlsTable) -> VsixExtensionInfo {
    let name = manifest.name.clone().unwrap_or_default();
    let display_name = manifest.display_name.clone().unwrap_or_else(|| name.clone());
    VsixExtensionInfo {
        name,
        display_name: resolve_nls_placeholders(&display_name, nls_table),
        publisher: manifest.publisher.clone().unwrap_or_default(),
        version: manifest.version.clone().unwrap_or_default(),
    }
}

fn extract_one_theme(
    archive: &mut zip::ZipArchive<File>,
    contribution: &PackageJsonThemeContribution,
    nls_table: &NlsTable,
    budget: &mut ExtractionBudget,
) -> AppResult<VsixExtractedTheme> {
    let entry_path = normalize_zip_path(VSIX_EXTENSION_ROOT, &contribution.path)?;
    let raw_json = read_zip_entry_string(archive, &entry_path, budget)?;

    let label = contribution
        .label
        .clone()
        .map(|label| resolve_nls_placeholders(&label, nls_table))
        .unwrap_or_else(|| theme_label_fallback(&entry_path));
    let ui_theme = contribution.ui_theme.clone().unwrap_or_else(|| DEFAULT_UI_THEME.to_string());
    let include_chain = resolve_include_chain(archive, &entry_path, &raw_json, budget);

    Ok(VsixExtractedTheme {
        label,
        ui_theme,
        raw_json,
        include_chain,
    })
}

fn theme_label_fallback(entry_path: &str) -> String {
    Path::new(entry_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| entry_path.to_string())
}

fn resolve_include_chain(
    archive: &mut zip::ZipArchive<File>,
    start_entry_path: &str,
    start_raw_json: &str,
    budget: &mut ExtractionBudget,
) -> Vec<VsixThemeIncludeEntry> {
    let mut chain = Vec::new();
    let mut visited = std::collections::HashSet::new();
    visited.insert(start_entry_path.to_string());

    let mut current_entry_path = start_entry_path.to_string();
    let mut current_raw_json = start_raw_json.to_string();

    while chain.len() < VSIX_INCLUDE_CHAIN_MAX_DEPTH {
        let Some(include_relative) = extract_include_path(&current_raw_json) else {
            break;
        };
        let current_dir = parent_zip_dir(&current_entry_path);
        let Ok(include_entry_path) = normalize_zip_path(&current_dir, &include_relative) else {
            break;
        };
        if !visited.insert(include_entry_path.clone()) {
            break;
        }
        let Ok(include_raw_json) = read_zip_entry_string(archive, &include_entry_path, budget) else {
            break;
        };

        chain.push(VsixThemeIncludeEntry {
            path: include_entry_path.clone(),
            raw_json: include_raw_json.clone(),
        });

        current_entry_path = include_entry_path;
        current_raw_json = include_raw_json;
    }

    chain
}

fn parent_zip_dir(entry_path: &str) -> String {
    Path::new(entry_path)
        .parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn strip_json_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut result = String::with_capacity(source.len());
    let mut index = 0;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escape_next = false;

    while index < chars.len() {
        let char = chars[index];
        let next_char = chars.get(index + 1).copied();

        if in_line_comment {
            if char == '\n' {
                in_line_comment = false;
                result.push(char);
            }
            index += 1;
            continue;
        }
        if in_block_comment {
            if char == '*' && next_char == Some('/') {
                in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if in_string {
            result.push(char);
            if escape_next {
                escape_next = false;
            } else if char == '\\' {
                escape_next = true;
            } else if char == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if char == '"' {
            in_string = true;
            result.push(char);
            index += 1;
            continue;
        }
        if char == '/' && next_char == Some('/') {
            in_line_comment = true;
            index += 2;
            continue;
        }
        if char == '/' && next_char == Some('*') {
            in_block_comment = true;
            index += 2;
            continue;
        }
        result.push(char);
        index += 1;
    }

    result
}

fn strip_trailing_commas(source: &str) -> String {
    static TRAILING_COMMA_PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = TRAILING_COMMA_PATTERN.get_or_init(|| Regex::new(r",(\s*[}\]])").expect("valid trailing comma regex"));
    pattern.replace_all(source, "$1").into_owned()
}

fn extract_include_path(raw_json: &str) -> Option<String> {
    let normalized = strip_trailing_commas(&strip_json_comments(raw_json));
    let value: serde_json::Value = serde_json::from_str(&normalized).ok()?;
    value.get("include")?.as_str().map(str::to_string)
}

fn normalize_zip_path(base_dir: &str, relative: &str) -> AppResult<String> {
    let mut segments: Vec<String> = base_dir.split('/').filter(|part| !part.is_empty()).map(str::to_string).collect();

    for part in relative.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                if segments.pop().is_none() {
                    return Err(AppError::InvalidArgument(format!("vsix 경로가 확장 루트를 벗어납니다: {relative}")));
                }
            }
            other => segments.push(other.to_string()),
        }
    }

    if segments.first().map(String::as_str) != Some(VSIX_EXTENSION_ROOT) {
        return Err(AppError::InvalidArgument(format!("vsix 경로가 확장 루트를 벗어납니다: {relative}")));
    }

    Ok(segments.join("/"))
}

fn read_zip_entry_string(archive: &mut zip::ZipArchive<File>, name: &str, budget: &mut ExtractionBudget) -> AppResult<String> {
    let entry = archive
        .by_name(name)
        .map_err(|error| AppError::NotFound(format!("vsix 항목을 찾을 수 없습니다 ({name}): {error}")))?;

    let mut bytes = Vec::new();
    entry
        .take(VSIX_ENTRY_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| AppError::Internal(format!("vsix 항목 읽기 실패 ({name}): {error}")))?;

    if bytes.len() as u64 > VSIX_ENTRY_MAX_BYTES {
        return Err(AppError::InvalidArgument(format!("vsix 항목이 너무 큽니다 ({name})")));
    }
    budget.consume(bytes.len() as u64)?;

    String::from_utf8(bytes).map_err(|_| AppError::InvalidArgument(format!("vsix 항목이 UTF-8 이 아닙니다: {name}")))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use uuid::Uuid;
    use zip::write::SimpleFileOptions;

    use super::*;

    fn temp_vsix_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-vsix-test-{name}-{}.vsix", Uuid::new_v4()))
    }

    fn write_vsix(entries: &[(&str, &[u8])]) -> std::path::PathBuf {
        let mut zip_bytes: Vec<u8> = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut zip_bytes);
            let mut writer = zip::ZipWriter::new(cursor);
            for (name, content) in entries {
                writer.start_file(*name, SimpleFileOptions::default()).unwrap();
                writer.write_all(content).unwrap();
            }
            writer.finish().unwrap();
        }

        let path = temp_vsix_path("fixture");
        std::fs::write(&path, &zip_bytes).unwrap();
        path
    }

    const SIMPLE_PACKAGE_JSON: &str = r#"{
        "name": "one-dark-pro",
        "displayName": "One Dark Pro",
        "publisher": "zhuangtongfa",
        "version": "3.16.0",
        "contributes": {
            "themes": [
                { "label": "One Dark Pro", "uiTheme": "vs-dark", "path": "./themes/OneDark-Pro.json" }
            ]
        }
    }"#;

    #[test]
    fn 기본_테마_1종을_추출한다() {
        let theme_json = r#"{ "name": "One Dark Pro", "colors": {} }"#;
        let path = write_vsix(&[
            ("extension/package.json", SIMPLE_PACKAGE_JSON.as_bytes()),
            ("extension/themes/OneDark-Pro.json", theme_json.as_bytes()),
        ]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.extension.display_name, "One Dark Pro");
        assert_eq!(result.extension.publisher, "zhuangtongfa");
        assert_eq!(result.extension.version, "3.16.0");
        assert_eq!(result.themes.len(), 1);
        assert_eq!(result.themes[0].label, "One Dark Pro");
        assert_eq!(result.themes[0].ui_theme, "vs-dark");
        assert_eq!(result.themes[0].raw_json, theme_json);
        assert!(result.themes[0].include_chain.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn displayname이_없으면_name으로_대체한다() {
        let package_json = r#"{ "name": "plain-ext", "publisher": "acme", "version": "1.0.0", "contributes": { "themes": [] } }"#;
        let path = write_vsix(&[("extension/package.json", package_json.as_bytes())]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.extension.display_name, "plain-ext");
        assert!(result.themes.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn label과_uitheme이_없으면_기본값으로_대체한다() {
        let package_json = r#"{
            "name": "no-meta",
            "version": "1.0.0",
            "contributes": { "themes": [ { "path": "themes/plain.json" } ] }
        }"#;
        let theme_json = r#"{ "colors": {} }"#;
        let path = write_vsix(&[
            ("extension/package.json", package_json.as_bytes()),
            ("extension/themes/plain.json", theme_json.as_bytes()),
        ]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.themes.len(), 1);
        assert_eq!(result.themes[0].label, "plain.json");
        assert_eq!(result.themes[0].ui_theme, "vs-dark");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn include_체인을_따라간다() {
        let package_json = r#"{
            "name": "chained",
            "version": "1.0.0",
            "contributes": { "themes": [ { "label": "Chained", "path": "./themes/main.json" } ] }
        }"#;
        let main_json = r##"{ "include": "./base.json", "colors": { "a": "#fff" } }"##;
        let base_json = r##"{ "colors": { "b": "#000" } }"##;
        let path = write_vsix(&[
            ("extension/package.json", package_json.as_bytes()),
            ("extension/themes/main.json", main_json.as_bytes()),
            ("extension/themes/base.json", base_json.as_bytes()),
        ]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.themes.len(), 1);
        assert_eq!(result.themes[0].raw_json, main_json);
        assert_eq!(result.themes[0].include_chain.len(), 1);
        assert_eq!(result.themes[0].include_chain[0].path, "extension/themes/base.json");
        assert_eq!(result.themes[0].include_chain[0].raw_json, base_json);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn include_순환은_무한루프_없이_멈춘다() {
        let package_json = r#"{
            "name": "cyclic",
            "version": "1.0.0",
            "contributes": { "themes": [ { "label": "Cyclic", "path": "./a.json" } ] }
        }"#;
        let a_json = r#"{ "include": "./b.json" }"#;
        let b_json = r#"{ "include": "./a.json" }"#;
        let path = write_vsix(&[
            ("extension/package.json", package_json.as_bytes()),
            ("extension/a.json", a_json.as_bytes()),
            ("extension/b.json", b_json.as_bytes()),
        ]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.themes.len(), 1);
        assert_eq!(result.themes[0].include_chain.len(), 1);
        assert_eq!(result.themes[0].include_chain[0].path, "extension/b.json");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 경로_탈출을_시도하는_테마_항목은_건너뛴다() {
        let package_json = r#"{
            "name": "escape",
            "version": "1.0.0",
            "contributes": {
                "themes": [
                    { "label": "Escape", "path": "../../../../etc/passwd" },
                    { "label": "Safe", "path": "./themes/safe.json" }
                ]
            }
        }"#;
        let safe_json = r#"{ "colors": {} }"#;
        let path = write_vsix(&[
            ("extension/package.json", package_json.as_bytes()),
            ("extension/themes/safe.json", safe_json.as_bytes()),
        ]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.themes.len(), 1);
        assert_eq!(result.themes[0].label, "Safe");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn package_json이_없으면_에러를_반환한다() {
        let path = write_vsix(&[("README.md", b"no manifest here")]);

        let result = extract_themes(&path);

        assert!(result.is_err());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn package_json_파싱에_실패하면_에러를_반환한다() {
        let path = write_vsix(&[("extension/package.json", b"{ not json")]);

        let result = extract_themes(&path);

        assert!(result.is_err());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 테마_기여가_없으면_빈_목록을_반환한다() {
        let package_json = r#"{ "name": "no-themes", "version": "1.0.0" }"#;
        let path = write_vsix(&[("extension/package.json", package_json.as_bytes())]);

        let result = extract_themes(&path).expect("extract");

        assert!(result.themes.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 상한을_초과하는_테마_항목은_건너뛴다() {
        let package_json = r#"{
            "name": "oversized",
            "version": "1.0.0",
            "contributes": { "themes": [ { "label": "Big", "path": "./themes/big.json" } ] }
        }"#;
        let oversized_content = vec![b'a'; (VSIX_ENTRY_MAX_BYTES + 1) as usize];
        let path = write_vsix(&[
            ("extension/package.json", package_json.as_bytes()),
            ("extension/themes/big.json", &oversized_content),
        ]);

        let result = extract_themes(&path).expect("extract");

        assert!(result.themes.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn nls_플레이스홀더를_치환한다() {
        let package_json = r#"{
            "name": "nls-ext",
            "displayName": "%displayName%",
            "publisher": "acme",
            "version": "1.0.0",
            "contributes": { "themes": [ { "label": "%themeLabel%", "path": "./themes/main.json" } ] }
        }"#;
        let nls_json = r#"{ "displayName": "GitHub Theme", "themeLabel": "Dark" }"#;
        let theme_json = r#"{ "colors": {} }"#;
        let path = write_vsix(&[
            ("extension/package.json", package_json.as_bytes()),
            ("extension/package.nls.json", nls_json.as_bytes()),
            ("extension/themes/main.json", theme_json.as_bytes()),
        ]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.extension.name, "nls-ext");
        assert_eq!(result.extension.display_name, "GitHub Theme");
        assert_eq!(result.themes[0].label, "Dark");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn nls_파일이_없으면_플레이스홀더를_그대로_둔다() {
        let package_json = r#"{
            "name": "no-nls",
            "displayName": "%displayName%",
            "version": "1.0.0",
            "contributes": { "themes": [] }
        }"#;
        let path = write_vsix(&[("extension/package.json", package_json.as_bytes())]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.extension.display_name, "%displayName%");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 주석_안의_include_는_무시하고_실제_include_를_따라간다() {
        let package_json = r#"{
            "name": "commented",
            "version": "1.0.0",
            "contributes": { "themes": [ { "label": "Commented", "path": "./themes/main.json" } ] }
        }"#;
        let main_json = "{\n  // \"include\": \"./decoy.json\",\n  \"include\": \"./base.json\",\n  \"colors\": {}\n}";
        let base_json = r##"{ "colors": { "b": "#000" } }"##;
        let path = write_vsix(&[
            ("extension/package.json", package_json.as_bytes()),
            ("extension/themes/main.json", main_json.as_bytes()),
            ("extension/themes/base.json", base_json.as_bytes()),
        ]);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.themes[0].include_chain.len(), 1);
        assert_eq!(result.themes[0].include_chain[0].path, "extension/themes/base.json");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 테마_기여_개수가_상한을_초과하면_초과분을_건너뛴다() {
        let total = VSIX_MAX_THEME_CONTRIBUTIONS + 5;
        let mut contributions = String::from("[");
        let mut theme_entries: Vec<(String, Vec<u8>)> = Vec::new();
        for index in 0..total {
            if index > 0 {
                contributions.push(',');
            }
            contributions.push_str(&format!(r#"{{ "label": "T{index}", "path": "./themes/t{index}.json" }}"#));
            theme_entries.push((format!("extension/themes/t{index}.json"), br#"{ "colors": {} }"#.to_vec()));
        }
        contributions.push(']');

        let package_json = format!(r#"{{ "name": "many", "version": "1.0.0", "contributes": {{ "themes": {contributions} }} }}"#);

        let mut entries: Vec<(&str, &[u8])> = vec![("extension/package.json", package_json.as_bytes())];
        for (path, content) in &theme_entries {
            entries.push((path.as_str(), content.as_slice()));
        }
        let path = write_vsix(&entries);

        let result = extract_themes(&path).expect("extract");

        assert_eq!(result.themes.len(), VSIX_MAX_THEME_CONTRIBUTIONS);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn extraction_budget_은_상한을_초과하면_에러를_반환한다() {
        let mut budget = ExtractionBudget::new(10);

        assert!(budget.consume(5).is_ok());
        assert!(budget.consume(4).is_ok());
        assert!(budget.consume(2).is_err());
    }

    #[test]
    fn 경로_정규화는_확장_루트_밖으로의_탈출을_거부한다() {
        assert!(normalize_zip_path(VSIX_EXTENSION_ROOT, "../outside.json").is_err());
        assert!(normalize_zip_path("extension/themes", "../../../outside.json").is_err());
        assert_eq!(
            normalize_zip_path(VSIX_EXTENSION_ROOT, "./themes/a.json").unwrap(),
            "extension/themes/a.json"
        );
        assert_eq!(
            normalize_zip_path("extension/themes", "../other/b.json").unwrap(),
            "extension/other/b.json"
        );
    }
}
