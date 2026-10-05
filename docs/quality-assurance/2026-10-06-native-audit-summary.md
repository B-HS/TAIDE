# Rust-native 전환 전수 감사 요약 (2026-10-06)

## 대상과 방법

- 대상: 브랜치 `to_rust_native`, HEAD `2824005`, 미커밋 작업 트리(추적 수정 44개, 미추적 `native/` Rust 약 16.7만 줄 포함 405건).
- 방법: 영역 10개를 workflow 작업자 10개(opus 3, sonnet 7)가 읽기 전용으로 감사했습니다. TS 기능을 화면·상태·상호작용 단위로 열거하고, native 구현 여부와 실제 앱 실행 경로(`main.rs` → `NativeApplication`) 도달 여부를 호출 체인으로 판정했습니다.
- 메인 직접 확인: native 앱 `cargo check` exit 0(51.34초). 감사 핵심 판정 6건(전역 Visuals 미적용, 설정 section 8개 고정, OS drop 처리 0건, 다중 viewport 호출 0건, `poll_agents` native 호출부 0건, 미지원 탭 폴백 문구)을 `rg`로 교차 확인해 일치했습니다.
- 빌드·테스트 실행과 실기 검증은 감사 범위에 넣지 않았습니다.

영역별 상세는 같은 디렉터리의 `2026-10-06-native-audit-{shell,editor,terminal-agent,explorer-search,preview,git,settings,commands-ui,ipc-coverage,architecture}.md`입니다.

## 판정 기준

| 상태 | 의미 |
| --- | --- |
| done | native 구현이 있고 실제 앱에서 도달하며 TS 동작과 대응 |
| partial | 실제 앱에 연결돼 있으나 하위 동작 일부 누락 |
| unwired | 모듈·테스트·백엔드는 있으나 실제 앱 화면에서 도달 불가 |
| missing | native 구현 없음 |
| n/a | 웹 기술 전용이라 대응 불필요 |

## 기능 대응 집계

| 영역 | 전체 | done | partial | unwired | missing | n/a |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 셸·창·탭·프로젝트 | 99 | 32 | 25 | 4 | 33 | 5 |
| 편집기·LSP·스니펫·AI | 87 | 23 | 11 | 6 | 45 | 2 |
| 터미널·에이전트·IDE·태스크 | 78 | 38 | 7 | 10 | 17 | 6 |
| 탐색기·검색·아웃라인·문제 | 46 | 18 | 5 | 2 | 20 | 1 |
| 미리보기 | 40 | 24 | 12 | 0 | 3 | 1 |
| Git | 65 | 0 | 3 | 33 | 28 | 1 |
| 설정 계열 | 83 | 44 | 7 | 21 | 8 | 3 |
| 명령·키맵·공용 UI | 102 | 33 | 20 | 9 | 32 | 8 |
| **화면 기능 합계** | **600** | **212** | **90** | **85** | **186** | **27** |

n/a를 뺀 573개 기준 done은 212개(37.0%)입니다. 이전 기록의 363/433(83.83%)은 세부 체크리스트 완료 수이며 기능 대응률이 아닙니다.

별도 축:

- IPC command 206종 중 native UI 호출부가 있는 것은 91종, facade만 있고 native 호출부가 없는 것은 109종(Git 41, AI 8, sync 5, 원격 자격증명 5, 검색 4 포함), facade가 `src-tauri`에만 있는 것은 2종(`agent_cli_install`·`agent_cli_uninstall`)입니다. event 30종 중 native가 반영하는 것은 15종입니다.
- 구조 감사 49항목 중 정상 12, 보강 필요 25, 미연결 3, 기반 누락 9입니다.

## 실제로 되어 있는 것

- 메인 창 slot·pane 분할, 상태바, Zen, dirty 탭 닫기, 종료 drain, 세션 복원
- 문서 저장소·IME·저장·자동 저장·hot exit·충돌 배너·untitled·app-file, LSP 문서 동기화·진단 수신·저장 시 포맷
- 터미널 표면(PTY 연결, 선택, 붙여넣기, 링크, OSC 7/8/133, 컨텍스트 메뉴, 마우스 모드)
- 파일 트리 CRUD·클립보드·컨텍스트 메뉴 대부분, 문제 패널
- 미리보기 6종 캐시와 UI 크롬
- 설정 section 8개, 테마 편집기, 키바인딩 편집기, 스니펫 편집기, 시스템 사용량, 토스트 엔진
- 키맵 엔진과 기본 키맵 41개, i18n 카탈로그(en·ko·ja 각 1,108키)

## 없는 것 (사용자 영향 순)

1. **편집기 표시 계층**: `editor_surface.rs`(724줄)는 단색 평문 위젯입니다. 구문 강조, 찾기/바꾸기, 접기, minimap, word wrap, 다중 커서 조작, 편집 명령 약 150종, 컨텍스트 메뉴가 없습니다. 장식·오버레이를 받을 확장 지점이 없어 재설계가 선행 조건입니다.
2. **LSP 상호작용 UI**: 완성, hover, signature help, 정의·참조 이동, rename, code action, inlay hint, semantic token, 진단 밑줄이 없습니다. 요청 계약 27종은 테스트에서만 호출됩니다.
3. **Git 화면 전체**: SCM 패널, 브랜치, 커밋 그래프, diff 탭, 파일 히스토리, blame, gutter, 충돌 해결이 없습니다(done 0). 백엔드 41종은 원격 경로에서만 호출됩니다.
4. **사이드바 뷰 전환과 검색·아웃라인 패널**: files/search/git/outline tablist가 없고 파일 트리 하나만 있습니다.
5. **커맨드 팔레트와 명령 레지스트리**: 5개 모드 전부 없습니다. 키 8개(⌘P, ⌘⇧P, ⌘T, ⌘F, ⌘⇧F, ⌘⇧H, ⌘⇧E, ⌃⇧G)는 실행 대상이 없습니다.
6. **Diff·ClaudeDiff·SearchEditor·Welcome 탭 표면**: 해당 탭은 라벨과 내부 문구만 표시합니다.
7. **셸 상호작용 계층**: 모든 drag and drop, 프로젝트·그룹·탭 컨텍스트 메뉴와 다이얼로그, 프로젝트 닫기, 최근 프로젝트, OS 파일 drop, native 메뉴바, 창 상태 복원이 없습니다.
8. **보조 창**: `main.rs`는 단일 창이며 보조 창 렌더 경로는 호출되지 않습니다. 기존 데이터의 보조 창 탭은 보이지 않습니다.
9. **설정 section 6개**: LSP, AI, Plugins, Sync, Remote, Performance UI가 없어 AI 토큰, GitHub 동기화, 원격 비밀번호·링크, 플러그인·LSP·CLI 설치를 native에서 할 수 없습니다.
10. **에이전트·알림·IDE 응답·태스크**: 에이전트 감지 폴링, OS 알림 발송, IDE diff·저장·선택·진단 응답, Task Runner가 native 화면에 연결돼 있지 않습니다.
11. **공용 UI 기반**: 테마 토큰이 egui 전역 Visuals에 반영되지 않고, Dialog·Popover 같은 공용 프리미티브 모듈과 파일 타입 아이콘이 없습니다.

## 잘못 구현됐거나 보강이 필요한 것

### 동작 결함 (high)

| 결함 | 근거 | 조치 |
| --- | --- | --- |
| 선택 없이 복사·잘라내기 시 클립보드를 빈 문자열로 덮어씀 | `native/taide-native-ui/src/editor_surface.rs:486-494` | 현재 줄 복사·잘라내기 |
| undo가 글자 단위로 쪼개짐 | `native/taide-native-editor/src/editing.rs:281`, `store.rs:1163-1168` | 타이핑 세션 단위 그룹 |
| macOS 표준 이동·삭제 키 미처리 | `editor_surface.rs:571-596` | 단어·줄·문서·페이지 이동과 삭제 추가 |
| Enter 자동 들여쓰기와 Shift+Tab 없음 | `editor_surface.rs:617-630` | 들여쓰기 유지, 줄 단위 indent·outdent |
| 출력 과부하 시 backpressure 없이 터미널 세션 kill | `native/taide-native-app/src/terminal_frames.rs:117-178`, `native/taide-native-terminal/src/session.rs:70-82` | pause·resume로 교체 |
| 에이전트 감지가 동작하지 않음 | `poll_agents` 호출부가 `src-tauri/src/lib.rs`뿐 | native에 주기 작업 등록 |
| IDE `openDiff`가 600초 대기 후 거절, `saveDocument` 항상 실패 | `native/taide-native-app/src/ide-tools.rs:144-283` | ClaudeDiff 탭과 저장 응답 경로 연결 |
| 원격 접근을 켜도 링크·비밀번호를 만들 수 없음 | `remote_issue_link` 등 native 호출부 0건 | Remote section 구현 |
| 실행 불가 명령이 키바인딩 편집기에 편집 가능 행으로 노출 | `keybinding-commands.json` 212개 중 157개가 `monaco.*` | 명령 레지스트리로 교체 |
| 테마가 egui 전역 Visuals에 반영되지 않음 | `set_visuals`·`set_style` 호출 0건 | 토큰에서 Visuals 생성·적용 |
| 보조 창 탭이 데이터에는 있으나 접근 불가 | `shell.rs`가 `layout.root`만 렌더 | 보조 창 구현 또는 메인 병합 방식 결정 |

### 구조 문제

| 문제 | 근거 | 권장 |
| --- | --- | --- |
| native 크레이트 6개가 루트 workspace 밖의 개별 workspace·lock, `[patch]` 4중 복제, CI 미포함 | `native/*/Cargo.toml` | 단일 native workspace와 CI 편입 |
| 제품 크레이트가 `experiments/` 경로에 의존 | `taide-native-app/Cargo.toml`의 eframe·vte path, fixture bin·example | fork를 `native/vendor`로 이동 |
| 공유 target 디렉터리 약 219GB | `experiments/native-shell-spike/target`, 통합 테스트 바이너리 61개 | 전용 target과 dev profile, 테스트 바이너리 통합 |
| 실행 파일 크레이트에 원격 HTTP/WS·IDE 서버·미리보기 파서 포함, Tauri 쪽과 전처리 이중 구현 | `remote-*.rs`, `remote-gateway.rs` | `taide-remote`·`taide-ide`로 추출 |
| `application.rs` 5,534줄(필드 92개), `terminal_surface.rs` 8,294줄 | 전역 키맵 라우팅이 터미널 모듈에 위치 | 키맵·상태 소유자 분리 |
| 오류가 영구 상태바 문자열로 남음 | `self.status = Some(...)` 약 100곳 | 로컬라이즈된 toast로 전환 |
| `cfg(test)`에서 다른 크레이트 소스를 `include!`로 복제 | `native-app/src/keymap.rs:1-21` 외 | 테스트를 소유 크레이트로 이동 |
| HostBridge가 명령을 단일 worker로 직렬 처리, 큐 초과 시 폐기 | `host.rs:15,452-500` | 도메인별 task와 backpressure |
| `--data-dir` 필수와 격리 keychain 서비스명 | `bootstrap.rs:15,24-52` | 기본 데이터 경로·서비스명 승계 정책 결정 |
| `taide-model`이 `taide-remote-wire`에 의존 | `crates/taide-model/Cargo.toml` | 의존 방향 복원 |
| 로거 미초기화, 패키징·서명 경로 없음, 원격 브라우저 번들 생성 스크립트 없음 | 검색 0건, `remote-assets.rs:14-16` | cutover 전 기반 작업 |

### 목표와 어긋나는 범위

- 브라우저 Wasm 클라이언트(`native/taide-remote-web`, 약 1.35만 줄)와 원격 dispatch에 작업이 집중돼, 데스크톱 native 화면보다 원격 경로가 먼저 연결된 기능이 많습니다.
- HTML·오디오·비디오 미리보기는 vendored wry의 WKWebView에, PDF·AVIF·ICC는 macOS CoreGraphics/ImageIO에 의존합니다. macOS 외 플랫폼에서는 실패 화면이 됩니다.
- egui hard gate 6종(다중 창, CJK IME 실기, 접근성, native 메뉴, 외부 drop, GPU fallback) 중 코드로 닫힌 것은 없습니다.

추적 파일 44개 변경은 Tauri 전용 코드를 공유 crate로 옮긴 리팩터링이며, 읽은 범위에서 IPC 계약과 원격 허용표를 깨는 변경은 발견되지 않았습니다.

## 전환 배치 순서

| 배치 | 내용 | 선행 |
| --- | --- | --- |
| 1 | 기존 native 결함 수정과 기반: 편집기 입력 결함, 터미널 backpressure·resize·오류 표시, 에이전트 폴링, 테마 Visuals, 창 설정, 미지원 탭 안내, 탐색기 스크롤, XLSX 셀 | 없음 |
| 2 | 명령 레지스트리, 커맨드 팔레트(파일·명령·줄 이동), 공용 Dialog·Popover·아이콘 모듈, toast 일반 API와 status 문자열 이전 | 1 |
| 3 | 편집기 표시 계층 재설계(줄 매핑·장식·gutter·오버레이)와 구문 강조, 찾기/바꾸기, 편집 명령 | 1, 구문 강조 엔진 결정 |
| 4 | 사이드바 뷰 tablist, 검색·교체 패널, 아웃라인, 탐색기 아이콘·Git 데코레이션 | 2 |
| 5 | LSP 상호작용 UI(완성, hover, 정의·참조, rename, code action, 진단 밑줄) | 3 |
| 6 | Git: SCM 패널, diff 렌더러, 히스토리, 그래프, blame·gutter, ClaudeDiff와 IDE 응답 | 3, 4 |
| 7 | 셸 상호작용: 탭·프로젝트 컨텍스트 메뉴, drag and drop, 그룹 관리, Welcome, OS drop, native 메뉴 | 2, 메뉴 의존성 결정 |
| 8 | 설정 section 6개, 알림 발송, 에이전트 UI, Task Runner | 2 |
| 9 | 보조 창(다중 viewport), 창 상태 복원 | 7 |
| 10 | 구조 정리(workspace 통합, vendor 이동, crate 추출, 대형 파일 분해), 패키징·서명, 데이터 경로 승계, cutover 게이트 | 전체 |

## 사용자 결정이 필요한 사항

1. 미커밋 405건의 체크포인트 커밋 여부. 이전 합의는 M8 완료 전 commit·push 보류입니다. 미추적 파일은 git worktree 격리를 쓸 수 없어 병렬 구현이 제한되고 유실 위험이 있습니다.
2. 구문 강조 엔진. TS는 Shiki(TextMate 문법)와 VSIX 문법을 사용합니다. TextMate 호환 엔진(예: syntect)과 tree-sitter 중 선택이 필요하며 새 의존성입니다.
3. native 메뉴바 의존성(예: muda) 추가 여부.
4. 보조 창 방식: eframe 다중 viewport 구현 또는 메인 창 병합.
5. 브라우저 Wasm 클라이언트 작업을 데스크톱 native 대응 완료 전까지 동결할지 여부.
6. HTML·미디어 미리보기의 WebView 의존 유지 여부와 macOS 외 플랫폼 지원 범위.
7. 기본 데이터 경로와 keychain 서비스명 승계 시점.
