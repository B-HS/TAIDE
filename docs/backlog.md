# 백로그 — 3순위 후보 (Phase 7.5 이후 재검토)

> 2026-08-06 확정. **1·2순위가 전부 끝난 뒤 다시 검토한다.**
> 지금 구현하지 않는다. 우선순위·범위는 재검토 시점에 다시 정한다.
> 1·2순위 목록과 진행 상황은 `docs/roadmap.md` Phase 7.6.

## 판단 근거

전부 "있으면 좋지만 없어도 IDE 로 쓸 수 있는" 것들이다.
1·2순위(찾기/바꾸기·진단·브랜치·아웃라인 등)는 **코드를 읽고 고치는 핵심 루프**라 먼저 간다.

## 후보 목록

| 항목 | 내용 | 선행 조건 / 메모 |
|------|------|------------------|
| ~~멀티 윈도우~~ | Move into New Window, 새 OS 창으로 탭 분리 | **구현 완료 확인**(2026-08-16 Wave I — 사용자 지시로 "MVP 가 아니라 완전 구현" 격상. `domain::window`+레이아웃 `auxiliary_windows` 축+`layout_move_tab_to_window`+창별 close/hot-exit+LSP·pty 다중 구독+capability 글로브. `docs/features/layout-shell.md` §7). 항목 종결. **`Copy into New Window` 만 미구현으로 신규 하위 항목화**(같은 탭을 두 창이 공유하는 별도 동기화 설계 필요, 이번 범위 밖) |
| ~~Zen / 포커스 모드~~ | 사이드바·탭바·상태바 숨김 토글 | **구현 완료 확인**(2026-08-16 Wave I — `layout_set_shell_view`+`ShellViewState`+⌘K Z chord+Esc 복귀+전체화면/상태바 설정 2종. `docs/features/window-chrome.md` §6). 항목 종결 |
| **Copy into New Window** | 같은 탭을 두 OS 창이 동시에 열어 상태를 공유 | 멀티 윈도우(위 항목)에서 분리된 잔여 항목. `Tab` 자체를 두 트리가 참조하는 동기화 설계가 필요해 Wave I 범위에서 제외됐다 |
| ~~스니펫~~ | 사용자 정의 코드 조각 + Monaco completion 기여 | **구현 완료 확인**(2026-08-15 Wave F — `entities/snippet`·`widgets/snippet-editor`·`docs/features/editor.md` §10). 항목 종결 |
| ~~작업 러너 패널~~ | `package.json` scripts / `Cargo.toml` / Makefile 감지 후 실행 | **구현 완료 확인(2026-08-28 감사)** — `domain/task` + `widgets/task-runner` |
| **git 충돌 해결 UI** | 3-way merge 뷰 | Monaco DiffEditor 는 있으나 3-way 는 별개. `git.md` 2차 범위 |
| ~~임의 두 파일 비교~~ | 파일 선택 → diff 탭 | **구현 완료 확인**(2026-08-13 갭 분석 — `src/widgets/explorer/explorer-container.tsx:254`). 항목 종결 |
| **북마크 / 북마크 패널** | 줄 단위 북마크 + 목록 | 영속화 위치(project.json vs 별도 파일) 결정 필요 |
| **에디터 설정 확장** | ~~word wrap · minimap 토글 · 줄 번호~~ · 들여쓰기 가이드 | word wrap·minimap·줄 번호는 **구현 완료 확인(2026-08-28 감사)** — settings 필드(`editor_minimap`·`editor_word_wrap` 등)+설정 UI. 잔여는 들여쓰기 가이드 토글뿐 |
| **접근성** | 키보드만으로 전체 조작, 포커스 트랩, 스크린리더 라벨 | Radix 가 상당 부분 제공. 전수 점검이 필요한 성격 |
| **pptx LibreOffice 폴백** | 외부 `soffice` 감지 후 PDF 변환 | **제외 확정**(2026-08-06). 재검토 시에만 되살린다 |
| ~~remote-control~~ | 웹에서 프로젝트 접근 | ~~보류 확정~~ → **W6(2026-08-12)부터 범위 재합의 후 구현 완료** — `domain/remote` WS 서버·비밀번호 인증·허용/거부 정책(d-38). 정본: `features/remote-control.md` |

## Phase 7.7 검토에서 보류된 후속 후보 (2026-08-07)

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| hooks override 세션 단위 정밀화 | 현재 hook 이벤트는 에이전트 이름 단위 override — 같은 프로젝트의 동일 이름 다중 세션은 뭉뚱그려짐 | Claude Code hook 페이로드에 TAIDE 세션 식별자를 실을 방법이 없어 설계 재논의 필요 |
| `agentHooksEnabled` OFF 시 주입분 자동 제거 | 현재는 리스너 종료+override 무시까지만. `.claude/settings.local.json` 의 훅 항목은 남는다 | 사용자 파일을 지우는 파괴적 동작 — 확인 UX 합의 선행 |
| hooks/IDE 로컬 리스너 동시 연결 상한 | 타임아웃 도입으로 누적은 유한해졌으나 상한(semaphore)은 없음 | 상한값이 제품 판단 사항 |
| `AgentHooksStatus` 에 "재설치 필요" 상태 노출 | URL 자동 재주입(자가 치유)으로 실사용 문제는 해소됨 | IPC 계약+i18n+UI 확장이라 실수요 확인 후 |
| Monaco 스크롤바 색 통일 | 오버레이 스크롤바와 시각 통일 (`editor.updateOptions` + CSS) | 별건 작업 |
| 스프레드시트 시트탭·마크다운 내부 pre 의 오버레이 스크롤바 | 15표면 목록 밖 소품 표면 2곳 | 후속 소품 |
| 탐색기 붙여넣기 메뉴 사전 비활성화 | 자기 하위 붙여넣기는 Rust 가 거부(토스트) — 메뉴 disabled 조건은 미적용 | 데이터 손상은 이미 차단됨 |

## Phase 7.8 검토에서 보류된 후속 후보 (2026-08-07)

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| untitled 루트 밖 저장 UX | 저장 다이얼로그는 전 파일시스템을 여는데 루트 밖은 root_guard 가 거부(에러 토스트) | 프로젝트 밖 저장 허용 여부가 보안·UX 결정 사항 |
| 테마 크레딧 UI | ResolvedTheme author/license/source 와 `settings.themeCredits` 등 i18n 6키가 미노출 — `settings.themeCredits` 키는 T2-I 미참조 정리에서 카탈로그·MESSAGE_NAMESPACES 로부터 제거됨(구현 시 3언어+스키마 재추가 필요) | 노출 위치(ThemePicker 카드/프리뷰 헤더) 설계 결정 필요 |
| Monaco syntax 토큰 19종 미전달 | 번들 테마가 에디터 구문색에서 VSCode 원본보다 단조 | QA 8 범위 밖 — 실기 확인 후 판단 |
| 라이브 프리뷰 되돌림 의혹 | 검토가 high 로 보고했으나 수정자가 React Compiler 실컴파일로 오탐 판정(반환 객체 memo 확인) | 실기에서 재현되면 다른 원인(window focus refetch 등) 조사 |

## 재검토 시 확인할 것

- 1·2순위 구현 중 이 목록의 선행 조건이 이미 충족됐는지 (예: layout 필드 추가 여부)
- 실사용에서 실제로 아쉬웠던 것이 무엇인지 — 목록 순서보다 그것이 우선한다

## Phase 7.10 결정에서 분리된 후속 후보 (2026-08-11)

> TextMate 문법 엔진은 2026-08-11 사용자 확정으로 **7.10-W7 로 승격**되어 이 목록에서 제외됐다.

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| Gemini IDE companion | HTTP MCP 서버 + discovery 파일로 openDiff·컨텍스트 공유 (공식 스펙 존재) | domain/ide 급 증분(HTTP 트랜스포트 신설) — hooks 배지(W4)와 분리 |
| Codex app-server 패널 | `codex app-server` JSON-RPC 클라이언트로 에이전트 UI 내장 | 터미널 연동과 다른 제품 축. auto-tab(Codex 토큰)과 별개 |
| chord·when 키맵 엔진 | ⌘K ⌘S 류 2단 조합 + 컨텍스트(when) 평가 | keymap 타입·매칭·overrides 직렬화 전면 개정 필요 |
~~앱데이터 파일 에디터 편집~~ | 설정/프롬프트 파일을 에디터 탭에서 직접 편집·저장 | **구현 완료 확인**(2026-08-16 Wave I — `TabKind::AppFile`+`app_file_read`/`app_file_write`(root_guard 무접촉, 전용 화이트리스트 가드)+`SettingsChanged` 이벤트. `docs/data-model.md` §8). 항목 종결 |

## 7.10-W7 결정에서 분리된 후속 후보 (2026-08-12)

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
~~VSIX `contributes.grammars` 임포트~~ | .vsix 에서 TextMate 문법 + languages 기여를 함께 추출해 신규 언어를 추가 | **구현 완료 확인**(2026-08-16 Wave I — `vsix_import_plugin`+`infra::language::language_id_for_path`(확장자→언어 테이블 오버레이, D1)+monaco 동적 언어 등록·shiki 재생성(C1). `docs/features/plugins.md` §6). 항목 종결 |
| `.tf`→`terraform` / `.mdx`→`mdx` grammar 정밀화 | 현재 `.tf` 는 shiki `hcl` grammar, `.mdx` 는 `markdown` grammar 로 매핑된다. shiki 에는 각각 전용 `terraform`(hcl 과 별도, MPL-2.0)·`mdx`(MIT) grammar 가 존재해 더 정밀한 토큰화가 가능하다 | W7 범위 밖의 동작 변경(기존 매핑을 바꾸는 것) — 별도 검토·사용자 확인 후 진행 |

## QA6 후속에서 분리된 후속 후보 (2026-08-12)

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| 에디터 한글 자모 분리 — 상류 대응 | monaco 재현 이슈 제출(+WebKit #274700 케이스 첨부), `monaco-editor` latest 변경 또는 vscode `editContext/textArea/**` IME 커밋 등장 시 재검토. 실험 옵션 `accessibilitySupport: 'off'`(시드 축소) 는 미검증 가설 | 원인이 WebKit 계층(EditContext 미구현 #269922·한글 조합 이벤트 결함 #274700, 전부 미수정)이라 앱 레벨 근본 수정 불가. monaco 0.56.0 이 최신 — 업그레이드 대상 부재. 상세: `docs/bug/2026-08-12-editor-korean-ime.md` |
| shift+기호 키 캡처 정밀화 | 키 바인딩 캡처가 `event.key` 기반이라 shift+`=` 가 `+` 로 기록되는 등 레이아웃·IME 의존. VS Code 식 물리 코드(`event.code`) 기반 전환 검토 | 단축키 모달(QA6 후속)의 범위 밖 — 매칭·직렬화 전면 개정 필요, chord/when 엔진과 함께 다루는 것이 효율적 |

## 기능 확장 3차에서 분리된 후속 후보 (2026-08-14)

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| 프로젝트 스코프 원격 세션 | 프로젝트별 비밀번호로 들어온 세션은 그 프로젝트만 미러(진짜 격리) | 커맨드 90+곳·이벤트 22종 필터링 재설계 = 게이트의 10배 규모 + "상태 완전 공유" 확정 계약 번복. 사용자 결정(2026-08-14)으로 전역 비밀번호 1개만 채택, 스코프는 별도 안건 |
| Remote 하드닝 잔여 | 개별 세션 TTL 만료의 라이브 WS 단절(현재는 전체 revoke 만 즉시 단절), Host 허용목록(DNS rebinding 방어), 비밀번호 최소 길이·trim 정합, 잠금의 전역 카운터 DoS 완화, stale nonce 실패 집계 제외, X-Forwarded-Proto 검증, gist 인바운드 patch 의 게이트 필드 필터 | 검토 웨이브 미검증 오버플로(minor 등급) — 2요소+백오프+fail-closed 로 핵심 방벽은 확보된 상태. 상세: docs/PROCESS.md 기능 확장 3차 절 |
| Hot Exit 미세 항목 | 저장 직후 디바운스 발화로 미러 부활 가능성(락 직렬화로 실발생 미확인), 미러 쓰기의 begin_mutation 락 빈도(500ms·대용량), HotExitFlushRequested.timeoutMs payload 프론트 미사용 | 실기 QA 에서 증상 관찰 후 판단 — 코드상 가능성만 확인된 항목들 |
