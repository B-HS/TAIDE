# 7.10-W2 — LSP 원클릭 매니페스트 14종 반영 + 설치 실행 경로 구현 시 판단 기록

> 범위: `docs/PROCESS.md` 7.10-W2. Contract 문서(`2026-08-11-lsp-w2-spine-decisions.md`)가 만든
> 스키마·인프라 위에서, 리서치가 확정한 14종 서버 데이터를 `lsp-servers.json` 에 반영하고
> 설치 실행 경로(툴체인 대행·SDK 프로브·다운로드)를 완성했다.

## 1. zls(Zig Language Server) 제외 — 14종 중 13종만 반영

리서치 데이터의 `archive: "tar-xz"` 는 현재 `LspArchiveKind` 에 없는 값이고, 이를 지원하려면
`xz2` 크레이트가 필요하다. 사용자 확정 사항("승인 신규 의존성: reqwest·sha2·flate2·tar·zip
(필요시 xz2 는 보고 후 보류)")에 따라 **xz2 는 추가하지 않았다** — darwin-arm64 용 zls 배포가
`builds.zigtools.org`/GitHub 모두 `.tar.xz` 뿐이라 대체 포맷이 없으므로, zls 는 이번 배치에서
제외했다.

**메인 결정 필요**: (a) xz2 를 승인하고 zls 를 download 전략으로 추가하거나, (b) zls 를
`sdk-detect`(brew 설치 여부만 감지, 자동 설치는 미지원)로 낮춰서 추가.

## 2. taplo `archive: "gz"` — `LspArchiveKind::Gz` 신규 추가 (승인된 flate2 만 사용)

taplo 의 darwin-arm64 배포는 tar 컨테이너가 아닌 단일 gzip 압축 바이너리(`taplo-darwin-aarch64.gz`)다.
이미 승인된 `flate2` 만으로 해제 가능하므로(새 의존성 불필요), `LspArchiveKind` 에 `Gz` 변형을
추가하고 `infra::lsp_install::decompress_gz()` + 기존 `write_binary()` 조합으로 처리했다.
(`domain/lsp/types.rs`, `infra/lsp_install.rs`, `domain/lsp/commands.rs::run_download_install`)

## 3. 관리 디렉토리 실행파일 해석: `command.bin` 과 `install.download.binPathInArchive` 분리

`resolve_command()` 는 기존에 `command.bin` 하나만으로 프로젝트 로컬 → managed_dir → PATH 순서
탐색을 했다. clangd(`clangd_22.1.6/bin/clangd`)·kotlin-lsp(`kotlin-server-262.9593.0/bin/intellij-server`)
처럼 아카이브 루트에 버전 문자열이 박힌 하위 경로에 실행 파일이 있는 경우, `command.bin` 을
그 상대경로로 채우면 managed 설치는 동작하지만 PATH 폴백(예: brew 로 설치된 `clangd`)이 절대
매칭되지 않는 문제가 있었다(리서치가 이미 지적).

`resolve_command(command, root, managed_dir, managed_relative_path)` 로 4번째 인자를 추가해
managed_dir 해석에는 `install.download.binPathInArchive` 를 우선 사용하고, `command.bin` 은
PATH/프로젝트 로컬 탐색에 쓰이는 **단순 실행파일명**으로 되돌렸다(`clangd`·`intellij-server`·
`lua-language-server`). 버전 문자열은 `binPathInArchive` 한 곳에만 존재해, 서버 버전을 올릴 때
`command.bin` 을 함께 고칠 필요가 없다. (`domain/lsp/service.rs::resolve_command`/`detect_servers`,
`domain/lsp/commands.rs::spawn_process`)

## 4. `sdk_detect`/`toolchain` 프로브 평가를 `install.strategy` 와 분리

기존 `detect_servers()` 는 `strategy == SdkDetect` 일 때만 `sdk_detect.probes` 를 평가했다.
jdtls 는 `strategy: download` 이면서 JDK 21 전제조건을 `install.sdkDetect` 로 함께 표현하는데,
기존 코드는 이 필드를 무시해 `LspServerDetection.sdk_available` 이 항상 `None` 이었다.

`sdk_available`/`toolchain_available` 평가를 전략값이 아니라 **해당 필드의 존재 여부**로
바꿔(`spec.install.sdk_detect.as_ref().map(...)`, `spec.install.toolchain.as_ref().map(...)`),
download 전략에 딸린 SDK 전제조건도 정상적으로 감지 결과에 반영되게 했다. `manifest.rs` 의
`validate_install` 도 대응해, `sdk_detect` 필드는 어느 전략에서든 존재하면 `probes` 비어있음을
검증하고, `sdk-detect` 전략인데 필드 자체가 없으면 검증 실패로 바꿨다(기존에는 조용히 통과했음).

## 5. sha256 검증 상태 — 메인 재검증 권장 3건

리서치 데이터의 sha256 은 모두 값이 채워져 있어 `null` 로 남긴 항목은 없다. 다만 출처가 갈린다.

- **공식 체크섬 파일로 확인됨** (그대로 신뢰): jdtls, expert, terraform-ls, kotlin-lsp
- **공식 체크섬 미공개 — 리서치가 로컬 다운로드 후 직접 계산**: clangd, lua-language-server, taplo

후자 3건은 무결성 근거가 상대적으로 약하다는 리서치의 자체 평가를 그대로 승계한다. 메인이
독립적으로 재다운로드·재계산해 일치를 재확인하는 것을 권장한다(설치 시 sha256 불일치는
`AppError::Internal` 로 하드 실패하므로, 값이 틀렸다면 해당 서버는 설치 자체가 항상 실패한다 —
보안상 fail-closed 이나 사용성엔 영향).

## 6. THIRD_PARTY_LICENSES.md — 다운로드형 7종만 추가

`toolchain`/`sdk-detect` 전략(gopls·ruby-lsp·metals·haskell-language-server·dart·sourcekit-lsp)은
TAIDE 가 바이너리를 다운로드·재배포하지 않고(시스템 툴체인이 자체 설치하거나 이미 설치된 SDK 를
감지만 함) 라이선스 고지 대상이 아니라고 판단해 제외했다. `download` 전략 7종(jdtls·clangd·
expert·lua-language-server·taplo·terraform-ls·kotlin-lsp)만 새 섹션 "Downloaded Language Servers"
로 추가했다. kotlin-lsp 는 저장소 라이선스(Apache-2.0)와 별개로 배포 아카이브에 JBR·일부
proprietary 컴포넌트가 포함된다는 리서치 지적을 그대로 반영해 별도 경고 문구를 남겼다.

## 7. 검증

`cargo fmt --all` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace`
(357 + 6 + 9 = 372 테스트 전량 통과, `tests::typescript_바인딩을_생성한다` 로 bindings 재생성 확인) →
`bun run typecheck` → `bun run lint`(기존 경고 4건, 이번 변경과 무관 — `file-tree.tsx`/`commit-graph.tsx`
의 `useVirtualizer` React Compiler skip) → `bun test`(340 pass) 전부 통과.
