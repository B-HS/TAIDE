pub use taide_model::settings::*;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use regex::Regex;

    use super::*;

    /// `R5#3` — the field set `Settings` actually serializes (read off `Default`'s real serde
    /// output, so a future `#[serde(rename = ...)]` override is honored rather than guessed from
    /// the Rust identifier) must match the field set the generated TS binding exposes. Every field
    /// here is `#[serde(default)]`, so a field added to one side and forgotten on the other never
    /// fails to deserialize — it just silently stops round-tripping. This test is the parity check
    /// that gap has been missing.
    ///
    /// Only the TS side is text-sliced (the Rust side above is real serde output, not source
    /// parsing): the block between `export type Settings = {` and its first following `};` — safe
    /// only because `Settings` is a flat field bag with no nested `{ ... }` object-typed field to
    /// contain an earlier `};` of its own. A future field whose TS type is an inline object literal
    /// would truncate this slice at that nested closer, silently dropping every field declared after
    /// it from `ts_fields` (`rust_fields` would then look like it grew fields the TS side lacks, not
    /// the reverse). See `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md` §1 T1-E.
    #[test]
    fn settings_필드_집합은_rust와_bindings_ts에서_일치한다() {
        let serialized = serde_json::to_value(Settings::default()).expect("Settings 직렬화 성공");
        let rust_fields: BTreeSet<String> = serialized
            .as_object()
            .expect("Settings 는 JSON 객체로 직렬화된다")
            .keys()
            .cloned()
            .collect();

        let bindings_source = include_str!("../../../../src/shared/api/bindings.ts");
        let start = bindings_source
            .find("export type Settings = {")
            .expect("bindings.ts 에서 Settings 타입 시작을 찾을 수 없습니다")
            + "export type Settings = {".len();
        let end = bindings_source[start..]
            .find("};")
            .expect("bindings.ts 에서 Settings 타입 끝을 찾을 수 없습니다");
        let block = &bindings_source[start..start + end];
        let pattern = Regex::new(r"(?m)^\s*([a-zA-Z_][a-zA-Z0-9_]*)\??:\s").expect("유효한 정규식");
        let ts_fields: BTreeSet<String> = pattern.captures_iter(block).map(|capture| capture[1].to_string()).collect();

        assert_eq!(
            rust_fields, ts_fields,
            "Rust Settings 필드 집합과 bindings.ts 의 Settings 타입 필드 집합이 다릅니다"
        );
    }
}
