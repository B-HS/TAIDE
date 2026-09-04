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
| git status 캐시·rename 방향 축소 (M-2) | 결과 캐시+워처 무효화 재계산, 양방향 rename 감지 축소, guarded index refresh | 캐시 설계·동작 변경 판단 수반 — 대량 dirty 실측 후 |
| git_log refs 캐시·커서 재개 (L-1) | 페이지마다 refs 전체 재구축 + O(skip) revwalk → GitRefsChanged 무효화 캐시·커서 | 태그 수천 규모에서만 체감 — 실수요 확인 후 |
| 워처 선택적 IdCache (M-10) | IGNORED_DIR_NAMES 하위를 FileIdMap 워크에서 제외하는 커스텀 IdCache | d-35 NoCache 기각 사유(rename 오분류) 보존 필요 — 난이도 L |
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
