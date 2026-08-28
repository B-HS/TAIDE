# 기능 — VSCode 테마 임포트 (7.10-W5)

> 결정 근거: `docs/acknowledge/2026-08-11-qa5-batch-decisions.md` — "extension: 테마/문법/스니펫
> 임포트로 명명. 이번 단계는 VSIX 로컬 임포트 + 테마 추출". **extension host 실행(코드 실행형)은
> 미지원 공식 선언**, MS Marketplace 연동은 약관상 금지(로컬 `.vsix` 파일만 받는다).
> IPC 계약 정본은 `ipc-contract.md` "vsix" 절.

## 1. 범위 (이 웨이브)

- 로컬 `.vsix` 파일(사용자가 dialog 로 직접 선택)에서 **테마 기여만** 추출한다. Rust
  (`vsix_extract_themes`)는 원본 텍스트 추출까지만 담당한다.
- VS Code 확장의 `contributes.grammars`(TextMate 문법)·스니펫·커맨드 등 다른 기여점은 이
  단계에서 다루지 않는다. 7.10-W7 에서 TextMate 문법 렌더링 엔진(shiki) 자체는 확정됐지만,
  **VSIX 에서 grammar 를 추출해 신규 언어를 늘리는 기능은 W7 에서도 범위 밖**이다
  (`docs/backlog.md` 참고 — 언어 id 충돌 정책과 `LANGUAGE_ID_BY_EXTENSION` 런타임화가 선행돼야
  하는 별개 축). 임포트가 테마의 `tokenColors`(원본 TextMate 룰)는 보존한다(§9, §10).
- 변환(VSCode 색상 키 → TAIDE 시맨틱 토큰)은 프론트가 담당한다 — `scripts/convert-vscode-theme.ts`
  의 순수 로직을 `src/shared/lib/theme-convert/` 로 옮겨 CLI 스크립트와 임포트 플로우가 공유한다
  (2회 이상 사용 규칙 충족, §9).
- 저장은 기존 `theme_save` IPC(`themes/` 디렉토리)를 재사용한다 — 임포트 테마는 사용자 로컬
  파일이라 라이선스 번들 문제가 없다. 중복 이름 처리·관리 UI(현행 컴포넌트명은 Wave I 통합
  이후 `PluginInstallButton`/`VsixImportDialog` — §9)도 이 웨이브에 포함된다.

## 2. VSIX 포맷과 Rust 의 책임

`.vsix` 는 zip 아카이브다. `vsce`(공식 패키징 도구)가 만드는 레이아웃은 고정돼 있다.

```
extension.vsixmanifest       (XML — 파싱하지 않음, §3 참고)
[Content_Types].xml
extension/
├── package.json             확장 매니페스트 (npm package.json 형식, 표준 JSON)
│   └── contributes.themes: [{ label, uiTheme, path, id? }]
└── themes/*.json             실제 테마 파일 (VSCode 는 주석 포함을 금지하지 않음 — §4)
```

`vsix_extract_themes(vsixPath)` 는:

1. `extension/package.json` 을 표준 JSON 으로 파싱해 `name`·`displayName`(없으면 `name`)·
   `publisher`·`version`(임포트 출처 표기용)과 `contributes.themes[]` 를 읽는다. `name` 은 VS Code
   확장 매니페스트 규격상 항상 ASCII 인 안정 식별자라, 프론트가 테마 id 를 만들 때는
   `displayName` 이 아니라 `publisher`+`name` 을 우선한다(§4-1) — `displayName` 은 NLS
   플레이스홀더(`%key%`)나 비ASCII 문자열일 수 있어 id 재료로 부적합하기 때문이다.
2. `extension/package.nls.json` 이 있으면 읽어 `%key%` 플레이스홀더를 치환한다(§4-1). 파일이
   없거나 파싱에 실패해도 전체 호출은 실패하지 않고 원문 플레이스홀더를 그대로 둔다.
3. 각 테마 기여의 `path` 를 `extension/` 루트 기준으로 정규화해 아카이브에서 찾고, **원본 문자열
   그대로**(`rawJson`) 반환한다 — 파싱·변환하지 않는다(§4).
4. 테마 JSON 최상단에 `"include": "..."` 가 있으면(VSCode 의 테마 체이닝 — 상속 테마가 부모
   파일을 지정하는 방식) 그 경로를 현재 파일 기준 상대경로로 풀어 재귀적으로 따라간다
   (`includeChain[]`, §5). include 가 없으면 빈 배열.
5. 항목 하나가 손상돼도(경로 탈출·파싱 실패·과대 용량) 전체 호출을 실패시키지 않고 그 항목만
   건너뛴다 — `plugins.md` §3 "하나가 깨져도 나머지는 계속 로드된다"와 같은 견고성 원칙.

## 3. `extension.vsixmanifest`(XML) 를 파싱하지 않는 이유

VSIX 최상단의 `extension.vsixmanifest` 는 XML 이고, publisher·버전 등을 `Identity` 속성으로도
담고 있다. 이 정보는 **`extension/package.json` 에도 표준 필드(`publisher`/`version`/
`displayName`)로 이미 존재한다**(VSCode 확장 매니페스트 규격상 필수). XML 파서 의존성을 새로
들이지 않고도(신규 패키지 금지 — 세션 지시) 같은 정보를 얻을 수 있으므로 XML 은 읽지 않는다.

## 4. 테마 JSON 은 원본 그대로 반환한다 (파싱하지 않음)

VSCode 의 공식 테마 스키마는 순수 JSON 이지만, 실제로 배포되는 테마 파일 중 일부는 주석을
포함한다(엄밀히는 스펙 위반이지만 VSCode 자체가 관대하게 파싱해 동작한다). `scripts/
convert-vscode-theme.ts` 에는 이미 이런 파일을 다루기 위한 `stripJsonComments`/`parseJsonc`
가 있다 — **Rust 가 다시 구현하지 않는다.** `vsix_extract_themes` 는 `rawJson`/
`includeChain[].rawJson` 을 파싱 없이 그대로 돌려주고, 실제 변환(주석 제거·색상 매핑·경고 수집)
은 그 기존 로직을 소비하는 프론트 몫이다.

`include` 경로 탐지만은 예외적으로 필요하다(다음 파일을 찾아야 체인을 이어갈 수 있으므로).
텍스트에 정규식을 직접 매칭하면 주석 안의 `"include": "..."` 나 최상위가 아닌 중첩 필드까지
잘못 집어낼 수 있으므로, 자체 주석 스캐너(`strip_json_comments` — TS `jsonc.ts` 의
`stripJsonComments` 와 동일한 상태 기계를 Rust 로 재구현, 문자열 리터럴 안의 `//`/`/* */` 는
건너뛴다)로 주석을 제거하고 트레일링 콤마를 정리한 뒤 `serde_json::Value` 로 구조적으로
파싱해 **최상위 `include` 필드만** 읽는다. 파싱 자체가 실패하면(심하게 손상된 JSON) 그 지점에서
include 체인이 끊긴다(§6).

### 4-1. NLS 플레이스홀더 치환

`extension/package.nls.json` 은 `{ "key": "value" }` 또는 `{ "key": { "message": "value", ... } }`
형태를 지원한다(VS Code `vscode-nls` 규격). 두 형태 모두 읽어 평면 `key → message` 테이블을
만들고, `displayName` 과 각 테마 기여의 `label` 에서 `%key%` 패턴을 이 테이블로 치환한다.
테이블에 없는 키는 원문 그대로 남긴다(잘못된 치환보다 명시적으로 남은 플레이스홀더가 더 안전
하다). 로케일별 NLS 파일(`package.nls.<locale>.json`)은 이번 웨이브에서 다루지 않는다 — 기본
(영어) NLS 파일만 읽는다.

## 5. 경로 안전성 — 아카이브 내 가상 경로 정규화

플러그인 도메인(`plugins.md` §3)은 실제 파일시스템 경로라 `fs::canonicalize` 로 탈출을
검증하지만, zip 엔트리는 파일시스템에 존재하지 않는 **가상 경로**라 그 방법을 쓸 수 없다.
`normalize_zip_path(base_dir, relative)` 가 대신 세그먼트 단위로 `.`/`..` 를 직접 접어(zip
엔트리는 항상 `/` 구분자), 결과가 `extension/` 루트 아래로 유지되는지 확인한다. 아래 두
지점에서 쓰인다.

- 테마 기여의 `path` (base = `extension/`, 예: `./themes/foo.json` → `extension/themes/foo.json`)
- `include` 값 (base = **현재 테마 파일의 부모 디렉토리** — VSCode 의 실제 동작과 동일하게,
  `extension/` 루트가 아니라 include 를 선언한 파일 기준 상대경로로 푼다)

`../../../../etc/passwd` 류 시도는 세그먼트가 바닥나거나 첫 세그먼트가 `extension` 이 아니게
되어 거부된다. 개별 항목의 거부는 §2-5 대로 그 항목만 건너뛰는 것으로 처리한다(전체 실패 아님).

## 6. include 체인 — 깊이 상한과 순환 보호

- `VSIX_INCLUDE_CHAIN_MAX_DEPTH = 5` — `scripts/convert-vscode-theme.ts` 의 동명 상수와 같은
  값을 맞췄다(코드 공유는 아니고 의미상 정합만 — Rust 는 값만, 실제 병합 로직은 프론트가 가진다).
  깊이를 넘기면 에러 없이 그 지점에서 체인을 멈춘다.
- 방문한 아카이브 경로를 집합으로 추적해 `A includes B, B includes A` 같은 순환을 즉시 끊는다
  (깊이 상한과 별개의 이중 방어).
- 체인 중 한 파일이라도 없거나 경로 탈출이면 그 지점에서 체인을 멈추고, **이미 얻은 부분은
  그대로 반환**한다(주 테마 자체는 항목 스킵 대상이 아니다 — include 실패는 "그 테마를 아예
  버린다"가 아니라 "체인이 거기서 끊긴다"로 처리한다).

## 7. 크기 상한

`VSIX_ENTRY_MAX_BYTES = 2MiB`(파일 크기 4단계 임계값의 `LARGE_FILE_BYTES` 와 같은 값 — 테마
JSON 은 통상 수십~수백 KB 라 여유 있게 잡았다). zip 메타데이터의 선언 크기를 그대로 신뢰하지
않고 `Read::take(cap + 1)` 로 **실제 읽은 바이트 수**를 상한 이후에 검사한다 — 압축 아카이브의
선언 크기 필드는 신뢰할 수 없는 입력이므로(조작된 zip 이 압축 해제 시 훨씬 큰 데이터를 뱉을 수
있음) 실제 읽기 자체를 잘라야 한다.

엔트리 1건당 상한만으로는 총량을 막지 못한다 — manifest 가 작은 테마 기여를 아주 많이
선언하면(각 기여가 자신의 include 체인까지 끌고 온다) 엔트리 개별 상한을 넘지 않고도 누적
압축 해제량이 커질 수 있다. 이를 막기 위해 두 상한을 추가로 둔다.

- `VSIX_MAX_THEME_CONTRIBUTIONS = 64` — manifest·nls 파일을 제외하고 처리하는 테마 기여 개수의
  상한. 초과분은 `log::warn!` 후 건너뛴다.
- `VSIX_TOTAL_MAX_EXTRACTED_BYTES = 64MiB` — manifest·nls·테마 본문·include 체인 전체를 합친
  누적 추출 바이트 예산(`ExtractionBudget`). 예산을 넘는 엔트리를 읽으려 하면 그 항목(또는 include
  체인의 그 지점)만 실패로 처리되고 나머지는 계속 진행된다 — §2-5 의 견고성 원칙과 동일하게
  개별 실패가 전체 호출을 막지 않는다.

## 8. 루트 가드 미적용

`vsixPath` 는 `resolve_within_open_project` 를 거치지 않는다. 프론트 `dialog` 로 사용자가
명시적으로 고른 파일이고, Rust 는 그 경로를 **읽기만** 한다(쓰기·삭제 없음). `project_open(path)`
가 이미 같은 성격의 예외다 — 프로젝트 루트를 정하는 진입점 자체이므로 검증할 기존 "루트"가
없다. 상세는 `ipc-contract.md` "vsix" 절 참고.

## 9. 완료한 것과 다음 단계

이번 웨이브에서 완료:

- `scripts/convert-vscode-theme.ts` 의 순수 변환 로직을 `src/shared/lib/theme-convert/` 로
  옮겨 CLI 스크립트와 임포트 플로우가 공유한다(2회 이상 사용 규칙 충족).
- 임포트 UI(Wave I 에서 테마·grammar 임포트 단일 다이얼로그로 통합 — 현행 컴포넌트명):
  `PluginInstallButton`(`plugin-install-button.tsx`, 파일 dialog → `vsix_extract_themes` 호출) →
  `VsixImportDialog`(`vsix-import-dialog.tsx`, 변환 → 미리보기 → 중복 이름 처리 → `theme_save` IPC 로 저장,
  `themes/` 디렉토리 재사용 — 임포트 테마는 사용자 로컬 파일이라 라이선스 번들 문제 없음).
  `src/widgets/plugin-manager/plugin-manager.tsx` 에 실배선되고(설정 화면의 PLUGINS 섹션 —
  `src/widgets/settings-view/settings-plugins-section.tsx` 가 이 위젯을 그대로 임베드).
- i18n `settings.themeImport*` 키는 `locale/service.rs` 의 `MESSAGE_NAMESPACES` +
  `src-tauri/resources/locales/{en,ko,ja}.json` 4곳 동기로 관리한다(T2-I 외부화 반영). 실소비
  키는 `src/widgets/plugin-manager/vsix-import-dialog.tsx` 의 5키 — 성공/중복
  (`themeImportSuccess`/`themeImportDuplicate`)과 항목 단위 실패 사유
  (`themeImportThemeParseFailure`/`themeImportThemeIncomplete`/`themeImportThemeContrastFailure`).
  초기 웨이브의 `themeImportButton`/`themeImportDialogTitle`/`themeImportFailure` 는 임포트 UI 가
  plugin-manager 다이얼로그로 옮겨가며 미소비가 되어 T2-I 미참조 정리에서 제거됐다(재도입 시
  3언어+스키마 재추가 필요).

다음 웨이브 몫 (범위 밖):

- VSIX `contributes.grammars` 임포트(신규 언어 추가) — W7 에서도 범위 밖(`docs/backlog.md`).
  TextMate 문법 렌더링 엔진 자체(기존 31언어 대상)는 7.10-W7 에서 shiki 로 확정됐다
  (`docs/theme-system.md` §4.2).
- 로케일별 NLS 파일(`package.nls.<locale>.json`) — 현재는 기본(영어) `package.nls.json` 만 읽는다(§4-1).

## 10. tokenColors 보존 (7.10-W7)

`convertVscodeTheme` 의 반환에 `tokenColors`(TAIDE `Theme` 스키마 §2.1 형태)가 추가됐다.
`readVscodeTheme` 이 `fontStyle` 원문 필드를 함께 보존하므로, VSIX 로 임포트한 테마는 변환
시점에 버려지던 원본 TextMate 룰(및 underline 등 bold/italic 으로 표현 못 하던 fontStyle)이
그대로 저장된다. `buildTheme`(`src/shared/lib/vsix-theme-import.ts` 내부 함수)이 이 필드를 `theme_save`
페이로드에 실어 전달한다. 상세 스키마·오버레이 합성 규칙은 `docs/theme-system.md` §2.1·§4.2.2.
