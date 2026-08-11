# LSP 설치 UI(프론트) 구현 시 자율 판단 기록

> 범위: 7.10-W2 잔여 — `LspServerStatusList` 개편 + `entities/lsp` 확장.
> `docs/acknowledge/2026-08-11-lsp-w2-spine-decisions.md`·`2026-08-11-lsp-w2-impl-decisions.md` 가
> 만든 데이터 위에서 프론트 UI 를 구현했다.

## 1. `LspServerDetection` 에 필드 2종 추가 (Rust, 최소 확장)

프론트가 다음 두 가지를 프로액티브하게(클릭 전에) 판단하려면 백엔드 정보가 필요했는데, 기존
`LspServerDetection` 에는 없었다.

- **`downloadAvailable: Option<bool>`** — `download` 전략 서버가 현재 플랫폼(`platform_key()`)의
  `urls`/`sha256` 를 모두 갖췄는지. `None` = 애초에 `install.download` 자체가 없음(기존 5종:
  vtsls·rustAnalyzer·basedPyright·ruff·marksman — `install.strategy` 는 `download` 로 표기돼 있지만
  실제로는 npm/uv/rustup/brew 수동 설치 대상), `Some(false)` = 다운로드 스펙은 있으나 이 플랫폼 미지원,
  `Some(true)` = 설치 버튼 활성화 가능.
- **`toolchainTool: Option<String>`** — `toolchain` 전략 서버의 실행 바이너리명(`go`/`gem`/`cs`/`ghcup`,
  기존 `service.rs::toolchain_binary()` 재사용). 버튼 라벨에 "설치를 대행할 도구"를 노출하기 위함.

`service.rs::detect_servers()` 안에서 기존 `toolchain_available`/`sdk_available` 과 동일한 방식으로
계산만 추가했다(새 로직·새 설치 경로 없음). `cargo fmt`→`clippy -D warnings`→`cargo test --workspace`
(357+6+9 전량 통과, `tests::typescript_바인딩을_생성한다` 로 bindings 재생성 확인) 로 검증했다.

이 두 필드가 없었다면 "sha256 null 이면 버튼 비활성화" 를 클릭 전에 보여줄 방법이 없었다
(런타임 에러로 사후 표시만 가능 — `lsp_install` 은 이미 이 두 경우 모두 한국어 `AppError` 메시지를
던진다). 대안으로 클라이언트에서 `install_hint` 텍스트를 파싱해 도구명을 추출하는 방법도 검토했으나
(`"go install ..."` 의 첫 토큰이 실제로 `go` 와 일치), 텍스트 형식에 의존하는 방식이라 채택하지 않았다.

## 2. i18n 은 Contract 가 이미 넣어 둔 `settings.lsp*` 14개 키만 사용

`domain/locale/service.rs` 에 `lspInstall`·`lspInstalling`·`lspInstallFailed`·`lspCancel`·
`lspToolchainMissing`·`lspSdkMissing`·`lspExperimental`·`lspChecksumPending` 등이 이미 en/ko/ja +
MESSAGE_NAMESPACES 4곳에 동기돼 있어 새 키를 추가하지 않았다. 툴체인 설치 버튼 라벨은
`"{toolchainTool} " + t('settings.lspInstall')`(예: "go 설치")로 조합해 새 인터폴레이션 키 없이
도구명을 노출했다.

## 3. `downloadAvailable !== true` 사유 텍스트가 기존 5종엔 부정확할 수 있음

`lspChecksumPending`("체크섬이 아직 게시되지 않았습니다")은 문자 그대로는 "다운로드 스펙은 있는데
체크섬만 없다"는 뜻이지만, 기존 5종은 애초에 `install.download` 자체가 없다(순수 수동 설치).
이 경우에도 같은 문구를 재사용했다 — 사용 가능한 키가 이것뿐이고, 바로 아래 실제 수동 설치
커맨드(`installHint`)와 복사 버튼이 함께 노출되므로 실사용에 지장은 없다고 판단했다. 더 정확한
문구가 필요하면 별도 키(`lspManualInstallOnly` 등) 추가를 다음 웨이브에서 검토.

## 4. Monaco 언어 매핑(항목 4) — 프론트 변경 없음, 실측만

`OpenedFile.languageId` 는 Rust `LANGUAGE_ID_BY_EXTENSION` 이 유일한 소스이며, 프론트는
`editor-pane.tsx` 에서 `language={file.languageId}` 로 Monaco 에 그대로 전달한다(별도 확장자 맵
없음). 신규 14종 언어(haskell·zig·erb·heex 포함, W2 impl 결정 §4)에 대한 Monaco 구문강조 등록이나
`LANGUAGE_SERVERS_BY_LANGUAGE_ID`(에디터↔LSP 세션 실배선용) 매핑 추가는 W2 범위 밖으로 이미 확정된
"Phase 5 잔여" 항목이라 손대지 않았다.

## 5. `shared/ui/progress.tsx` 신규 추가 — 신규 의존성 아님

`radix-ui`(통합 패키지)는 이미 의존성에 있고 `Progress` 프리미티브만 아직 로컬에 생성돼 있지
않았다. 다른 `shared/ui/*` 파일과 동일하게 shadcn 원본 스타일(`function` 컴포넌트)을 그대로
따랐다 — 이 디렉토리는 벤더/스캐폴딩 코드로 취급되는 기존 패턴(button.tsx·switch.tsx·checkbox.tsx
전부 `function`)을 유지한 것.
