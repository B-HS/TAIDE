# 2026-08-29 전수조사 (Fable) — 종합 보고서

> 사용자 지시: "Fable 로 프로젝트 전수조사 — ① 성능·최적화 ② GitHub Actions 빌드 시간 단축
> ③ 기능 버그 ④ UI/UX (VS Code·Zed 대비 누락) ⑤ 기타 고도화". 조사 6축 병렬 에이전트 수행,
> 기지 이슈(d-43~d-45, d-39 e2e 파일럿, 감사 297건 처리분)는 제외·기지 표기.
> 처분: 결정 불요 항목은 d-50(Rust)·d-51(프론트) workflow(opus+xhigh)로 즉시 구현,
> 결정 필요 항목은 §6, 대형·설계 변경은 §7 백로그 이관.
>
> 상태: **6축 전체 완료(2026-08-29)**.

## 처분 요약

| 처분 | 항목 |
|------|------|
| d-50 Rust 일괄 (즉시) | Rust 버그 12건 + Rust 성능 H-1~H-4(부분)·M-1·M-3~M-7·M-9·L-2·L-3 |
| d-51 프론트 일괄 (즉시) | 프론트 성능 1~2·4~9·11~14 + TS 버그 확증 29·유력 8그룹(검증 후 안전분) |
| d-52 CI (무결정분 즉시 + 결정분 답변 후) | save-if 정리·릴리스 노트 조기 게이트(무결정) / 캐시 워밍·러너·LTO(§6) |
| d-53 UX (범위 결정 후) | 저비용 즉효 5건(±α — §6 Q4) |
| 사용자 결정 (§6) | 릴리스 프로필·캐시 워밍·러너 전환·UX 범위 |
| 백로그 이관 (§7) | git status 캐시·refs 캐시·워처 IdCache·트리 응답 재설계·ide diff 이벤트·중대형 UX·차별화 6건 |

---

## §1 성능 — 프론트엔드

### H (심각)

1. **코드 스플리팅 전무 — 초기 청크 5.4MB + modulepreload 1.7MB**. `vite.config.ts` manualChunks 없음, `React.lazy` 사용 0건. monaco·xlsx·pdfjs·xterm·settings-view·theme-editor·plugin-manager·welcome 전부 eager. → d-51: PreviewPane 분기(Pdf/Spreadsheet/Hwp)·settings/theme-editor/plugin-manager/keybindings lazy 분리.
2. **PDF 워커 무조건 기동 스폰** — `src/shared/lib/pdf/setup.ts:4` 모듈 평가 시 `new PdfWorker()`(1.2MB), 정적 import 체인으로 부팅 시 실행. → d-51: 첫 PdfPreview 마운트 시 지연 초기화.
3. **검색 스트리밍 O(n²)** — `entities/search/use-search-run.ts:54` 매치 1건당 `groupSearchMatches(collected)` 전체 재그룹핑 + setState(최대 10,000회). → d-50 검색 스테이지(백엔드 배칭과 동시 수정).
4. **검색 결과 비가상화** — `features/search/search-results-list.tsx:39` 최대 1만 행 + 컨텍스트 라인 전량 DOM. → d-51: react-virtual 평탄화 적용(file-tree 패턴 재사용).
5. **페인 리사이즈 드래그 홍수** — `widgets/editor-area/pane-node-view.tsx:61` pointermove 당 `resizePane` IPC + 이벤트 에코 리페치(revision 가드가 자기 에코를 못 거름). 모든 레이아웃 뮤테이션마다 에코 리페치 1회 부수 발생. → d-51: 드래그 종료 커밋(trailing debounce) + 캐시 revision 비교로 에코 억제.

### M

6. **닫은 탭 monaco 모델 영구 보존(메모리)** — `entities/editor/model-registry.ts`, 실파일 dispose 호출처 0(untitled 만 2곳). `docs/research/performance-memory.md` 자체 경고와 일치. → d-51: useCloseTab 의 stillOpenElsewhere=false 시 disposeModel.
7. **부팅 시 grammar 31종 전량 로드(~1.6MB, cpp 단독 797KB)** — `shared/lib/shiki/lang-map.ts:82`. → d-51: 코어 소수 + 파일 열림 시 온디맨드 `loadLanguage`.
8. **git 이벤트가 GIT.PROJECT 전폭 무효화** — `app/providers/ipc-sync-provider.tsx:190` predicate 없음 → REV_IMMUTABLE 보장이 이벤트 에코 경로에서 무너짐 + 뮤테이션과 이중 리페치. fs:changed 축의 d-44 기각과는 별건(.git 축 내부 비일관성). → d-51: `isGitQueryScopeMutable` predicate 2줄.
9. **키스트로크마다 `model.getValue()` 전체 문자열 생성** — `features/editor/code-editor.tsx:147`(수신측은 디바운스 소비). → d-51: 게으른 접근자/디바운스 시점 읽기.
10. **(기지 L1-03) 팔레트 files fuzzy 전량 스캔** — d-42 §5 이월 존치. 실측 후 판단 결정 유지, 이번 미착수.

### L

11. 워크스페이스 심볼 검색 미디바운스(`use-workspace-symbol-search.ts:25`) → d-51 debounce.
12. 마커 전체 재수집 + Problems 비가상화(`use-monaco-markers.ts:11`, `problems-panel.tsx:75`) → d-51 가상화.
13. fs:changed 대량 배치 path 당 2회 invalidate(`ipc-sync-provider.tsx:221`) → d-51 Set+predicate 통합.
14. CommandPalette 상시 렌더 비용(`command-palette.tsx:158`) → d-51 분기 내 이동+스냅샷 승격.

양호 확인: 파일 트리·커밋 그래프 가상화, 대용량 tier, 터미널(WebGL·RAF·flow control·dispose), monaco 모델 탭 재사용, Context 남용 없음, d-45 수정 확인.

## §2 성능 — Rust

### H

- **H-1 검색 단일 스레드 순차 스캔** — `domain/search/service.rs:326` `ignore::Walk` 순차(문서 architecture.md:51 "병렬 스캔" 과도 불일치). → d-50: `build_parallel()` 전환.
- **H-2 검색이 파일 전체 무제한 read 후 8KB sniff** — `service.rs:228`(치환 `:397` 동일). 크기 상한·파일 내 취소 부재. (버그 축 #8 과 동일 건) → d-50.
- **H-3 file 도메인 spawn_blocking 0건** — 50MB 동기 read·fsync 쓰기·재귀 복사가 async 워커+전역 mutation guard 하에서 실행(`domain/file/service.rs:88` 등). architecture.md §2.1 위반. → d-50.
- **H-4 트리 mutation — 전역 락 하 동기 read_dir + 전체 트리 재직렬화 반환** — `domain/tree/commands.rs:119`, `service.rs:180-200`. → d-50 은 IPC 계약 불변 범위만(락 밖 선행 read, flatten 조기 종료, entry_order 정렬 키 사전 계산). 응답 형태 재설계는 §7.

### M

- M-1 git 조회 7종 spawn_blocking 누락(gutter 핫패스 포함) + `git_show_file` 블롭 무상한 — `domain/git/commands.rs:225,251,258,270,482,489,568` → d-50.
- M-2 git_status 전량 재계산·양방향 rename·stat-stale 재해시 반복 → §7(캐시 설계·rename 방향은 동작 변경 판단 수반).
- M-3 에이전트 폴링 500ms×pty 세션수 `ps` fork — `domain/agent/commands.rs:208` → d-50: pid 일괄 배칭+세션 0 조기 반환.
- M-4 리터럴 검색 라인당 String 2개 할당 — `search/service.rs:34` → d-50.
- M-5 터미널 링버퍼 `Vec::drain` — 64KB 유입마다 2MB memmove — `terminal/service.rs:6` → d-50: VecDeque/원형 버퍼.
- M-6 `pty_write` async 워커 블로킹 쓰기 — `terminal/commands.rs:296` → d-50: spawn_blocking.
- M-7 `run_git` 무타임아웃(+pull 이 guard 보유 중 네트워크 대기) — `git/commands.rs:414`, `service.rs:1894` → d-50: wait_timeout+kill(락 분리는 기존 보류 결정 존중).
- M-8 `opt-level = "s"` — §6 Q1 (릴리스 프로필 결정).
- M-9 검색 매치 1건당 Channel 1건(최대 10k 직렬화) — `search/commands.rs:97` → d-50: 파일 단위 그룹 배칭(프론트 §1-3 과 동시 수정).
- M-10 워처 FileIdMap 이 무시 디렉토리 포함 전체 트리 워크 — §7(커스텀 IdCache, d-35 NoCache 기각 사유 보존).

### L

- L-1 git_log 페이지마다 refs 맵 재구축 + O(skip) revwalk → §7.
- L-2 pty flusher 5ms 상시 wakeup(세션당 초당 200회) → d-50: condvar/park.
- L-3 LSP wait 50ms 폴링 + 수신 버퍼 drain memmove → d-50.
- L-4 `ide:diff-requested` 가 파일 전문을 이벤트로 운반 → §7(request_id + pull 재설계).
- L-5 소형: `search_list_files` 무상한은 d-42 계약상 의도 존중(FsChanged 캐시는 §7), keyring 동기 호출 등 영향 미미.

## §3 GitHub Actions 빌드

실측(v0.1.2, run 33190360486): 전체 9m54s = test-frontend 21s ∥ test-rust 5m18s ∥ **build 9m35s**(cargo release ~7m59s가 지배) → release 13s.

| 항목 | 절감 | 처분 |
|------|------|------|
| 태그 빌드 rust-cache 항상 콜드(태그 전용 트리거 + 캐시 ref 스코프 — `v0-rust-release-*` 캐시 부재 실증) → main 캐시 워밍 워크플로 | 큼 (8분→3~4분대 추정, 항목2 결합) | §6 Q2 |
| fat LTO + codegen-units=1 링크 비용(캐시로도 불소거) → thin LTO | 큼 | §6 Q1 |
| test-rust 가 태그마다 542MiB 무효 캐시 저장(28s) → save-if 추가 | 중간 | d-52 즉시 |
| 릴리스 노트 검증이 build 후에만 존재 → build job 앞단 복제(실패 조기화 ~10분/회) | 중간(실패 경로) | d-52 즉시 |
| release·test-frontend 의 macOS 러너 → ubuntu | 작음 | §6 Q3 |
| timeout 축소(build 75→40m, test-rust 45→30m) | 행 시 한정 | d-52 즉시 |
| 프론트(bun install 2s·vite 14.4s)·러너 등급·사이드카(동일 target 공유, 이중 컴파일 없음) | 비병목 — 조치 불요 | — |

숫자 4 금지 게이트: acknowledge 정본 존재 확인, 빌드 시간 영향 없음. 버전 스킵 체계는 태그·tauri.conf·Cargo.toml 3곳 동기 유지 필요(사실 기록).

## §4 기능 버그

### §4-A Rust (확증 7 · 유력 5 — 1·2번은 스크래치 바이너리 실증, 6번은 libgit2 1.9.6 소스 확인)

1. **[확증] 대소문자-only rename 무시** — `file_rename` 의 canonicalize 가 디스크 실표기를 돌려줘 from==to no-op(`domain/file/commands.rs:45`, `infra/root_guard.rs:101`). readme.md→README.md 무반응.
2. **[확증] rename 목적지 기존 파일 무경고 덮어쓰기(데이터 파괴)** — `rename_entry` 존재 검사 없음(`file/service.rs:152`). 프론트 중복 검사는 대소문자 구분이라 통과.
3. **[확증] 비 UTF-8 파일 열기→저장 시 원본 영구 파손** — `from_utf8_lossy` 열기 + 손실 표식 없음 + 저장 가능(`file/service.rs:88,115,452`). EUC-KR/Latin-1 이 U+FFFD 로 고정. → 최소 수정: lossy 감지 시 읽기 전용 강제+표식(인코딩 왕복 지원은 §7).
4. **[확증] 워처 무시 목록이 파일명·git ref 에도 적용** — `infra/watcher.rs:138` 전 성분 비교 → 루트의 `build`/`dist` 파일 변경 영구 미반영, `refs/heads/*build*` 류 브랜치 ref 이벤트 전부 드롭.
5. **[확증] pty_attach 리플레이·브로드캐스트 원자성 부재** — 재-attach 순간 출력 청크 중복/유실(`terminal/commands.rs:346,248`).
6. **[확증] untracked 디렉토리 stage 상시 실패** — status 는 `dir/` 단일 행, `index.add_path` 는 디렉토리에 GIT_EDIRECTORY(`git/service.rs:97,1676`). 다중 선택에 섞이면 전체 실패.
7. **[유력] LSP workspace folder URI 미인코딩** — `lsp/commands.rs:98` `format!("file://{root}")` vs 프론트 `monaco.Uri.file`(인코딩) — 공백·한글 경로 멀티루트 불일치.
8. **[유력] 검색/치환 파일 크기 무상한**(=성능 H-2) — GB 급 파일 메모리 스파이크·취소 불가.
9. **[유력] Range suffix(`bytes=-N`) RFC 7233 오해석** — `infra/range_file.rs:60` 앞에서 N+1 바이트 반환 → mp4 프리뷰 탐색 실패 가능.
10. **[유력] git discard 절대경로 호출 시 untracked 디렉토리 조용한 no-op** — 트레일링 슬래시 소거로 행 불일치(`git/service.rs:164,1634`).
11. **[유력] LSP 프레이밍 — Content-Length 없는 헤더에 영구 wedging** — `infra/lsp_proc.rs:137` 불량 헤더 미drain → 세션 무응답+버퍼 무한 성장.
12. **[확증·경미] write_atomic 실패 시 tmp 잔존** — `infra/persist.rs:11`(with_mode 는 정리함).

→ 전건 d-50. 우선순위: 2(데이터 파괴)·3(파손)·6(상시 실패) 최우선.

### §4-B TypeScript (확증 29 · 유력 8그룹)

**A. 데이터 손실·저장소 오염 (7)**

- **A1 [확증] 핫엑시트 미러 복원이 sync effect 에 즉시 되씌워짐** — 탭 복귀·재시작 시 미저장 편집 소실 + 500ms 뒤 미러의 복구 데이터까지 파괴. `widgets/editor-pane/editor-pane.tsx:177`(sync) vs `use-editor-file-persistence.ts:382`(복원) — 같은 커밋 발화, Wave A `02e9c54` 의 queueMicrotask 유예로 실행 순서 역전, useEffectEvent 가 구식 dirty=false 를 읽어 가드 통과. React 19 effect 순서로 기계 확정.
- **A2 [확증] 접힌 폴더 대상 붙여넣기·개명 무경고 덮어쓰기** — 충돌 검사가 화면 가시 rows 만 봄(lazy 트리) + 백엔드 rename/copy 목적지 가드 부재(`use-explorer-clipboard.ts:45`, Rust §4-A-2 와 동근). 대소문자·NFD/NFC 변형 포함.
- **A3 [확증] rename/delete 후 열린 탭 미추종** — 파일 rename 시 에러 탭 전환·중복 탭, 폴더 rename 후 ⌘S 는 `write_atomic` 의 create_dir_all 이 옛 폴더를 재생성해 옛 경로에 저장(조용한 fork). 탭 경로 갱신 코드 전무.
- **A4 [확증] merge 충돌 중 커밋이 마커째 자동 스테이지·커밋** — 충돌 행이 staged/unstaged 판정 양쪽에서 제외돼 stage-all 확인 미발동(`git-panel.tsx:117`).
- **A5 [확증] Replace All 이 화면 결과가 아닌 클릭 시점 입력값·토글로 치환** — `search-panel-container.tsx:59` `buildQuery()` 직접 사용, 표시된 적 없는 매치 비가역 치환.
- **A6 [확증] 터미널 탭 복귀 시 재부착 실패** — 스폰 성공을 SESSIONS 캐시에 미반영(staleTime Infinity) → 매번 새 셸 + 기존 셸 고아화(`terminal.query.ts:10`, `terminal-session.tsx:58`). features/terminal.md §3 설계 위반.
- **A7 [확증] 스플릿 동일 파일 — 한쪽 저장 후 반대쪽 dirty 잔존 + 가짜 "changed on disk" 배너 + 미러 부활** — 저장 성공이 경로 단위로 정착되지 않음(A1·B7 과 동일 "외부 저장 미정착" 클래스).

**B. 핵심 기능 오동작 (15)** — B1 APP 키맵·재바인딩 동일 키 이중 발동 / B2 보조 창 ⌘K ⌘S 키만 삼킴 / B3 monaco Find 위젯 포커스 중 ⌘F 가 전역 검색으로 이탈 / B4 pinned 탭 미보호(unpin 라벨 버튼이 닫기 실행·⌘W·미들클릭 무가드 — tabs.md §3 위반) / B5 활성 테마 삭제 시 fallback 전무(재시작 후 하이라이트 전멸) / B6 vsix 테마 무변경 저장만으로 tokenColors·author·source 영구 소실 / B7 플러그인 설치 후 기존 파일 재하이라이트 누락(FILE 무효화·setModelLanguage 부재) + ide_save 외부 저장 미정착 코드확정 / B8 Search Editor 매치 클릭 한 번에 결과·수정 쿼리 전파괴+복귀 시 자동 재실행 / B9 프로젝트 전환 후 검색 잔존·클릭 시 레이아웃 교차 오염 / B10 git diff 탭 상대경로 이원화(fs:changed 무효화 불발·중복 탭·충돌 판정 상시 false) / B11 staged rename diff 가 전체 추가로 표시(beforePath 미전달) / B12 아웃라인·브레드크럼 편집 후 영구 stale / B13 IME(한글) isComposing 가드 전면 사망(WKWebView 실측 — keyCode 229 공통 가드 필요: 키맵 chord·검색 Enter·탐색기 draft) / B14 죽은 세션 attach 먹통 터미널 / B15 설정 저장 실패 전면 무음.

**C. 기타 확증 (14)** — C1 잘라내기 제자리 붙여넣기가 "복사본" 개명 / C2 폴더 삭제 후 동명 재생성 시 유령 자식(Rust 트리 캐시 미정리) / C3 폴더 복사 접미 확장자 오분해(v1.2→"v1 복사본.2") / C4 인라인 생성 이중 Enter 재진입 오류 토스트 / C5 태그 다이얼로그 입력 잔존 / C6 커밋 메시지 뷰 전환 소실·프로젝트 교차 잔존 / C7 키바인딩 에디터 무수식 2단 chord 캡처 불가 / C8 키 검색 모드의 숨은 텍스트 필터 / C9 검색 미실행·실패를 0건으로 오표시 / C10 치환 부분 실패 무음 스킵·10k 상한 미표시 / C11 제외 글롭 placeholder 가 단순 매처와 불일치 / C12 AI 프로바이더 전환 후 모델 미선택 오표시 / C13 터미널 파일 링크 와이드 문자 좌표 어긋남 / C14 스폰 전 타이핑 유실·탭 활성 포커스 미이동·스폰 중 닫기 고아.

**D. 유력 (8그룹, 검증 후 수정)** — D1 팔레트 닫힘 시 Radix 포커스 복원 역전 / D2 단일 키 재바인딩에 기본 when 승계(터미널 포커스 침묵) / D3 충돌 검사가 monaco 기본 바인딩 미인지 / D4 검색 cancel·run 경합 외 5건(supersede 신뢰로 정리) / D5 flow-control pause 중 detach 영구 pause / D6 플러그인 삭제 후 disposed highlighter 참조 등 4건 / D7 sync "Pull Remote" 무피드백 / D8 hunk 경계 프런트·백엔드 정확일치 가정.

무결 확인: Tauri listen 해제·0-렌더·`||` 오용·unmount setState 위반 없음. 로케일 918키×3 완전 일치(i18next 복수형 변형 부재는 기록만).

→ 전건 d-51 편입. Rust 결부 항목(A2 copy/rename 가드·A3 탭 추종 커맨드·B11 beforePath·C2 트리 캐시)은 d-50 해당 스테이지로 이관.

**처분 정정 (2026-08-29, 구현 후 검토)**

- **B11 = 부분 이행**(전건 처리 아님). `git_diff_file(beforePath)` 축은 Rust·bindings·`QUERY_KEY.GIT.DIFF`·`DiffPane` 까지 배선이 끝났고 **커밋 diff 축은 실제로 동작**한다. 그러나 git 패널의 상태 행(`StatusRow.origPath`)은 구조적으로 항상 `null` 이라(`git2` 의 `StatusEntry::path()` 가 이미 `old_file.path` 를 돌려주고, `orig_path` 는 그것을 `path` 와 비교해 걸러낸다 — d-50 §5 S3 실측) **워킹트리·스테이지 축의 "개명이 전체 추가로 보이는" 증상은 그대로 남는다.** 근본 해소는 선행 §2 M-2(양방향 rename 정정 — 이 조사에서 §7 백로그로 이관)에서 `StatusRow` 에 실제 새 경로/옛 경로 쌍을 채우는 것으로 처리한다.
- §4-A-3(비 UTF-8 저장 파손)의 **프론트 차단 경로가 처음에는 `EditorPane` 한 곳뿐이었다.** 구현 후 검토에서 `workspace-edit-applier`(LSP `WorkspaceEdit`)와 `ide-sync-provider`(Claude Code 저장 요청) 두 진입점이 가드를 통과하지 않는 것을 확인해 함께 닫았다 — d-50 §4 참조.

## §5 UI/UX 격차 (VS Code·Zed 대비) + 고도화

이미 구현 확인(제안 제외): 미니맵·sticky scroll·breadcrumbs·bracket colorization·format/auto save·LSP 어댑터 20종(workspace-symbol 포함)·git gutter/blame/그래프/stash/hunk stage/충돌 해석·전역 replace+정규식·검색 에디터·탭 DND 스플릿·멀티 윈도우·Zen·세션 복원·키맵 chord/when 엔진·마크다운/이미지/PDF/xlsx/HWP 프리뷰·AI inline suggest/Inline Edit/MCP 서버. PRD 비목표(확장 마켓·협업·DAP·AI 채팅 패널)는 격차로 세지 않음.

### 저비용 즉효 (→ §6 Q4, d-53)

1. diff 에디터 `hideUnchangedRegions` + `showMoves`(`features/git/diff-view.tsx:25` 현재 미설정)
2. `guides.bracketPairs`(가이드선)
3. `smoothScrolling` + `cursorSmoothCaretAnimation`
4. `suggest.preview`
5. `editorRulers` 설정 신설

### 중간 규모 (→ §7 백로그)

로컬 히스토리/타임라인(AI 대량 수정 안전망 — 가치 특대), EditorConfig, trim/final-newline on-save, DocumentColor(색 데코레이터), Linked Editing, 탭 tear-off 제스처, vim 모드(의존성 합의 필요).

### 대형 (→ §7)

Multibuffer(Zed 시그니처 — monaco 무대응, 자체 설계), Call Hierarchy.

### 차별화 고도화 제안 (→ §7, 사용자 선별)

① 병렬 에이전트 워크트리 오케스트레이션(worktree×프로젝트×터미널×에이전트 배지 조립형) ② AI diff 리뷰 ③ Problems→Inline Edit 파이프 ④ AI 편집 체크포인트(로컬 히스토리의 AI 특화) ⑤ MCP 서버 매니저(.mcp.json GUI) ⑥ PROCESS.md 워크플로 패널.

## §6 사용자 결정 필요 (질문 발신됨)

1. **릴리스 프로필** — thin LTO+opt-level 3(추천: CI 링크 시간·런타임 동시 개선, dmg 크기 실측 비교 보고) / thin LTO 만 / 현행.
2. **CI 캐시 워밍** — main push paths 필터 워밍(추천) / 전 push / 미도입.
3. **러너** — release job 만 ubuntu(추천: macOS bun test 커버리지 유지) / 둘 다 / 유지.
4. **UX 착수 범위** — 저비용 5건만(추천) / +on-save 정리·EditorConfig / 백로그만.

## §7 백로그 이관 목록 (docs/backlog.md 등재 예정)

- Rust: git status 결과 캐시+rename 방향 축소(M-2), git_log refs 캐시·커서 재개(L-1), 워처 선택적 IdCache(M-10), 트리 mutation 응답 재설계(H-4 후반), ide:diff-requested pull 전환(L-4), search_list_files FsChanged 캐시, 비 UTF-8 인코딩 왕복 지원(§4-A-3 후속).
- UX 중형: 로컬 히스토리·EditorConfig·on-save trim(Q4-b 미채택 시)·DocumentColor·Linked Editing·탭 tear-off·vim 모드.
- UX 대형: Multibuffer·Call Hierarchy.
- 차별화: §5 ①~⑥ (사용자 선별 대기).
- 프론트: 팔레트 fuzzy(L1-03 기지 — 실측 후).
