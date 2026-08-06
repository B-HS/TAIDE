# 기능 — 탐색 사이드바 (파일 / 검색 / Git 전환)

> FR-C. content 좌측 사이드바 — Cursor/VSCode 구성. Git 뷰 내용은 `git.md`,
> 성능 전략 근거: `docs/research/performance-memory.md` §4, `docs/research/react-frontend-stack.md` §5.

## 1. 구성

- 상단 아이콘 전환: **파일(Explorer) / 검색(Search) / Git(SCM)** — Cursor 스타일(FR-C2).
  단축키 `⇧⌘E` / `⇧⌘F` / `⌃⇧G` (VSCode 동일). 아이콘에 배지(검색 결과 수, git 변경 수).
- 사이드바 토글 `⌘B`. 폭 리사이즈(react-resizable-panels `Panel collapsible`) — 프로젝트별 저장.

## 2. 파일 트리 (FR-C3·C4)

### 2.1 데이터 구조 — 확정 (사용자 결정, 2026-08-06)

**Rust 소유 트리 + flat rows + @tanstack/react-virtual.** 트리 상태(펼침 포함)를 Rust 가 소유하고
`tree_rows(offset, limit)` 로 보이는 행만 페이지네이션, view 는 가상 스크롤 렌더만 담당한다.
ADR-0004 와 완전 일치(view reload 복원 공짜), 수만 파일 규모 최적, 외부 트리 라이브러리 불필요.
rename 인라인 편집·트리 DND 는 직접 구현(비용 감수 — 결정 근거).

기각: @headless-tree(트리 상태가 JS 로 이동 — Rust 소유 원칙 예외 증가),
react-arborist(redux5+react-dnd14+react-window 동반 + dnd-kit 과 DnD 이중화).

### 2.2 동작

- **lazy load**: 폴더 펼침 시 그 한 단계만 `read_dir`(재귀 프리로드 금지). 펼침 상태는 Rust 트리에
  기록되고 layout 영속화에 포함(재시작 복원).
- 렌더: 고정 행높이 22px, `getItemKey` = 안정 노드 id, overscan 12. 행 = 들여쓰기 + 폴더/파일
  아이콘 + 이름 + git 상태색(`explorer.git*` 토큰 — M/A/D/U/ignored 흐림).
- 클릭 = preview 탭으로 열기, 더블클릭 = 고정 탭(FR-C4, `tabs.md` §3).
- 키보드: ↑↓ 이동, ←→ 접기/펼치기, Enter 열기, 타이핑 시 이름 점프(typeahead).
- context menu: 새 파일/폴더, 이름 변경(인라인 입력), 삭제(휴지통 이동 + 확인), 복사/붙여넣기,
  경로 복사, Finder 에서 열기, (git 있으면) 하위 항목 — 전부 Rust fs 명령 경유.
- 파일 조작 후 watcher 이벤트로 트리 갱신(자기 쓰기 echo 는 origin 플래그로 낙관 갱신).
- 트리 내 DND(파일 이동)는 2차(dnd-kit 재사용).

### 2.3 갱신·성능

- watcher(notify 8.2 + notify-debouncer-full 0.7, 300ms debounce)가 **변경 디렉토리의 자식만**
  재조회 — 루트 재스캔 금지. 무시 목록(`.git`, `node_modules`, `target`, `dist`, `.next` 등)은
  워처·트리·검색이 **동일 규칙 공유**(`shared` 상수 + Rust 상수 동기).
- Linux inotify watch 한도 초과는 조용히 실패 — 에러를 UI 배너로 노출(research 함정).
- 이벤트는 경로 배열 1건으로 묶어 emit(파일당 1 emit 금지).

## 3. 검색 (FR-C5)

- 프로젝트 전역 텍스트 검색: Rust `domain/search` 가 **ripgrep 라이브러리(grep 크레이트 계열)** 또는
  자체 병렬 스캔으로 수행(구현 시 grep-searcher 채택 검토 — 무시 목록·.gitignore 존중).
- UI: 검색어 + 옵션(대소문자/단어/정규식) + 포함/제외 glob, 결과는 파일별 그룹 트리(매치 라인
  하이라이트), 클릭 시 해당 위치로 열기. 결과 상한(예: 10,000 매치) + "더 보기".
- 스트리밍: 결과는 Channel 로 점진 수신(대형 리포에서 첫 결과 즉시 표시), 새 검색 시작 시
  이전 검색 태스크 취소(CancellationToken).
- 치환(replace)은 2차.

## 4. 수명주기

- 사이드바 뷰 전환은 탭 유지(마운트 유지 여부: 검색 상태 보존을 위해 Search 뷰는 keep-alive,
  Explorer 는 가상화라 재마운트 저렴 — 상태는 Rust 소유이므로 어느 쪽이든 복원됨).
- 트리 row 캐시는 view 메모리에 페이지 단위 LRU(최대 수천 행) — reload 시 Rust 에서 재조회.
