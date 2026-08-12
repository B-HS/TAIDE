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
| **멀티 윈도우** | Move/Copy into New Window, 새 OS 창으로 탭 분리 | 요구 7번과 탭 context menu 4항목(Move/Copy into New Window, Reveal in Explorer View 일부)의 전제. `capabilities/main.json` 이 단일 `main` 창 전제라 권한 구조부터 손봐야 한다. **규모가 크고 다른 기능의 전제가 아니라서 뒤로 미뤘다** |
| **Zen / 포커스 모드** | 사이드바·탭바·상태바 숨김 토글 | layout 도메인에 표시 상태 필드 추가 필요 |
| **스니펫** | 사용자 정의 코드 조각 + Monaco completion 기여 | LSP completion 어댑터와 우선순위 조정 필요 |
| **작업 러너 패널** | `package.json` scripts / `Cargo.toml` / Makefile 감지 후 실행 | 터미널 세션 재사용. pty 도메인 위에 얹으면 된다 |
| **git 충돌 해결 UI** | 3-way merge 뷰 | Monaco DiffEditor 는 있으나 3-way 는 별개. `git.md` 2차 범위 |
| **임의 두 파일 비교** | 파일 선택 → diff 탭 | `DiffPane` 재사용 가능. 파일 선택 UX 만 필요 |
| **북마크 / 북마크 패널** | 줄 단위 북마크 + 목록 | 영속화 위치(project.json vs 별도 파일) 결정 필요 |
| **에디터 설정 확장** | word wrap · minimap 토글 · 들여쓰기 가이드 · 줄 번호 스타일 | 현재 `code-editor.tsx` 에 **하드코딩**돼 있다. settings 필드 추가 + 설정 UI |
| **접근성** | 키보드만으로 전체 조작, 포커스 트랩, 스크린리더 라벨 | Radix 가 상당 부분 제공. 전수 점검이 필요한 성격 |
| **pptx LibreOffice 폴백** | 외부 `soffice` 감지 후 PDF 변환 | **제외 확정**(2026-08-06). 재검토 시에만 되살린다 |
| **remote-control** | 웹에서 프로젝트 화면 미러링 | **보류 확정**. 잠자기 중 동작이 OS 정책상 불가 — 범위 재합의가 선행돼야 한다. `docs/research/remote-control.md` |

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
| 테마 크레딧 UI | ResolvedTheme author/license/source 와 `settings.themeCredits` 등 i18n 6키가 미노출 | 노출 위치(ThemePicker 카드/프리뷰 헤더) 설계 결정 필요 |
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
| 앱데이터 파일 에디터 편집 | 설정/플러그인 파일을 에디터 탭에서 직접 편집·저장 | file_save 루트 가드 + 탭 projectId 종속 동시 해제 필요 — W1 은 Finder/기본 앱 열기까지 |

## 7.10-W7 결정에서 분리된 후속 후보 (2026-08-12)

| 항목 | 내용 | 보류 사유 |
|------|------|-----------|
| VSIX `contributes.grammars` 임포트 | .vsix 에서 TextMate 문법 + languages 기여를 함께 추출해 신규 언어를 추가 | W7 은 기존 31종의 토큰화 정확도가 목표. 신규 언어 추가는 `LANGUAGE_ID_BY_EXTENSION`(Rust 컴파일 타임 상수)의 런타임 오버레이화 + 언어 id 충돌 정책(테마의 "사본 저장"이 언어에는 적용 불가) 설계가 선행돼야 한다. 같은 목적은 플러그인 `grammar` 기여(`docs/features/plugins.md` §2)로 이미 달성 가능 |
| `.tf`→`terraform` / `.mdx`→`mdx` grammar 정밀화 | 현재 `.tf` 는 shiki `hcl` grammar, `.mdx` 는 `markdown` grammar 로 매핑된다. shiki 에는 각각 전용 `terraform`(hcl 과 별도, MPL-2.0)·`mdx`(MIT) grammar 가 존재해 더 정밀한 토큰화가 가능하다 | W7 범위 밖의 동작 변경(기존 매핑을 바꾸는 것) — 별도 검토·사용자 확인 후 진행 |
