# 라이선스 MIT 채택 검토·결정 (2026-09-05)

> 사용자 지시: "현재 License 를 MIT 로 가능한지 검토". 결론 **가능** — `LICENSE`(MIT, Copyright (c) 2026
> Hyunseok Byun) 추가, `package.json`·`src-tauri/Cargo.toml`·`crates/taide-cli/Cargo.toml` 에 `license` 필드 명시.
> 서드파티 고지 정본은 `THIRD_PARTY_LICENSES.md`.

## 1. 검토 범위와 방법

| 대상 | 방법 | 결과 |
|------|------|------|
| npm 프로덕션 의존 폐쇄 집합 171개 | `package.json` `dependencies` 에서 `node_modules` 를 따라 내려가며 `license` 필드 수집 (스크립트, devDependencies 제외) | MIT 153 · Apache-2.0 11 · ISC 2 · 0BSD 1 · 듀얼(MIT/Apache) 3 · `dompurify` MPL-2.0 OR Apache-2.0 |
| Rust macOS 그래프 392 crate | `cargo metadata --filter-platform aarch64-apple-darwin` 의 resolve 노드 × `license` 필드 | 전부 permissive 또는 permissive 선택지가 있는 듀얼. 단독 copyleft·UNKNOWN·license-file 전용 0건 |
| 번들 테마 36·TextMate 문법 30·다운로드 LSP 서버·emmet | 기존 `THIRD_PARTY_LICENSES.md` | 이미 고지 완비 (2026-08-12 회색지대 4건 결정 포함) |
| 번들 폰트·이미지 | `src`·`src-tauri/icons`·`resources` 탐색 | 자체 제작 앱 아이콘뿐. 아이콘 라이브러리는 `lucide-react`(ISC) |

## 2. MIT 와의 호환 판단

- **Apache-2.0** (pdfjs-dist, xlsx/SheetJS 계열, class-variance-authority, tao, zopfli, sync_wrapper, OpenSSL 3 소스): MIT 앱에 포함 가능. 배포 시 저작권·라이선스 고지 유지 의무 → `THIRD_PARTY_LICENSES.md` 요약 절로 충족.
- **MPL-2.0 파일 단위 copyleft**: npm `dompurify` 는 Apache-2.0 선택. Rust `cssparser`·`cssparser-macros`·`selectors`·`dtoa-short`·`option-ext` 는 무수정 사용 — 수정하지 않는 한 의무 없음(HCL 문법과 같은 정책).
- **Unicode-3.0**(ICU4X 18 crate)·**CDLA-Permissive-2.0**(webpki-roots)·**Zlib**·**BSD-2/3**·**ISC**·**CC0**·**Unlicense**: 전부 permissive.
- **vendored 네이티브**: libgit2(GPL-2.0 with linking exception — 링크 예외로 MIT 앱 허용), libssh2(BSD-3), OpenSSL 3(Apache-2.0), liblzma(0BSD), zlib(Zlib). 전부 정적 링크 허용.
- **git2 `vendored-openssl`** 의 OpenSSL 은 Apache-2.0 이므로 위 고지 의무만 있고 광고 조항(구 1.x) 없음.
- 다운로드형 LSP 서버(EPL·Apache·MPL·JetBrains 독점 포함)는 **레포·번들에 포함되지 않아** 앱 라이선스와 무관(사용자 기기가 upstream 에서 직접 받음).

## 3. 결정

1. 프로젝트 본체 라이선스 **MIT**. 저작권자 표기는 raw-viewer 와 동일한 `Copyright (c) 2026 Hyunseok Byun`.
2. `THIRD_PARTY_LICENSES.md` 상단에 "TAIDE 본체는 MIT" 를 명시하고, 말미에 런타임 의존성 라이선스 요약 절을 추가해 Apache-2.0 고지 의무를 문서로 충족.
3. 이후 의존성을 추가할 때 GPL/AGPL/LGPL(정적)·SSPL·BUSL 등 MIT 와 충돌하는 라이선스는 도입 전에 사용자 확인. MPL 은 무수정 조건에서만.

## 4. 검토 스크립트

세션 스크래치에서 실행한 2개(`npm-licenses.ts`·`rust-licenses.ts`)는 저장소에 넣지 않았다. 재검토가 필요하면
`cargo metadata --format-version 1 --filter-platform aarch64-apple-darwin` 과 `node_modules/*/package.json` 의 `license`
필드를 다시 집계한다(위 표의 방법).
