# HANDOFF — 2026-10-09 세션 스냅샷

> 최종 갱신일: 2026-10-09 · 대응 커밋: `bf5175ca` (브랜치 `to_rust_native`, `origin/to_rust_native`와 동일, 작업 트리 깨끗)
> 2026-10-26 주의: 이 스냅샷 이후 배치 8~23이 진행됐고 배치 24가 진행 중입니다. "배치 7까지만" 지시는 해제됐습니다. 현재 상태는 `docs/PROCESS.md` 맨 위 절과 `docs/quality-assurance/2026-10-10-native-remaining-batch-plan.md`가 정본입니다.
> 이 문서는 새 세션의 단일 진입점입니다. 이전 HANDOFF(2026-10-06, 이전 에이전트의 스니펫 작업 기록 849줄)는 덮어썼고 Git 이력(`bf5175ca:docs/HANDOFF.md`)에 남아 있습니다.

## 1. 프로젝트 한 줄 정의

TAIDE는 Tauri + React/TypeScript로 만든 데스크톱 IDE이며, 지금은 TypeScript view를 걷어내고 UI까지 Rust-native(egui/eframe 0.36.2)로 옮기는 전환(M8)을 진행 중입니다.

## 2. 현재 목표

- **최종 목표**: 기존 TS view의 화면·상태·상호작용을 100% 재현한 Rust-native 앱으로 전환하고, 게이트 통과 뒤 TS·Tauri·Monaco·xterm 코드를 제거한다(`docs/roadmap-rust-native.md` Phase 8·9).
- **현재 마일스톤**: M8 native UI 동등성. 2026-10-06 전수 감사 기준 화면 기능 573개 중 212개(37.0%)가 native에서 동작했고, 이후 배치 1~6과 배치 7 일부를 반영했습니다. 반영 후 기능 대응률은 다시 집계하지 않았습니다.
- **직전 작업**: 전환 배치 7(언어 구성 → 편집 명령 1 → 편집 명령 2, 병렬로 찾기 정규식 조사)을 사용자 지시로 일시정지하고 진행분을 체크포인트로 커밋했습니다(`fbc28b84`).

## 3. 완료 / 진행 중 / 미착수

### 3.1 완료 (커밋 단위, 상세는 각 통합 문서)

| 배치 | 내용 | 주요 커밋 | 통합 기록 |
| --- | --- | --- | --- |
| 인수 | 이전 에이전트의 미커밋 405건 체크포인트 커밋 | `81e96198` `40057316` `fe241af0` `8da7c528` | `docs/PROCESS.md` 항목 g |
| 감사 | 영역 10개 전수 감사(TS 기능 대 native 구현·실제 앱 연결) | `1d5af3b1` | `docs/quality-assurance/2026-10-06-native-audit-summary.md` |
| 1 | 편집기 입력 결함, 터미널 backpressure·resize·오류 해제·정리, 에이전트 폴링, 테마 Visuals, 창 설정, 미지원 탭 안내, 탐색기 스크롤, XLSX 셀 | `67fcfade` `d7d52b98` `ae8c621b` `3d6d6223` | `…native-batch1-integration.md` |
| 2 | 실패 테스트 8건 정리, PTY `openpty` 재시도, 명령 레지스트리(212개 등재), 커맨드 팔레트 | `b461f0b7` `b3531a1c` `30d63474` | `…native-batch2-integration.md` |
| 3 | toast 일반 API와 status 문자열 이전(106곳 중 39 toast·11 로그·56 유지), 공용 modal, 아이콘 레지스트리, 편집기 설계 문서 | `562ef60c` `e425b002` | `…native-batch3-integration.md` |
| 4 | 편집기 표시 계층 골격, 글꼴 체인, word wrap | `f0b3ae0a` `1fd80f32` | `…native-batch4-integration.md` |
| 5 | 구문 강조(`native/taide-native-syntax` 신설, 엔진 게이트, 토큰 파이프라인, 표면 연결, 저장 정리 공급) | `2fe96367` `c819657c` | `…native-batch5-integration.md` |
| 6 | 플러그인·VSIX 문법, 장식·앵커 기반, 접기 | `f4dc0282` `0243b8bb` | `…native-batch6-integration.md` |

통합 기록 경로는 모두 `docs/quality-assurance/2026-10-06-native-batchN-integration.md`입니다. 단계별 상세는 같은 디렉터리의 `2026-10-06-native-batchN-<단계>.md`입니다.

마지막으로 메인이 직접 실행한 전체 테스트(배치 6 종료 시점, `0243b8bb`): native 앱 전체 66개 대상 608 통과·1 실패, syntax 89, editor 124, ui 241, remote-web 53 통과. 실패 1건은 인수 이전부터의 `native/taide-native-app/tests/preview-spreadsheet-xlml.rs`의 `xlml은_할당전_grid_합산_인덱스와_xml_보안_경계를_거절한다`(`<Row ss:Index="0">` 입력이 거절되지 않음)이며 미수정입니다.

### 3.2 진행 중 — 전환 배치 7 (일시정지)

체크포인트 커밋 `fbc28b84`에 단계 1 완료분과 단계 2 진행분이 함께 들어 있습니다. 커밋 전 확인값: native 크레이트 5개 `cargo check --tests` 오류 0, syntax 136·editor 134·ui(lib + editor_surface) 189·앱 lib 369 통과. 앱 전체 통합 테스트 대상은 실행하지 않았습니다.

| 단계 | 상태 | 다음 한 줄 |
| --- | --- | --- |
| 찾기 정규식 방언 조사 | 완료 | `docs/research/2026-10-07-native-find-regex-dialect.md` 7절의 결정 6건을 사용자에게 확인 |
| 1. 언어 구성(Monaco conf 추출, Enter 자동 들여쓰기, 괄호·따옴표 쌍, 접기 표식) | 구현 완료, 리뷰 판정 needs-fix, 차단 1건 미수정 | `native/taide-native-editor/src/auto-indent.rs:410`의 `enclosing_opener_line`이 닫는 괄호 내어쓰기의 짝을 찾을 때 문자열·주석·정규식 안의 괄호까지 셉니다. 줄 토큰 종류가 Other인 괄호만 세도록 고칩니다(Monaco `bracketPairsTree/tokenizer.js:140-144` 근거) |
| 2. 편집 명령 1(줄·텍스트 조작과 Monaco 기본 키) | 구현 도중 중단, 리뷰 전 | 실제 `DocumentEdit`/dispatcher에 연결된 것은 `DeleteAllLeft`·`OutdentLines` 두 종류뿐입니다(`native/taide-native-app/src/command-dispatch.rs:93-94`). 줄 이동·복사·삭제·삽입·합치기·정렬·대소문자·주석 토글과 편집기 문맥 기본 키가 남았습니다 |
| 3. 편집 명령 2(커서·선택·다중 커서) | 시작 전 | 위·아래 커서 추가, ⌘D 계열, 줄 선택, 선택 확장·축소, ⌥클릭, 컬럼 선택 |
| 배치 7 통합 검증·기록 | 미실행 | 변경 크레이트 전체 테스트 대상을 `--no-fail-fast`로 1회 실행하고 `…native-batch7-integration.md` 작성 |

재개 방법은 8절 TODO 1번에 있습니다.

### 3.3 미착수 (배치 7 이후의 전환 범위)

감사 당시 배치 번호는 실제 분할과 달라졌으므로 번호를 확정하지 않습니다. 순서는 의존 관계 기준의 권장입니다.

| 영역 | 내용 | 근거 문서 |
| --- | --- | --- |
| 편집기 마무리 | 찾기/바꾸기 위젯, 괄호 매칭·쌍 색·가이드, 렌더 옵션(캐럿·공백·rulers 등), 진단 밑줄, sticky scroll, minimap, inlay·code lens·content widget, 접근성 노드, semantic token, diff 편집기 | 설계 문서 7절 단계 6·8~17 |
| LSP 화면 | 완성, hover, signature help, 정의·참조 이동과 peek, rename, code action, 문서·워크스페이스 심볼(팔레트 `@`·`#`) | `…native-audit-editor.md` |
| 사이드바·검색 | files/search/git/outline 뷰 전환, 검색·교체 패널, 검색 편집기 탭, 아웃라인, 탐색기 Git 데코레이션·자동 reveal | `…native-audit-explorer-search.md` |
| Git | SCM 패널, 커밋, 브랜치·stash, diff 렌더러, 히스토리, 그래프, blame·gutter, 충돌 해결, ClaudeDiff와 IDE `openDiff`·`saveDocument` 응답 | `…native-audit-git.md` |
| 셸 상호작용 | 탭·프로젝트 컨텍스트 메뉴, 프로젝트 닫기·그룹 관리, welcome 완성, drag and drop, OS 파일 drop, native 메뉴바 | `…native-audit-shell.md` |
| 설정·기타 | 설정 section 6개(LSP, AI, Plugins, Sync, Remote, Performance), OS 알림 발송, 에이전트 상태 UI, Task Runner, AI 인라인 완성·편집, 스니펫 완성 연결 | `…native-audit-settings.md`, `…native-audit-terminal-agent.md` |
| 보조 창 | eframe 다중 viewport, 창 상태 복원 | `…native-audit-shell.md` |
| 구조 정리 | native workspace 통합과 CI 편입, vendored fork를 `native/vendor`로 이동, 원격·IDE 서버의 crate 추출, `application.rs`·`terminal_surface.rs` 분해, `include!` 테스트 복제 제거 | `…native-audit-architecture.md` |
| 전환 준비 | 기본 데이터 경로·keychain 승계, 단일 인스턴스·CLI 열기, 패키징·서명·공증, 로깅, macOS 외 플랫폼 범위, 전수 재감사, TS 제거 게이트 | `docs/roadmap-rust-native.md` 10절 |

### 3.4 화면 확인 부채

실제 화면으로 확인한 것은 배치 3 종료 시점의 welcome 화면 하나입니다(`docs/quality-assurance/assets/2026-10-06-native-welcome-after-batch3.png`: 창 제목·크기, 자체 타이틀바, 다크 테마, 상태바 확인). 배치 4~7의 글꼴 체인, word wrap, 구문 강조, gutter 폭 변화, 접기 컨트롤, 팔레트, toast, 탐색기·탭 아이콘은 **한 번도 실제 화면으로 보지 못했습니다**. 캡처 시도 때마다 화면이 잠겨 있었습니다(최상위 창 소유자 `ScreenLock`). 캡처 방법은 `docs/memory/2026-10-07-native-transition-operations.md` 5절에 있습니다.

## 4. 의사결정 요약

상세와 기각된 대안은 `docs/acknowledge/2026-10-06-native-transition-decisions.md`입니다.

| 결정 | 채택 | 이유 | 기각된 대안 |
| --- | --- | --- | --- |
| 미커밋 M8 작업 | 체크포인트 커밋 후 검증된 논리 단위로 커밋·푸시 | 미추적 16.7만 줄의 유실 위험과 병렬 구현 제약 | M8 완료 전 commit 보류(이전 합의, 대체됨) |
| 구문 강조 엔진 | `ferriki-textmate =0.12.0` + `ferroni =1.8.1`, 게이트 통과(불일치 0) | VS Code tmLanguage·플러그인·VSIX 문법을 그대로 읽고 Shiki와 같은 색을 냄 | syntect(`.sublime-syntax`만 읽음), syntect-tmlanguage(while·injection 미지원), tree-sitter(TextMate 문법 재사용 불가), giallo(EUPL-1.2, 문서 단위 API) |
| 문법 자산 | `@shikijs/langs 4.4.3`의 30종 + 임베드 7종 전부 포함(회색 지대 4종 포함) | 사용자 답 "4종까지 싹 다 포함" | 회색 지대 제외 |
| 브라우저 Wasm 클라이언트(`native/taide-remote-web`) | 데스크톱 대응 완료 전까지 동결(컴파일만 유지) | 데스크톱 화면보다 원격 경로에 작업이 쏠려 있었음 | 병행 개발 |
| 보조 창 | eframe 다중 viewport로 구현 | TS 보조 창 재현 원칙 | 메인 창 병합 |
| native 메뉴바 | muda 계열 의존성 추가 | TS 앱 메뉴 재현에 필요 | 메뉴 없음 |
| 터미널 오류 표시 | 화면 도색을 없애고 경고 로그 | TS는 write·resize 오류를 무시 | 화면에 영구 문구(이전 구현) |
| 세션 중 테마 전환 | 새 테마로 다시 토큰화 | TS의 색 섞임은 테마 이름 고정에서 오는 결함 | TS 결함 재현 |
| 접기·장식 표면 인자 | 기존 `show` 계열 서명 유지, 새 진입 함수(`show_presented`·`show_tokenized`·`show_request`) 추가 | 기존 UI 테스트 literal과 동결된 remote-web 소비처 보존 | `EditorAppearance` 필드 추가 |
| 빌드 캐시 | 사용자 지시로 `cargo clean`, 단계 작업자는 필요한 대상만 빌드 | 산출물 741GB로 디스크 99% | 방치 |

보조 창과 native 메뉴바는 질문 당시 메인이 추천안을 명시하지 않았던 항목이라, "나머지는 추천대로"라는 답에 근거해 위 값을 적용하고 정정 가능하다고 사용자에게 알렸습니다.

## 5. 사용자 방향성 & 작업 규칙

사용자 전역 규칙은 `~/.claude/convention/*.md`(Codex는 `~/.codex/llm-rules/`)이며 항상 우선합니다. 이번 세션에서 확정된 것만 적습니다.

1. **전환 원칙**: 기존 TS view의 화면·상태·상호작용을 그대로 재현합니다. 새 기능·새 디자인을 추가하지 않습니다. 기준은 TS 소스와 TS가 쓰던 Monaco·Shiki·xterm의 실제 동작입니다. TS에 없는 동작이 필요하면 사용자 결정으로 올립니다.
2. **위임 방식**: 서브에이전트(Agent 도구)를 쓰지 않고 Workflow만 사용합니다. Workflow 모델은 opus와 sonnet만 쓰고 fable은 쓰지 않습니다(2026-10-06 지시). 이번 세션은 구현·설계 opus·xhigh, 리뷰 sonnet·xhigh로 돌렸습니다. 재개 경계마다 workflow 사용 여부는 다시 묻습니다.
3. **속도 제어**: 사용자가 배치 단위로 멈춤·재개를 지시합니다("배치 4까지만", "이번 배치까지만", "싹 다 일시정지"). 사용량 한도 리셋 시각에 맞춰 시작을 지시한 적이 있습니다. "얼마 남았어" 같은 질문은 정보 요청이며 중단 지시가 아닙니다. 명시적 중단 단어 없이는 workflow를 멈추지 않습니다.
4. **현재 유효한 중단 지시**: "배치 7까지만 하고 멈춤"이 유효합니다. 재개해도 배치 7의 검증·기록·커밋·푸시까지만 하고, 배치 8 이후는 사용자가 다시 지시해야 시작합니다.
5. **Git**: 검증된 변경을 논리 단위로 선별 스테이징해 커밋·푸시합니다(`llm-rules.auto-commit`·`auto-push` true). Conventional Commits, 설명은 한국어, AI 트레일러 금지, `git add -A` 금지, force push 금지. 사용자가 "여기까지 commit push"를 지시하면 중간 체크포인트도 상태를 메시지에 밝히고 커밋합니다.
6. **디스크**: 빌드 산출물이 디스크를 채우지 않게 합니다. `Bash(cargo clean *)` 허용 규칙이 프로젝트 설정에 있습니다. `rm -rf`는 자동 모드에서 거부됩니다.
7. **응답**: 한국어 존댓말, 첫 줄에 답 또는 실행 행동, 매 응답에 현재 상태 한 줄, 이모지·아스키아트 금지. 실행하지 않은 검사를 통과했다고 쓰지 않습니다.
8. **보호 대상**: `experiments/native-shell-spike/target/TAIDE M8 Egui Spike.app`·`TAIDE M8 Iced Spike.app`(실기용 번들), 사용자 실제 앱 데이터, OS 설정, 실제 클립보드·Keychain·Trash를 건드리지 않습니다. 화면 캡처에 합성 키·마우스 입력을 쓰지 않습니다.

## 6. 미해결 질문 / 사용자 확인 필요 항목

추천안을 제시했지만 답을 받지 않은 것입니다. 진행을 막지는 않습니다.

1. **찾기 위젯 정규식**(조사 문서 7절, 결정 6건): 추천은 JS 방언 유지와 `regress =0.12.0` 게이트 조건부 채택(새 의존성 1개, 네트워크 내려받기 필요). 찾기/바꾸기 위젯 구현의 선행 조건입니다.
2. **번들 테마의 잘못된 색**: `crates/taide-theme/resources/themes/intellij-islands-light.json:1119`의 `"#0083080"`은 7자리입니다. TS와 native 모두 이 테마에서 토큰 색 적용이 실패합니다. 올바른 값이 필요합니다.
3. **`glsl` 문법 포함 유지**: 상류에 라이선스 표기가 없는 다섯 번째 회색 지대 항목입니다. 기존 웹 번들에도 들어 있어 지금은 포함돼 있습니다.
4. **30만 줄 초과 문서의 저장 정리**: Monaco처럼 모든 줄의 후행 공백을 지우게 구현됐습니다(사용자 파일 변경). 유지 추천.
5. **전역 ⌘N**: `native/taide-native-app/src/application.rs`의 하드코딩 "새 untitled 파일" 단축키. TS에는 전역 ⌘N이 없습니다. 제거 추천.
6. **실행 경로 없는 명령의 노출**: 키바인딩 편집기가 실행되지 않는 명령도 편집 가능한 행으로 보여 줍니다(TS와 같은 무필터). 비활성 표시 추천.
7. **팔레트가 열린 동안의 배경**: 셸 전체가 비활성으로 흐려집니다. TS는 scrim만 덮습니다. 실기 확인 뒤 결정.
8. **status 라벨 56곳**: TS에 대응 표시가 없는 native 내부 오류 문자열입니다. 테마·로케일 실패는 `StatusErrorBanner`, 트리 실패는 인라인 `rootUnavailable` 상태를 구현하면 옮길 수 있습니다.
9. **컷오버 정책**: HTML·미디어 미리보기의 WebView(wry) 의존 유지 여부, macOS 외 플랫폼 지원 범위, 기본 데이터 경로·keychain 서비스명 승계 시점. 아직 묻지 않았습니다.
10. **Monaco 기본 키**: 접기 등 Monaco가 기본으로 처리하던 편집기 키를 native 키맵의 어느 계층에 둘지. 배치 7 단계 2가 다룰 예정이었습니다.

## 7. 환경 & 전제

- macOS arm64, zsh. Rust 1.98.1, native 크레이트 선언 `rust-version` 1.95, edition 2024. TS 쪽은 Bun.
- **native 크레이트는 루트 workspace 밖의 개별 workspace**이며 각자 `Cargo.lock`을 갖습니다(`native/taide-native-{app,ui,editor,syntax,terminal,retained}`, `native/taide-remote-web`). 제품 크레이트가 `experiments/native-shell-spike/vendor/eframe`과 `experiments/terminal-core-spike/vendor/vte`를 path로 참조합니다.
- **cargo 명령 형태**: `cargo <check|test> --manifest-path native/<crate>/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`. 한 번에 하나만 실행합니다. 새 path 의존성으로 lockfile을 갱신할 때만 `--locked`를 빼고 `--offline`으로 1회 실행합니다.
- **앱 lib 테스트의 선행 조건**: LSP 테스트는 `experiments/native-shell-spike/target/debug/examples/native-lsp-mock` 실행 파일이 필요합니다. 빌드 캐시를 비운 뒤에는 `cargo build --manifest-path native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir experiments/native-shell-spike/target`을 먼저 실행합니다.
- **앱 실행**: `experiments/native-shell-spike/target/debug/taide-native-app --data-dir <디렉터리>`. `--data-dir`이 필수이고 keychain 서비스명이 격리용(`net.gumyo.taide.native-isolated`)이라 기존 사용자 데이터와 분리됩니다. 바이너리가 둘이라 `cargo run`에는 `--bin taide-native-app`이 필요합니다.
- **CI**: `.github/workflows/ci.yml`은 `dev`·`main` push와 PR에서만 돕니다. `to_rust_native` push에서는 돌지 않습니다. 루트 `bun run format:check`는 체크포인트 때 들어온 `native/` 아래 JSON 때문에 이 브랜치에서 통과하지 못할 것으로 보입니다(실행해 확인하지 않음).
- **디스크**: 2026-10-09 기준 여유 713GB(62%). 전체 테스트 대상을 한 번 빌드하면 20~50GB가 늘어납니다.
- **Tauri 앱**: 기존 워크스페이스는 `cargo check --workspace --locked --offline` exit 0(2026-10-06 확인). `crates/taide-infra/src/pty.rs`의 `openpty` 재시도는 Tauri 앱도 쓰는 공유 코드입니다.
- **egui 채택의 미해결 게이트**: egui/eframe 0.36.2는 격리 후보이며 최종 채택 승인이 아닙니다. 다중 창, CJK IME 실기, 접근성(편집기 표면에 AccessKit 노드 없음), native 메뉴, 외부 파일 drop, GPU fallback 중 코드로 닫힌 것이 없습니다. 텍스트 계층 한계(리거처를 끌 수 없음, 컬러 이모지·bidi 미지원, 물결 밑줄 없음)는 기록하고 진행 중입니다. 사용자 담당 실기(CJK IME, VoiceOver)는 마지막 순서입니다.
- **문서 수치 주의**: 363/433(83.83%)은 이전 에이전트의 세부 체크리스트 수이고, 212/573(37.0%)은 배치 1 이전 감사 결과입니다. 현재 기능 대응률로 쓰지 않습니다.

## 8. 다음 세션 TODO (우선순위 순)

1. **배치 7 재개** (사용자가 재개를 지시한 경우에만).
    - 먼저 `fbc28b84` 상태가 컴파일되는지 확인합니다(7절의 cargo 명령으로 5개 크레이트 `check --tests`).
    - 단계 1의 차단 항목을 고칩니다: `native/taide-native-editor/src/auto-indent.rs:410` `enclosing_opener_line`에 줄 토큰 종류를 넘겨 Other가 아닌 괄호를 건너뜁니다. 실패하는 테스트로 재현한 뒤 수정합니다.
    - 단계 2·3을 이어서 구현합니다. workflow 스크립트 사본은 `docs/utils/2026-10-07-native-batch7-workflow.js`입니다. 새 세션에서는 `resumeFromRunId`를 쓸 수 없으므로, 단계 1은 이미 구현돼 있고 단계 2는 일부 진행됐음을 작업자 지시에 적어야 합니다(스크립트의 실행 순서 1번이 미커밋 변경 확인만 다루므로 커밋된 진행분을 따로 알려야 합니다).
    - 완료 조건: 변경 크레이트 전체 테스트 대상 `--no-fail-fast` 1회 실행, `docs/quality-assurance/2026-10-06-native-batch7-integration.md` 작성, 커밋·푸시. 그 뒤 멈춥니다.
2. **화면 확인**: 화면이 켜진 상태에서 프로젝트를 연 화면을 캡처해 배치 4~7의 외형을 확인합니다(`docs/memory/2026-10-07-native-transition-operations.md` 5절). 보정이 필요하면 별도 배치로 올립니다.
3. **6절의 결정 받기**: 특히 1번(찾기 정규식)은 편집기 마무리 배치의 선행 조건입니다.
4. **배치 8 이후**: 3.3의 "편집기 마무리"부터. 착수 전 사용자 재개 지시가 필요합니다.
5. **기능 대응률 재집계**: 배치 1~7 반영 뒤의 수치가 없습니다. 감사 workflow(`docs/memory/…operations.md` 2절의 구성)를 다시 돌리면 얻을 수 있으나 토큰 비용이 큽니다(1회 약 390만 토큰).

## 9. 문서 지도

| 문서 | 내용 |
| --- | --- |
| `docs/PROCESS.md` | 작업 상태 정본. 맨 위가 2026-10-07 점검과 전환 인수 체크리스트(a~x). 약 1MB이므로 맨 위 절만 읽습니다 |
| `docs/architecture.md` | 시스템 구조 정본. 9절이 Rust-native 전환 구조 |
| `docs/roadmap-rust-native.md` | 전환 Phase 0~9와 삭제 게이트 |
| `docs/acknowledge/2026-10-06-native-transition-decisions.md` | 이번 세션의 사용자 결정, 기각된 대안, 미결 |
| `docs/acknowledge/2026-09-30-m8-code-first-parity.md` | "TS view 재현 우선" 합의(커밋 보류 조항은 대체됨) |
| `docs/quality-assurance/2026-10-06-native-audit-summary.md` | 전수 감사 요약과 영역별 보고서 10개의 입구 |
| `docs/quality-assurance/2026-10-06-native-batch{1..6}-integration.md` | 배치별 통합 검증, 부채, 결정 필요 사항 |
| `docs/quality-assurance/2026-10-06-native-batch7-language-config.md` | 배치 7 단계 1의 구현 기록 |
| `docs/research/2026-10-06-native-editor-display-layer-design.md` | 편집기 표시 계층·구문 강조 설계(909줄, 구현 단계 0~17) |
| `docs/research/2026-10-07-native-find-regex-dialect.md` | 찾기 위젯 정규식 방언 조사와 결정 6건 |
| `docs/memory/2026-10-07-native-transition-operations.md` | 배치 운영 방법, 검증 게이트, 화면 캡처, 반복된 함정 |
| `docs/feedback/2026-10-07-build-cache-disk-usage.md` | 디스크 고갈 지적과 대응 |
| `docs/feedback/2026-10-07-batch-integration-full-test-gate.md` | 통합 검증에서 회귀를 놓친 경위와 규칙 |
| `docs/utils/2026-10-07-native-batch7-workflow.js` | 배치 7 workflow 스크립트 사본 |
| `docs/utils/2026-10-06-extract-shiki-grammars.ts`, `…generate-syntax-reference.ts`, `2026-10-07-extract-monaco-language-configurations.ts` | 문법·기준 자료·언어 구성 추출 스크립트 |
| `THIRD_PARTY_LICENSES.md` | 문법 37개, 두 크레이트, Monaco·Shiki 포팅 고지 |

## 10. 복기 신뢰도

- 배치 1~7의 커밋, 테스트 수, 경로는 Git 이력과 실제 파일로 대조했습니다(경로 34곳 모두 존재).
- 배치 6 이후의 전체 테스트는 다시 실행하지 않았습니다. 3.2의 수치는 `fbc28b84` 커밋 직전 값입니다.
- `bf5175ca`(2026-10-07 18:46)는 다른 에이전트(Codex)의 점검 커밋이며 `docs/PROCESS.md` 맨 위 절만 추가했습니다. 그 점검은 빌드·테스트를 실행하지 않았습니다.
- 단계별 작업자가 보고한 세부 판단(설계와 달라진 점 등)은 각 QA 문서가 정본이며 이 문서에는 요지만 옮겼습니다.
- 이번 세션 대화가 매우 길어, 작업자 보고의 "남은 차이" 목록은 통합 문서에 옮긴 것 외에 누락이 있을 수 있습니다. 단계별 QA 문서의 unresolved 절을 함께 보십시오.
