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

## 전수조사(2026-08-29 Fable)에서 이관된 후속 후보

> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §7. d-50/d-51 즉시 수정분과
> d-52(CI)·d-53(UX 5건+on-save+EditorConfig) 채택분을 제외한 잔여. 소형 이월은 각 배치 계약 §5 참조.

### Rust 성능·설계

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| ~~git status 캐시~~ · rename 방향 축소 (M-2) | 결과 캐시+워처 무효화 재계산, 양방향 rename 감지 축소, guarded index refresh | **결과 캐시는 구현 완료**(사용성 배치 4 C.2-6 ③ — `GitStore::StatusCache`, `fs:changed`/`git:status-changed`/`git:refs-changed` 무효화 + 2초 TTL + 세대 비교). 잔존: 양방향 rename 감지 축소 · `StatusOptions::update_index`(아래 별도 행) |
| git_log refs 캐시·커서 재개 (L-1) | 페이지마다 refs 전체 재구축 + O(skip) revwalk → GitRefsChanged 무효화 캐시·커서 | 태그 수천 규모에서만 체감 — 실수요 확인 후 |
| ~~워처 선택적 IdCache (M-10)~~ | IGNORED_DIR_NAMES 하위를 FileIdMap 워크에서 제외하는 커스텀 IdCache | **구현 완료**(사용성 배치 4 C.2-6 ② — `infra::watcher::ScopedIdCache`. d-35 의 전면 `NoCache` 기각 사유는 보존: 앱이 보고하는 모든 경로쌍의 rename 짝짓기가 유지된다. `docs/architecture.md` §2.3). 항목 종결 |
| 트리 mutation 응답 재설계 (H-4 후반) | full_page 전체 반환 → revision+영향 구간 축소 | IPC 계약 변경 — FE 동시 개정 필요 |
| ide:diff-requested pull 전환 (L-4) | 파일 전문 이벤트 운반 → request_id 신호+query pull | MCP 흐름 동시 수정 |
| search_list_files FsChanged 캐시 | 팔레트 열 때마다 전체 re-walk → 무효화 기반 캐시 | d-42 무상한 계약은 불변 — 캐시만 |
| 비 UTF-8 인코딩 왕복 지원 | d-50 은 lossy 읽기 전용 차단까지 — 인코딩 감지·보존 열기/저장은 별도 | 기능 신설 규모 |

### UX 중형·대형

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| 로컬 히스토리 / 타임라인 | git 커밋 사이 저장 스냅샷 안전망 — AI 대량 수정 IDE 라 가치 특대 | 저장 훅+스냅샷 저장소+패널 설계 |
| DocumentColor 색 데코레이터 | LSP colorProvider 어댑터 + monaco colorDecorators | 어댑터 20종 중 유일 누락 축 — 소형이나 d-53 범위 밖 |
| Linked Editing (태그 쌍 동시 개명) | vtsls linkedEditingRange + linkedEditing 옵션 | 어댑터 1개 — d-53 범위 밖 |
| 탭 tear-off 드래그 | 탭을 창 밖으로 드래그 → 신규 OS 창 (커맨드는 기존) | dnd-kit 드롭 좌표 제스처 설계 |
| vim 모드 | monaco-vim 도입 + 상태바 모드 표시 | 신규 의존성 — 사용자 합의 선행 |
| Multibuffer (Zed 시그니처) | 여러 파일 발췌를 하나의 편집 가능 뷰로 | monaco 무대응 — 자체 설계 최상 난이도 |
| Call Hierarchy | 호출 계층 트리 UI + LSP 어댑터 | monaco 미내장 — outline 급 자체 UI |

### 차별화 (AI IDE 정체성 — 사용자 선별 대기)

| 항목 | 내용 |
|------|------|
| 병렬 에이전트 워크트리 오케스트레이션 | git worktree 생성→프로젝트 자동 열기→터미널별 Claude 병렬→배지 관찰→diff 비교 머지 (기존 도메인 조립형) |
| AI diff 리뷰 | diff 뷰에 "이 변경 설명·리뷰" — instruct provider 재사용 |
| Problems → Inline Edit 파이프 | 진단 우클릭 → ⌘I 에 진단 메시지 프리필 |
| AI 편집 체크포인트 | 에이전트/Inline Edit 대량 수정 전후 자동 스냅샷+원클릭 롤백 (로컬 히스토리의 AI 특화) |
| MCP 서버 매니저 | 프로젝트 .mcp.json 등록·상태·로그 GUI |
| PROCESS.md 워크플로 패널 | docs/PROCESS.md 체크리스트를 사이드바 진행률·토글로 |

### 기지 유지

| 항목 | 상태 |
|------|------|
| 팔레트 files fuzzy 전량 스캔 (L1-03) | d-42 §5 이월 — 실측 후 판단 결정 유지 |

### d-50/d-51 구현 검토(2026-08-29)에서 분리된 후속 후보

| 항목 | 내용 |
|------|------|
| 검색 결과·Problems 목록 키보드 도달성 | perf-4·perf-12 가상화로 뷰포트 밖 행이 DOM 에 없어 Tab 순회가 첫 화면 분량에서 끊긴다. `features/explorer/file-tree.tsx` 의 컨테이너 로빙 포커스(`role='tree'` + 화살표 이동 + `scrollToIndex`)를 `search-results-list`·`problems-panel` 에도 적용한다. 두 목록의 행은 이미 `role='button' tabIndex={0}` 이라 개별 tabstop 을 로빙으로 바꾸는 작업이 함께 필요하다. |
| 키바인딩 충돌 판정의 위임 쌍 제외 | D3(monaco 기본 바인딩 인지) 도입 후, 앱 `find` 와 `monaco.actions.find` 가 같은 ⌘F 를 광고한다는 이유로 **아무 재바인딩도 하지 않은 기본 설치 상태**에서 충돌 1쌍이 상시 표시된다. 사실 관계로는 참이지만 사용자가 만든 실제 충돌과 구분되지 않아 경고 신뢰도를 낮춘다. 위임 관계(앱 액션이 에디터 포커스 시 monaco 액션으로 넘기는 쌍)를 카탈로그 데이터로 표기해 판정에서 빼고 "기본 상태 경고 0" 을 목표로 한다. |
| 삭제로 닫힌 탭의 닫은 탭 스택 정리 | `close_file_tabs_under` 는 삭제로 닫는 탭도 `close_tab` 을 그대로 태워 닫은 탭 스택에 적재한다(개명 축은 스택을 새 경로로 옮기는 것과 비대칭). ⇧⌘T 가 이미 지워진 파일을 열어 오류 탭이 된다. 스택에서 빼는 것이 옳은지(재열기 대상이 아님) 아니면 재열기 시점에 존재 검사를 하는지 결정이 필요하다. |
| 개명 시 다른 창의 라이브 버퍼 이관 | 창마다 모델 레지스트리·쿼리 캐시가 별개라, 보조 창에서만 편집 중이던 파일을 다른 창에서 개명하면 그 창의 화면 버퍼는 새 경로의 디스크 내용으로 리셋된다(초안 자체는 개명 전 미러 재조회로 새 경로 미러에 보존 → 재시작 복구는 가능). 창 간 이관 브로드캐스트 채널 설계가 필요하다. |

### 파일트리 git 데코레이션(2026-08-29)에서 분리된 후속 후보

| 항목 | 내용 |
|------|------|
| 파일트리 ignored 흐림 표시 | git status 는 ignored 파일을 나열하지 않아 현재 데코레이션(색·뱃지)으로는 판정 불가. 보이는 디렉토리 범위의 ignore 판정 IPC(git2 `status_should_ignore` 또는 check-ignore 배치)를 신설해 `FileTreeGitStatus 'ignored'`(렌더 경로는 기구현)를 채운다. |
| 터미널 kitty keyboard protocol | xterm.js 미지원으로 Shift+Enter 를 LF 매핑으로 우회 중(`acknowledge/2026-08-29-terminal-shift-enter-decision.md`). 프로토콜 구현 시 TUI 전반의 조합 키 구분이 근본 해결되나 키 인코딩 전반 변경의 회귀 리스크가 커 별도 배치로만 착수한다. |

### 퀵오픈 스테일 인덱스 수정(2026-09-04)에서 분리된 후속 후보

| 항목 | 내용 |
|------|------|
| 팔레트 `target:null` → focusedPane 소실 시 NotFound 토스트 | 파일 탭 열기는 `target: null` 로 "현재 포커스된 pane" 을 요청하는데, 그 `focusedPane` 이 이미 없으면 `layout/service.rs` 가 `NotFound("pane not found")` 를 돌려준다. 파일 자체는 멀쩡한데 `error.file.notFound` 와 같은 `NotFound` 코드라 `useOpenFileTab` 이 파일이 사라진 것으로 읽어 같은 토스트를 띄우고 퀵오픈 인덱스까지 무효화한다(무해하지만 불필요한 walk 1회). pane 부재는 파일 부재와 다른 코드/로케일 키로 분리하거나, 서버가 소실된 pane 을 유효한 pane 으로 대체(fallback)하도록 정한다. |

### 터미널 컨텍스트 메뉴(사용성 배치 4, 2026-09-04)에서 이월된 후보

> 계약 §F 가 "메뉴에 넣지 않는다"로 확정한 항목들. 정본 `docs/features/terminal.md` §6.2·§7·§8.

| 항목 | 내용 |
|------|------|
| 터미널 검색 UI (`⌘F`) | `SearchAddon` 은 `terminal-view.tsx` 가 로드만 하고 `findNext`/`findPrevious`/검색 바 참조가 0건이다. 결과 카운트(`onDidChangeResults`)·overviewRuler 하이라이트·터미널 포커스 한정 `⌘F` 게이트(키맵 화이트리스트 등록 동반)까지가 한 묶음이라 컨텍스트 메뉴 항목만 먼저 넣을 수 없어 제외했다. |
| 터미널 탭 이름 바꾸기 | 탭 제목을 바꾸는 커맨드 자체가 없다(`layout_rename_tab`/`set_title` 부재). 신규 Rust 커맨드 + 인라인 편집 UI 가 선행이라 메뉴 항목만 추가할 수 없다. 탭 바 쪽 "이름 바꾸기"(계약 §G.2-4)는 파일 탭을 탐색기 rename 으로 위임하는 별개 경로다. |
| 터미널 종료 확인 다이얼로그 | 현재 "터미널 종료"는 일반 탭 닫기와 동일하게 즉시 닫는다(`layout_close_tab` 이 pty 회수). 실행 중인 명령이 있을 때만 묻는 VS Code 식 확인은 "실행 중" 판정(OSC 133 블록 상태) + 설정 토글 + 3언어 로케일이 필요해 별도 건으로 분리한다. |
| "이 터미널 옮기기" 메뉴 항목 | 조사 안 C(하이브리드)의 하단 그룹. 지금도 탭 우클릭 Split 서브메뉴·`⌘\` 로 4방향 이동이 되고 탭 바 여백 메뉴에도 분할이 들어가, 터미널 표면에까지 놓으면 4중 중복이 된다. 서브메뉴가 8칸으로 늘어나는 비용도 크다. |
| `@xterm/addon-clipboard`(OSC 52) | 셸이 OSC 52 로 요청하는 클립보드 쓰기 지원. 애드온이 설치돼 있지 않고(설치 addon 5종), Tauri clipboard-manager 위임 provider 설계가 선행이다. 컨텍스트 메뉴의 복사/붙여넣기는 `navigator.clipboard` + `term.paste()` 로 이미 동작한다. |

### OS 네이티브 알림(사용성 배치 4 A, 2026-09-04)에서 이월된 후보

> 계약 §A.2-8 이 지정한 등재분. 정본 `docs/features/settings-ui.md` §2.1 ·
> `docs/ipc-contract.md` "사용성 배치 4 — OS 네이티브 알림" 절.

| 항목 | 내용 |
|------|------|
| 권한 상태 조회 · 거부 감지 | `tauri-plugin-notification` 의 데스크톱 백엔드는 `permission_state()`/`request_permission()` 이 항상 `Granted` 스텁이고 `show()` 실패도 삼킨다 — 사용자가 macOS 설정에서 TAIDE 알림을 꺼도 앱은 알 수 없다. 그래서 `NotificationDelivery::Delivered` 는 "전달됐다"가 아니라 "시도했다"이고, 설정 화면은 "알림 설정 열기" 버튼으로 사용자에게 확인을 미룬다. 실제 권한 조회는 `UNUserNotificationCenter` 를 직접 부르는 크레이트(또는 objc2 바인딩) 도입이 선행이다. |
| `UNUserNotificationCenter` 전환 | 위 항목의 상위 건. 권한 상태·전달 실패 콜백뿐 아니라 액션 버튼·응답 처리(알림 클릭 → 해당 탭 포커스)까지 이 API 가 있어야 열린다. 현재 `notify-rust`→`mac-notification-sys` 경로는 번들 서명된 앱에서만 이름·아이콘이 정상이고(dev 는 `com.apple.Terminal` 스푸핑), 클릭 콜백을 앱으로 되돌리지 못한다. |
| 알림 클릭 → 원 지점 이동 | 완료 알림을 눌렀을 때 그 에이전트·터미널·git 패널로 포커스를 옮기는 동작. 위 전환이 선행이고, 카테고리별 딥링크(프로젝트 id + 탭 id) 페이로드 설계가 필요하다. |

### 프로젝트 표시 설정(사용성 배치 4 D, 2026-09-04)에서 이월된 후보

> 계약 §D.2-4 가 지정한 등재분. 정본 `docs/features/layout-shell.md` §2.2 · `docs/data-model.md` §20.

| 항목 | 내용 |
|------|------|
| Welcome 최근 목록 · 보조 창 타이틀바에 표시 설정 반영 | 1차 범위는 **1차 사이드바 아이콘만**이다. Welcome 화면의 최근 프로젝트 목록과 보조 창 타이틀바(`auxiliary-title-bar-content.tsx`)는 이미 `Project`/`ProjectRef` 를 받으므로 추가 IPC 없이 `resolveProjectDisplay` 를 붙일 수 있지만, **표시 전용 필드 변경에 `ProjectListChanged`(프로젝트 목록 전체 fanout)를 재발행하는 현재 방식이 그 창들까지 갱신하는지**를 먼저 확인해야 한다. 필요하면 좁은 `ProjectDisplayChanged { projectId, display }` 이벤트를 신설한다(현재 신규 이벤트 0). |
| PRD FR-A4(포커스된 content 타입에 따른 아이콘 변경) | 사이드바 아이콘 슬롯을 이번 표시 설정과 다툰다 — 사용자가 지정한 아이콘·라벨을 런타임 상태가 덮어쓰면 설정의 의미가 사라진다. `PRD.md` FR-A4 에 보류 각주를 달아 두었고, 되살린다면 "사용자 지정이 없을 때만 content 타입 아이콘" 같은 우선순위 규칙을 먼저 정해야 한다. |
| 라벨 타이포 사다리 · lane 색 대비 실측 | `PROJECT_LABEL_CLASS_BY_LENGTH` 는 CJK 4자가 40px 버튼에 들어가도록 계산한 값이고, 색 12종은 `graph.lane1..12` 를 그대로 쓴다. 번들 테마 36종에서 `appSidebar.background` 대비가 충분한지, 4자 라벨이 실제로 잘리지 않는지는 앱 실행 확인이 필요하다. |

### 성능 극한 최적화(사용성 배치 4 C, 2026-09-04)에서 이월된 후보

> 계약 §C.2-8 이 지정한 등재분 + 조사 §9(`research/2026-09-04-batch4-topics1-5-research.md` 주제
> 3a·3b)가 "이번 배치 제외" 로 확정한 것. **감사 §1(d-51 계약 §5 F7)의 이월분이 이 백로그에 없어
> "이미 처리됨" 으로 오독되던 문제**를 여기서 해소한다. 측정 절차 정본은
> `docs/quality-assurance/2026-09-04-perf-baseline.md`.

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| monaco 지연 로딩 + "에디터 없는 첫 프레임" 셸 | `shared/lib/monaco/setup.ts` 의 `import * as monaco` 를 **27개 모듈이 값으로** import 해 React 렌더 이전에 3.76MB JS + 163kB CSS 를 파싱한다(부팅 페이로드의 64%). 터미널만 여는 세션도 전액을 지불한다. | 효과가 이 축에서 가장 크지만 **구조 수술**이다 — 접촉 파일 30+, 부팅 순서 계약 신설, 첫 페인트 게이트(`use-reveal-window.ts`)·shiki·키맵 런타임·LSP 어댑터가 전부 monaco 인스턴스에 줄 서 있어 실패 모드가 d-20 blank-window 급이다. 2차 결정 §7 에서 **별도 배치**로 확정. 착수 시 지표 1 재측정 필수 |
| `ts.worker` 청크(6.8MB) 축소 | TypeScript 파일을 처음 열 때 받는 워커 자산 크기 자체. 부팅 페이로드에는 잡히지 않는다(`MonacoEnvironment.getWorker` 가 요청 시점에 만들어 이미 지연). | monaco 구성 축 별건 — 위 항목의 하위 |
| grammar 코어 3종 선정을 실측 기반으로 | 현재 코어 3종은 "앱이 사용자 동작 없이 스스로 여는 표면" 기준의 **판단값**이다. 세션 복원으로 열리는 파일(대개 ts/tsx/rs)은 온디맨드 경로라 하이라이팅이 한 박자 늦게 붙는다. | 옳은 해법은 코어 목록 확대가 아니라 **직전 세션이 요구한 언어를 영속**해 부팅 시 선로드하는 것 — 새 저장 표면이 필요 |
| shiki tokens provider 전량 재부착 | `shikiToMonaco` 가 언어 1개만 등록하는 API 를 노출하지 않아, 새 언어가 붙을 때마다 로드된 전 언어분을 다시 등록하고 직전 묶음을 dispose 한다. | 서로 다른 언어의 파일을 **처음** 여는 횟수만큼만 일어나고 그 뒤로는 발생하지 않아 수용 중. 상류 API 변화 대기 |
| 마커 재수집 — `ide-sync-provider` 전량 티어 | C.2-7 로 상태바는 severity 카운트 티어(`useMonacoMarkerCounts`)로 내려왔으나, `app/providers/ide-sync-provider.tsx` 가 앱 루트에서 세션 내내 **전량 티어**(`useMonacoMarkers`)를 구독한다 — `onDidChangeMarkers` 마다 `getModelMarkers({})` 전량 순회가 여전히 armed 다. | 그 프로바이더는 마커를 `ideStatus?.running` 일 때만 쓰므로 **구독 자체를 그 조건으로 게이팅**하면 LSP 초기 인덱싱 구간의 잔여 비용이 사라진다. 소형이나 IDE 동기화 계약(`docs/features/ide-integration.md`) 확인이 선행 |
| `poll_agents` 의 `ps` fork 제거 (M-3 잔여) | pid 일괄 배칭·세션 0 조기 반환은 적용됐고 **프로젝트당 500ms 1회 fork 가 잔존**한다. `sysinfo` 로 교체하면 fork 가 사라진다. | 활동 판정 문자(`ps -o state` 의 R/S)가 `ProcessStatus` 의미로 바뀌어 **에이전트 배지 회귀 위험**이 있다. 2차 결정 §7 추천대로 이번 배치 제외 — `perf_snapshot` 계측으로 실제 비용을 먼저 확인한 뒤 판단 |
| libgit2 `StatusOptions::update_index` 채택 | 켜면 stat-stale 파일 재해시가 1회로 끝나 반복 status 가 크게 빨라진다(`status.c:315` `GIT_DIFF_UPDATE_INDEX`). | 매 status 가 `.git/index` 를 되쓰게 되어 **외부 git 프로세스와 락 경합**이 생기고, 자기 쓰기가 다시 `git/watch.rs` 의 Status 무효화를 유발하는 **피드백 루프** 위험이 있다. C.2-6 ③ 결과 캐시로 반복 호출은 이미 흡수됐으므로 실측 후 재판단 |
| `git_status` 단일 비행(single-flight) | 현재 캐시는 "직렬로 이어지는 조회" 만 합친다. 창 N개가 같은 이벤트로 **동시에** 조회하면 전부 miss 로 시작해 N회 계산이 남는다. | 계약 §C.2-6 이 "워처 무효화 + 2초 TTL" 로 범위를 확정했다. 프로젝트별 async 단일 비행 락이 다음 후보 |
| `.git` 워처 캐시의 `objects/**` 프루닝 | 하류 `classify_git_change` 가 `objects/**` 를 전부 떨구므로 캐시에서 프루닝해도 짝짓기 손실이 없고, 루스 오브젝트가 많은 저장소에서 절감이 크다. | 계약 §C.2-6 ② 가 `WatchScope::GitDir` 는 무필터로 확정 — 계약 밖 신규 결정이라 보류 |
| sniff 공유 리더의 `infra` 승격 | `domain/file/service.rs::read_text_bytes` 와 `domain/search/service.rs::read_scannable_bytes` 가 같은 형태의 "sniff 창 = 첫 청크" 리더를 각자 갖고 있다(`is_binary`/`BINARY_SNIFF_BYTES` 중복도 선행 상태). | 소비처가 2곳이 되는 시점이 승격 시점이다 — 배치 4 는 file 쪽만 소유해 한쪽만 고쳤다 |
| `read_children` 의 `directory_has_children` 지연화 | 엔트리 타입 판정은 `file_type()` 로 내렸으나(C.2-5), 하위 디렉토리마다 `read_dir` 1회는 남는다 — 디렉토리 D개면 D회 추가 open/getdirentries/close. | `Entry.has_children` → `TreeRow` 의 의미를 바꾸므로 "안전 수정(회귀 위험 0)" 범위 밖 |
| 트리 mutation 응답 재설계 (H-4 후반) | 위 "Rust 성능·설계" 표에 이미 있음 — `full_page` 전체 반환 → revision + 영향 구간 축소. | `docs/ipc-contract.md` 가 트리 응답 형태를 **불변**으로 못박고 있어 FE 동시 개정이 선행 |
| 팔레트 결과 가상화 | C.2-7 에서 **미적용 확정**(계약이 허용한 폴백). cmdk 1.1.1 이 가상화를 지원하지 않고(README "Good performance up to 2,000-3,000 items"), 방향키 이동·선택 유지·`CommandEmpty` 카운트가 전부 마운트된 항목 수에 묶여 있다. 현재 상한 200 은 상류 권장 구간의 1/10. | 재검토 조건은 `FILE_RESULT_LIMIT` 을 1000 이상으로 올리거나 cmdk 가 가상화를 지원할 때. 사유 정본 `docs/features/command-palette.md` §4.1 |
