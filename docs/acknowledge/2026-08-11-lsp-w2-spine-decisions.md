# 7.10-W2 — LSP 원클릭 스파인 구현 시 자율 판단 기록

> 범위: `docs/PROCESS.md` 7.10-W2 체크리스트의 "스펙 데이터화 + 설치 인프라" 부분.
> 14종 신규 서버 매니페스트 채움(java·go·ruby·flutter·swift·scala·haskell·elixir·clangd 등)과
> sha256 확정은 명시적으로 이번 작업 범위 밖 — 다음 단계(메인 직접 작업)로 남긴다.

## 1. `LspServerId`: enum → newtype(`String`) transparent struct

처음엔 `pub type LspServerId = String;` (순수 타입 별칭)으로 구현했으나, `tauri-specta` 는
타입 별칭에 대해 명명된 TS 타입을 생성하지 않고 항상 `string` 으로 인라인한다
(`bindings.ts` 에 `export type LspServerId = ...` 가 없어 프론트 `LspServerId` import 가 깨짐).

`ids.rs` 의 `string_id!` 매크로가 쓰는 `#[serde(transparent)] struct ProjectId(String)` 패턴을
그대로 따라 `LspServerId(pub String)` newtype 으로 전환 — `bindings.ts` 에
`export type LspServerId = string;` 가 정상 생성된다 (`ProjectId`/`PaneId`/`TabId` 와 동일 패턴).
`Display`·`From<String>`·`From<&str>`·`PartialEq<str>`·`PartialEq<&str>`·`as_str()` 를 부여해
기존 테스트의 문자열 리터럴 비교(`spec.id == "vtsls"`)를 그대로 유지했다.

## 2. `find_root` 디스패치: match on `LspServerId` → match on 데이터 필드 `root_strategy`

매니페스트 스키마 초안은 `rootMarkers[]` 하나만 요구했지만, 기존 5종의 루트 탐지 알고리즘은
서로 다른 특수 로직을 갖는다 (JS/TS 는 Deno 마커 회피, Rust 는 카고 워크스페이스 우선 탐색,
Python 은 pyproject → .venv 폴백). 순수 마커 목록만으로는 이 세 가지를 표현할 수 없어,
`rootMarkers` 외에 **`rootStrategy`**(`nearestMarker`|`jsTsDenoAware`|`cargoWorkspace`|`venvFallback`)
필드를 추가했다. 기존 서버 ID 하드코딩 `match` 는 이 데이터 필드 기반 `match` 로 대체됐고,
탐지 알고리즘 자체(마커 상수 포함)는 그대로 재사용해 **기존 lsp 단위 테스트를 문자 그대로
통과**시켰다(시그니처만 유지, 서버 ID 전달 대신 `LanguageServerSpec` 전달로 소폭 조정).

## 3. `install.strategy` 값: 기존 5종 전부 `download` 로 표기, 세부 정보는 미기입

승인된 하이브리드 설치 전략은 `download`|`toolchain`(go·gem·coursier·ghcup 전용)|`sdk-detect`
세 가지다. 기존 5종(vtsls·rust-analyzer·basedpyright·ruff·marksman)은 npm/uv/rustup/brew 로
설치되어 왔고, 이 중 어느 것도 승인된 `toolchain.tool` 4종에 해당하지 않는다.

`download` 를 잠정 기본값으로 채택했다 — 다섯 서버 모두 결국 GitHub Release 또는 레지스트리
바이너리 다운로드로 귀결 가능한 후보이고(예: rust-analyzer·ruff·marksman 은 실제로 플랫폼별
prebuilt 바이너리를 배포한다), `install.download` 서브 객체는 `None` 으로 비워 두었다.
`hint` 필드에 기존 `install_hint` 텍스트를 그대로 보존해 사용자에게 보이는 안내는 깨지지 않는다.
`lsp_install` 커맨드를 이 다섯 중 하나로 호출하면 "다운로드 정보가 아직 설정되지 않았습니다"
로 명확히 실패한다(자동 설치 시도 없음) — 다음 단계가 `download.urls`/`sha256`/`version` 을
채우면 그대로 동작한다.

## 4. 확장자 → languageId: Monaco 미지원 언어(haskell·zig·erb·heex)

`node_modules/monaco-editor/esm/vs/languages/definitions/*/register.js` 를 직접 열어
실제 등록된 `id`/`extensions` 를 확인했다(java·ruby·dart·swift·scala·elixir·kotlin·lua·hcl 확인,
haskell·zig·erb·heex 는 Monaco 기본 언어에 없음 확인). Monaco 지원 확장자는 Monaco 의 실제
id 를 그대로 사용했고(`tf`→`hcl`, `sbt`→`scala`, `ex`/`exs`→`elixir` 등 Monaco 자체 확장자
매핑과 일치), 미지원 4종은 LSP 표준 languageId 관례(haskell, zig)나 확장자 그대로(erb, heex)를
썼다 — 기존 `toml` 항목도 이미 Monaco 미지원 상태로 존재해 온 선례를 따랐다. 구문 강조는
plaintext 로 폴백되지만 LSP `textDocument/didOpen` 의 `languageId` 값은 올바르게 전달된다.

## 5. 남은 것 (다음 단계)

- 14종 신규 서버의 `lsp-servers.json` 항목 추가(다운로드 URL·sha256·툴체인/SDK 세부)
- Monaco 에 haskell/zig/erb/heex 구문 강조가 필요하면 별도 Monarch 문법 등록 검토
- 에디터 ↔ LSP 실배선(Phase 5 잔여, W2 범위 밖)에서 새 서버들의 didOpen languageId 확인
