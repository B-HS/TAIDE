# d-50 계약 — 전수조사 Rust 버그+성능 일괄 (2026-08-29)

> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §2·§4-A. 사용자 지시:
> 결정 필요 항목 외 전건 workflow(opus+xhigh) 즉시 구현. 산출물은 커밋 지시 전까지 워킹트리.
> 운영 규약 승계: Rust 한 시점 한 에이전트(구현 스테이지 직렬), 구현→검토(렌즈)→수정→메인 2차 검증.

## §0 범위 제외 (이 배치에서 하지 않음)

- M-2(git status 캐시·rename 방향)·L-1(refs 캐시)·M-10(IdCache)·L-4(diff 이벤트 재설계)·
  트리 응답 형태 변경 — 백로그(§7) 이관.
- M-8(opt-level)·CI 항목 — 사용자 결정 대기(d-52).
- pull 의 guard 분리 — 기존 보류 결정(2026-08-19 §1.1) 존중, 타임아웃만.

## §1 스테이지 (직렬)

| 스테이지 | 항목 (보고서 번호) | 주요 파일 |
|---|---|---|
| S1a search | 버그8(=H-2 크기 상한·sniff 선행·파일 내 취소)·H-1 병렬화·M-4 무할당 매칭·M-9 파일 단위 배칭·C10 Rust 절반(치환 스킵 사유 수집)·프론트 수신부(use-search-run 배칭·그룹핑 증분)만 동시 — 검색 UI 는 d-51 F2 | domain/search/*, entities/search/use-search-run.ts, 그룹핑 lib |
| S2 file | 버그1(case-only rename)·버그2+A2(rename·copy 목적지 존재 가드)·버그3(lossy 읽기 전용 강제+표식)·버그12(write_atomic tmp 정리)·H-3(spawn_blocking) | domain/file/*, infra/persist.rs, infra/root_guard.rs |
| S3 git | 버그6(untracked dir stage)·버그10(discard 슬래시 정규화)·M-1(조회 7종 spawn_blocking + show_file 상한)·M-7(run_git 타임아웃+kill)·B11 Rust 부분(diff beforePath 축) | domain/git/* |
| S4 terminal | 버그5(attach 원자성)·M-5(링버퍼)·M-6(pty_write spawn_blocking)·L-2(flusher condvar) | domain/terminal/*, infra/pty.rs |
| S5 lsp | 버그7(URI 인코딩)·버그11(프레이밍 wedge)·L-3(wait select·버퍼 커서) | domain/lsp/commands.rs, infra/lsp_proc.rs |
| S6 infra 소형 | 버그4(워처 무시 판정 조상 성분 한정+git 워처 면제)·버그9(Range suffix RFC 7233)·M-3(ps 일괄 배칭·세션 0 스킵) | infra/watcher.rs, infra/range_file.rs, domain/agent/commands.rs |
| S7 tree | H-4 계약 불변 부분(락 밖 선행 read_dir·flatten 조기 종료·정렬 키 사전 계산)·C2(삭제·개명 시 expanded·캐시 prefix 정리) | domain/tree/* |
| S8 layout | A3(rename/delete 열린 탭 추종 — 커맨드 신설·미러/model-registry 키 이관·FE 배선)·프론트 perf-6(닫힌 탭 모델 dispose) — FE 체인 F4 완료 후 착수(게이트) | domain/layout/*, entities/layout, widgets/explorer 배선 |

> 실행 형태: d-50 Rust 체인과 d-51 FE 체인을 한 workflow 에서 병행하되, Rust 는 한 시점 한
> 에이전트(직렬), 파일 접점은 게이트(S1a 선행·F3 는 S3 후·S8 은 F4 후)로 직렬화한다.

## §2 공통 규칙

- 버그는 실패 재현 테스트 먼저(가능한 형태) → 수정 → 통과. 기존 테스트 무손상.
- IPC 타입·커맨드 표면 변경 시 bindings 재생성 + dispatch/원격 정책 파리티 유지
  (docs/ipc-contract.md·agent-operations.md 절차).
- 각 스테이지 종료 전 cargo fmt + 해당 도메인 필터 테스트 그린. 전체 사다리는 최종 검증에서.
- 커밋·푸시 금지.

## §3 구현 기록 (스테이지별 append)

### S1a search (2026-08-29)

파일: `src-tauri/src/domain/search/{types.rs,service.rs,commands.rs}` ·
`src-tauri/src/domain/layout/types.rs`(doc 참조명 1줄) · `src/shared/api/bindings.ts`(재생성) ·
`src/entities/search/{search-result.ts,search-result.test.ts,search.ipc.ts,use-search-run.ts}` ·
`docs/ipc-contract.md`.

**(1) 버그8 = H-2 — 크기 상한 · sniff 선행 · 파일 내 취소** (`service.rs`)

- `read_scannable_bytes(path)` 신설이 `std::fs::read` 전량 읽기를 대체한다. 순서를 뒤집었다:
  ① `metadata().len()` 으로 `constants::REFUSED_FILE_BYTES`(50MB, file 도메인 티어 상수 재사용)
  이상이면 **한 바이트도 읽지 않고** 거절 → ② `Read::take(BINARY_SNIFF_BYTES)` 로 앞 8KB 만 읽어
  바이너리 판별 → ③ 통과한 파일만 나머지를 이어 읽는다. 리더 전체에도 `take(REFUSED_FILE_BYTES)`
  를 걸어 metadata 이후 파일이 커져도 버퍼가 무한해지지 않게 했다.
  상한을 `REFUSED_FILE_BYTES` 로 고른 근거: `file_open` 이 열기를 거절하는 경계와 같아
  "열 수 있는 파일 = 검색되는 파일" 집합이 유지된다(`READ_ONLY_FILE_BYTES`(20MB)로 잡으면 편집기가
  읽기전용으로 열어주는 파일이 검색은 안 되는 비대칭이 생긴다).
- `search_file` 이 `cancelled: &AtomicBool` 을 받아 라인 루프에서 `CANCEL_CHECK_LINE_STRIDE`(1024)
  줄마다 확인한다. 이전에는 파일 **사이**에서만 확인해, 수백만 줄짜리 단일 파일은 취소해도 끝까지
  스캔했다.
- `replace_in_file` 도 같은 `read_scannable_bytes` 를 쓴다(치환 경로 `:397` 동일 결함 해소).
- 추가 테스트(Rust 4): `크기_상한_이상인_파일은_읽지_않고_거절한다`(sparse 파일 `set_len` 으로
  50MB 를 디스크 없이 재현 — 사유가 `Binary` 가 아니라 `TooLarge` 여야 sniff 보다 크기 판정이
  선행한다는 뜻) · `크기_상한_이상인_파일은_검색_대상에서_제외된다` ·
  `크기_상한_이상인_파일의_치환은_스킵_사유와_함께_보고된다` ·
  `파일_내_라인_루프도_취소_플래그를_확인한다`.

**(2) H-1 — `build_parallel()` 병렬 스캔**

- `build_walk` 의 설정부를 `configure_walk(root, respect_gitignore) -> WalkBuilder` 로 분리해
  순차(`build()`)·병렬(`build_parallel()`) 두 워크가 **같은 필터 규약**(`IGNORED_DIR_NAMES` 무조건
  prune, `parents(false)` 등)을 공유한다.
- 공유 상태 3종: ① `cancelled: &AtomicBool` 을 모든 워커가 파일마다(+파일 내부에서) 읽고
  `WalkState::Quit` ② 매치 예산은 `AtomicU32` + `claim_match_budget` 의 compare-exchange 루프로
  파일 단위 원자 배분 — 단순 `fetch_add` 면 워커 N개가 각자 "9,999 사용"을 읽고 모두 append 해
  `SEARCH_MATCH_LIMIT` 를 초과한다 ③ 결과는 `mpsc` 채널로 보내고 **호출 스레드**가 드레인해
  `on_file_matches` 를 호출 — 덕분에 콜백이 `FnMut` 그대로 유지되고(호출부·테스트에 `Send`/락
  요구 없음), `WalkParallel::run` 이 블로킹임에도 스트리밍이 유지된다. 워크는
  `std::thread::scope` 의 스코프 스레드에서 돌려 `root`/`query`/컴파일된 쿼리를 빌려 쓴다.
- **순서 계약(신규·계약 §3 명기)**: 파일 **안**의 매치 순서는 소스 오름차순 보존, 파일 **간**
  도착 순서는 비결정. `docs/ipc-contract.md` "tree / search" 절 + 신설 "d-50 S1a" 절에 기록.
- `docs/architecture.md:51` 의 "자체 병렬 스캔" 표기는 이번 전환으로 **실제와 일치**하게 됐다
  (문서 수정 불요 — 감사 §2 H-1 이 지적한 불일치 해소).
- 추가 테스트: `병렬_스캔에서도_매치_상한을_정확히_지킨다`(파일 3개 × 각 6,666 매치 → total 정확히
  10,000). 기존 검색 테스트는 도착 순서에 의존하지 않도록 `collect_search` 헬퍼(경로 안정 정렬)로
  전환.

**(3) M-4 — needle 선계산 · 라인 복사 제거**

- `LiteralMatcher`(쿼리당 1회 생성)가 `find_matches_in_line` 의 라인당 `String` 2개 할당
  (`line.to_string()`/`to_ascii_lowercase()` + `query.text.clone()`)을 없앴다. 매치 없는 라인의
  할당은 0(빈 `Vec` 은 할당하지 않음).
- case_sensitive 경로는 `line[from..].find(needle)` 로 **std 의 two-way 검색에 그대로 위임**한다
  (직접 스캔을 짜면 긴 라인에서 오히려 느려진다). case-insensitive 경로만 첫 바이트 후보 필터 +
  `to_ascii_lowercase` 바이트 비교로 직접 스캔한다 — 기존 동작(ASCII 한정 케이스 폴딩)과 동일.
- `MatchMode` 가 `Literal(&LiteralMatcher)` / `Regex{regex, whole_word}` 로 바뀌어
  `matches_for_line` 이 더 이상 `&SearchQuery` 를 받지 않는다. 쿼리 1회 준비물은
  `CompiledQuery{regex, literal}` + `compile_query()` 로 묶었고, `compile_optional_regex` 는
  이것으로 대체(파일마다 matcher 를 다시 만들지 않도록 `replace_one_file` 시그니처가
  `(path, &CompiledQuery, replacement)` 로 변경 — Rust 내부 API, IPC 표면 아님).
- `find_matches_in_line(line, query)` 공개 시그니처는 유지(기존 테스트 무손상).
- 추가 테스트: `대소문자_무시_매칭은_ascii만_접는다_비ascii는_그대로_비교한다` ·
  `멀티바이트_문자_뒤_매치도_바이트_경계를_어긋나지_않는다`.

**(4) M-9 — 파일 단위 그룹 배칭 + bindings 재생성**

- `SearchMatch` → `SearchLineMatch`(=옛 형태에서 `path` 제거) + 신규
  `SearchFileMatches{ path, matches }`. `search_run` 의 채널이 `Channel<SearchFileMatches>` 가 됐다.
  커맨드 이름·인자·반환(`u32`)은 불변 → **dispatch 테이블·원격 정책 분류·커맨드 수 변동 없음**
  (`make_channel` 이 제네릭이라 `dispatch.rs` 무수정). 로케일 키 변동 없음(신규 사용자 문구 없음).
- `cargo test --lib typescript_바인딩을_생성한다` 로 `src/shared/api/bindings.ts` 재생성(+65/-3),
  `bun run format:check` 통과.
- 추가 테스트: `매치는_파일_단위_배치로_전달되고_파일_내_순서를_보존한다`.

**(C10 Rust 절반) 치환 스킵 사유 수집·반환**

- `replace_one_file` 의 `Option<u32>` 를 `ReplaceOutcome{ Replaced(u32) | NoMatch |
  Skipped(ReplaceSkipReason) }` 로 대체 — "매치 없음"과 "고칠 수 없었음"이 같은 `None` 이던 것이
  무음 실패의 근본이었다.
- IPC: `SearchReplaceResult` 에 `skipped: ReplaceSkippedFile[]` · `skippedCount: number` 순증,
  신규 타입 `ReplaceSkippedFile{path, reason}` · `ReplaceSkipReason`(`tooLarge`·`binary`·
  `notUtf8`·`unreadable`·`writeFailed`). 목록은 `REPLACE_SKIP_REPORT_LIMIT`(50)까지만 싣고
  `skippedCount` 가 전체 수(바이너리 자산이 많은 프로젝트에서 응답이 비대해지는 것 방지).
  **매치 없음은 스킵이 아니다**(테스트로 고정).
- 추가 테스트: `치환_스킵은_사유별로_구분해_보고한다` · `매치가_없는_파일은_스킵으로_세지_않는다`
  (+ 위 크기 상한 치환 테스트).

**(5) 프론트 수신부 — 배치 수신 + 타이머 flush + 증분 append**

- `entities/search/search-result.ts`: `groupSearchMatches(matches)`(매치 1건마다 전체 재그룹핑,
  감사 §1-3 의 O(n²)) 제거 → `createSearchResultAccumulator()` + `appendSearchFileMatches(acc,
  batches)`. `indexByPath` 맵으로 배치가 속할 그룹을 O(1) 로 찾고, 바뀌지 않은 그룹 객체는
  **정체성을 유지**해 하위 행 리렌더를 막는다. 같은 경로가 두 번 도착하는 경우도 병합·재중복제거
  (`dedupeAdjacentContext` 는 멱등)로 처리한다.
- `entities/search/use-search-run.ts`: 배치를 `pendingRef` 에 모아
  `RESULT_FLUSH_INTERVAL_MS`(50ms) 타이머 1회로 flush(최대 20 렌더/초), 완료 시 타이머 취소 후
  동기 flush + 최종 total 반영, 새 run·언마운트 시 타이머 정리. **rAF 대신 타이머**를 쓴 이유:
  검색이 도는 창이 배경 보조 창이면 rAF 가 사실상 정지해 완료 전까지 결과가 전혀 안 보이게 된다.
- `entities/search/search.ipc.ts`: `runSearch` 콜백이 `onMatch(match)` → `onFileMatches(batch)`.
- 테스트: `search-result.test.ts` 를 배치 API 기준으로 재작성(11케이스 — 기존 컨텍스트 중복제거
  케이스 전부 승계 + 그룹 정체성 유지·이어붙이기·누적 카운트 신규).

**F2(검색 UI) 접점 — 시그니처 변화 최소화 결과**

- **불변**: `SearchResultGroup{ path, matches }` · `SearchMatchRowData`(이제 `SearchLineMatch`
  별칭) · `useSearchRun` 반환 `{ results, totalMatches, isSearching, run }`. 따라서
  `features/search/*`(`search-panel`·`search-results-list`·`search-match-row`) ·
  `widgets/search-panel/*` · `widgets/search-editor/*` 는 **한 줄도 수정하지 않았다.**
- **F2 가 알아야 할 변경 2건**: ① `SearchReplaceResult` 에 `skipped`/`skippedCount` 가 생겼다
  (C10 FE 절반 = 스킵·잘림 표시가 이 필드를 소비한다) ② 결과 그룹의 **파일 간 순서가 비결정**
  이므로, 가상화(perf-4) 시 인덱스가 아니라 `path` 기반 key 를 쓸 것.

**검증**: `cargo fmt --all` 후 `cargo clippy --workspace --all-targets -- -D warnings` 무경고,
`cargo test --workspace` 1114+26 전부 통과(search 도메인 59), `bun run typecheck` ·
`bun run lint`(search 관련 0) · `bun run format:check` · `bun test` 1536 전부 통과.

### S2 file (2026-08-29)

파일: `src-tauri/src/domain/file/{service.rs,commands.rs,types.rs}` · `src-tauri/src/infra/persist.rs` ·
`src-tauri/src/domain/remote/dispatch.rs`(호출부 2줄) · `src-tauri/src/domain/locale/service.rs`
(키 등록 2) · `src-tauri/resources/locales/{ko,en,ja}.json` · `src-tauri/src/state.rs`(doc 1줄) ·
`src/shared/api/bindings.ts`(재생성) · `src/widgets/editor-pane/{editor-pane.tsx,
use-editor-file-persistence.ts}` · `docs/ipc-contract.md` · `docs/features/editor.md`.

**(1) 버그1 §4-A-1 — 대소문자만 다른 rename 의 조용한 no-op**

- 근본 원인은 `rename_entry` 가 아니라 **커맨드가 넘기는 목적지**였다. `resolve_owning_project` 가
  `to` 를 canonicalize 하는데, 대소문자 무시 파일시스템은 **디스크 실표기**를 돌려주므로
  `readme.md → README.md` 가 `from == to` 로 도착하고 `std::fs::rename` 은 POSIX 대로 "같은 항목"
  이라 아무 일도 하지 않는다. 실측: `mv readme.md README.md` 는 APFS 에서 정상 동작하므로
  `rename_entry` 만 보면 재현되지 않는다 — 재현은 정규화까지 포함한 파이프라인에서만 된다.
- `service::destination_with_requested_name(resolved_to, requested_to)` 신설 — 루트 가드가 검증한
  **정규 부모**는 유지하고 **요청한 파일명**만 되붙인다. 마지막 성분만 바뀌고 그것이 traversal
  일 수 없으므로(`file_name()` 이 `..` 에는 `None`) 루트 봉쇄는 그대로다.
- `rename_entry` 는 목적지가 존재할 때 `is_same_entry`(양쪽 canonicalize 비교)로 "같은 항목의 다른
  표기"를 가려내고, 그때만 `persist::temp_sibling` 임시 이름을 경유한 **2단 rename** 을 한다.
  임시 이름 형태는 워처가 이미 `is_temp_sibling` 으로 그룹에서 걸러내므로 프론트에는 개명 하나로
  보이고, 2단계 실패 시 원래 이름으로 되돌린다.
- 추가 테스트: `대소문자만_다른_개명은_정규화로_경로가_같아져도_요청한_표기로_반영된다`
  (커맨드 파이프라인 재현 — `destination` 대신 `resolved_to` 를 넘기면 실패하는 것을 실측 확인:
  `["readme.md"]` 로 남는다) · `destination_with_requested_name은_정규화된_부모에_요청한_이름을_붙인다`.

**(2) 버그2 §4-A-2 + §4-B A2 — rename·copy 목적지 존재 가드**

- `rename_entry`·`copy_entry` 모두 목적지에 항목이 있으면 `error.file.destinationExists`
  (`InvalidArgument`)로 거절한다. 존재 판정은 `symlink_metadata().is_ok()` — `exists()` 는 깨진
  심볼릭 링크를 놓치는데 rename 은 그것도 덮어쓴다. rename 의 유일한 예외가 위 (1)의 same-entry
  케이스이고, copy 에는 예외가 없다(대소문자 변형 목적지는 `to == from` 이라 기존 copyIntoSelf
  가드가 먼저 잡는다).
- 에러 신설 3종 파리티: `AppError::localized(InvalidArgument, "error.file.destinationExists")`
  + `with_arg("path")`, `MESSAGE_NAMESPACES` 의 `error` 네임스페이스 등록, ko/en/ja 카탈로그 3언어.
  (`내장_3종_로케일은_같은_키_집합을_가진다`·`en_메시지의_모든_키는_required_message_keys에_포함된다`
  통과)
- 추가 테스트: `개명_목적지에_다른_파일이_있으면_덮어쓰지_않고_거부한다` ·
  `복사_목적지에_이미_항목이_있으면_거부한다`(파일 덮어쓰기 + 기존 폴더 병합 두 경우).

**(3) 버그3 §4-A-3 — 비 UTF-8 파일 열기→저장 파손**

- `decode_utf8_lossy(bytes) -> (String, bool)` 로 교체(`String::from_utf8` 성공 경로는 같은 할당을
  그대로 쓰므로 정상 파일은 추가 비용 0, 실패 경로만 lossy 재디코딩). 손실이면
  **`OpenedFile.encoding_lossy = true` + `read_only` 강제**(크기 티어와 독립 — 티어는 크기만 등급).
- IPC 표면: `OpenedFile` 에 `encodingLossy: boolean` **순증**만. 커맨드 이름·인자·개수 불변이라
  dispatch 테이블·원격 정책·커맨드 수 변동 없음. `cargo test --lib typescript_바인딩을_생성한다` 로
  `bindings.ts` 재생성.
- FE 배선(최소): 기존 열람 전용 표시 경로는 `file.readOnly` 로만 갈라져 **사유와 무관하게 "큰
  파일이라" 문구**를 띄웠다 → `editor-pane.tsx` 배너가 `encodingLossy` 로 문구를 가른다(신규 키
  `editor.readOnlyLossyEncoding`, 3언어). `use-editor-file-persistence.ts` 의 `handleSave` 에
  `if (file?.readOnly) return` 가드 1줄 — monaco 는 이미 편집을 막지만 **hot-exit 미러 복원
  (`applyMirrorRestore`)은 에디터를 거치지 않고 draft 를 심으므로** 자동 저장이 손실 텍스트를
  되쓸 수 있었다. 크기 사유 열람 전용 파일도 같은 가드로 보호된다(배너가 원래 약속하던 동작).
- 인코딩 왕복 지원은 백로그(`docs/backlog.md` 기존 항목) — 착수하지 않음.
- 추가 테스트: `비_utf8_파일은_손실_표식과_함께_열람_전용으로_열린다`(EUC-KR 바이트) ·
  `utf8_파일에는_손실_표식이_붙지_않는다`.

**(4) 버그12 §4-A-12 — `write_atomic` 실패 시 임시파일 잔존**

- `write_atomic_with_mode` 와 동일한 패턴(클로저 결과 판정 후 `remove_file(...).ok()`)을 적용.
  추가 테스트 `write_atomic_은_실패시_임시파일을_남기지_않는다`(대상 경로가 디렉토리라 rename 실패).

**(5) §2 H-3 — file 도메인 spawn_blocking**

- `file_open`(최대 50MB 동기 read + UTF-8 디코드 + 라인 카운트) · `file_save`(fsync 쓰기) ·
  `file_copy`(재귀 복사) · `file_mirror_dirty`(500ms 타이핑 debounce 마다 도는 fsync JSON 쓰기)를
  `git_stage` 형태로 이관했다. **락 의미 불변**: `begin_mutation` 은 async 쪽에서 잡고 그대로 들고
  블로킹으로 들어간다(`begin_mutation_blocking` 은 쓰지 않는다 — 대기가 블로킹 풀 스레드를 점유해
  홀더와 교착할 수 있다는 `state.rs` 의 규약 그대로). 가드 없는 두 커맨드(`file_open`·
  `file_mirror_dirty`)는 기존 무가드 판정을 유지한다.
- `State<'_, AppState>` 는 `'static` 클로저로 못 넘기므로 `file_save`·`file_mirror_dirty` 는
  `app: AppHandle` 을 받아 클로저 안에서 `app.state::<AppState>()` 로 다시 빌린다
  (`project/commands.rs` 의 기존 패턴). `AppHandle` 은 specta 가 인자로 세지 않아 **bindings
  시그니처는 그대로**이고, `remote/dispatch.rs` 의 두 호출부만 `app.clone()` 을 추가했다.
- `file_save`/`file_copy` 가 "가드 보유 + 블로킹 풀 디스패치" 홀더 집합에 새로 들어갔으므로
  `state.rs::begin_mutation_blocking` 의 홀더 열거 doc 을 1줄 갱신했다(교착 여유 근거 문서 정합).

**검증**: `cargo fmt --all` 후 `cargo clippy --workspace --all-targets -- -D warnings` 무경고,
`cargo test --lib` 1121 전부 통과(file 28 · persist 11 · locale 16), `bun run typecheck` ·
`bun run lint`(0 error) · `bun run format:check` · `bun test` 1550 전부 통과.

### S3 git (2026-08-29)

파일: `src-tauri/src/domain/git/{service.rs,commands.rs}` · `src-tauri/src/domain/remote/dispatch.rs`
(인자 1줄) · `src-tauri/src/domain/locale/service.rs`(키 등록 2) ·
`src-tauri/resources/locales/{ko,en,ja}.json` · `src/shared/api/bindings.ts`(재생성) ·
`src/entities/git/git.ipc.ts` · `docs/ipc-contract.md`.

**(1) 버그6 §4-A-6 — untracked 디렉토리 행 stage 상시 실패**

- `collect_status_rows` 는 `recurse_untracked_dirs(false)` 라 미추적 디렉토리를 `nested/` **단일
  행**으로 접어 내려보내는데, `stage` 는 그 행을 `index.add_path` 로 넘겼다. libgit2 는 디렉토리를
  거절하고(`GIT_EDIRECTORY` — "cannot create blob from '…': it is a directory"), 트레일링 슬래시
  표기는 그보다 먼저 경로 검증에서 죽는다("invalid path: 'nested/'"). 둘 다 실측으로 재현했다.
  루프 중간에 `?` 로 빠져나가므로 `index.write` 에 도달하지 못해 **다중 선택 전체가 실패**했다.
- `absolute.is_dir()` 이면 `index.add_all([relative], IndexAddOption::DEFAULT, None)` 로 분기한다 —
  `git add nested/` 와 같은 호출이라 무시 파일은 그대로 건너뛰고, "이 아래 전부 stage" 라는 **UI
  행의 의미는 불변**이다. 파일·삭제 경로(`add_path`/`remove_path`)는 손대지 않았다.
- 트레일링 슬래시는 `without_trailing_separator`(신설, `Components::as_path` 로 절단)로 벗겨
  pathspec 에 넘긴다 — 세퍼레이터 리터럴을 손으로 쓰지 않기 위해서다.
- 추가 테스트 2: `미추적_디렉토리_행을_stage하면_하위_파일이_인덱스에_들어간다`(상대 경로 축 —
  `nested/a.txt`·`nested/deep/b.txt` 둘 다 added) · `미추적_디렉토리_행은_절대경로로_stage해도_
  하위_파일이_인덱스에_들어간다`(프론트가 `StatusRow::abs_path` 를 넘기는 축).

**(2) 버그10 §4-A-10 — 절대경로 discard 의 조용한 no-op**

- 근본 원인은 `to_repo_relative` 의 **비대칭**이다: 상대 경로는 `Path::to_path_buf` 가 원문을
  보존해 `nested/` 로 남고, 절대 경로는 `strip_prefix` 의 `Components::as_path` 가 트레일링
  세퍼레이터를 잘라 `nested` 로 온다. `discard` 는 이것을 상태 행의 `path`(`nested/`)와 **문자열**
  비교해 untracked 판정을 했으므로, 절대경로로 온 행은 tracked 로 분류되고 `checkout_index` 는
  미추적 디렉토리에 아무 일도 하지 않아 조용히 no-op 였다.
- 비교를 `without_trailing_separator(&row.path) == without_trailing_separator(relative)` 로 바꿨다 —
  `Path` 동등성이 컴포넌트 비교라 슬래시 유무 자체를 흡수한다. `to_repo_relative` 는 손대지 않았다
  (거기서 슬래시를 없애면 반대로 상대 경로 축이 어긋난다).
- 추가 테스트: `미추적_디렉토리_행은_절대경로로_discard해도_워킹트리에서_사라진다`.

**(3) §2 M-1 — 조회 7종 `spawn_blocking` + `git_show_file` 블롭 상한**

- `git_show_file`·`git_ahead_behind`·`git_remotes`·`git_gutter`·`git_current_user`·`git_branches`·
  `git_stash_list` 를 기존 `git_status` 와 **같은 모양**으로 이관했다(`spawn_blocking` → `.await` →
  `JoinError` 를 `AppError::Internal` 로). 전부 가드 없는 조회라 락 의미 변화는 없고, 시그니처·반환
  값도 불변이라 IPC 표면 변화가 아니다. 각 커맨드 doc 에 "왜 블로킹인가"(예: `git_gutter` 는
  `fs:changed` 마다 도는 거터 핫패스, `git_branches` 는 loose refs + packed-refs 워크)를 1~2줄로
  남겼다.
- `show_file` 은 `Odb::read_header` 로 **오브젝트 헤더만** 읽어 크기를 보고, `SHOW_FILE_MAX_BYTES`
  (= `constants::REFUSED_FILE_BYTES`, 50MB) 초과면 `error.git.blobTooLarge` 로 거절한다 — 초과
  블롭은 압축 해제조차 되지 않는다. 상한을 파일 도메인의 거절 티어로 고른 이유는 S1a 의 검색 상한과
  같다: `file_open` 이 거절하는 파일이 커밋 diff·파일 히스토리 탭을 통해서는 열리는 비대칭을 만들지
  않기 위해서다.
- 테스트 가능성을 위해 상한을 파라미터로 받는 `show_file_limited` 를 사이에 뒀다(50MB 블롭을
  실제로 만들지 않고 경계를 검증하기 위함). 추가 테스트:
  `show_file은_상한을_넘는_블롭을_거부하고_상한_이하는_돌려준다`(경계 포함 — 상한과 같은 크기는 통과).

**(4) §2 M-7 — `run_git` 타임아웃 + kill**

- `Command::output()` 을 `run_command_with_timeout(command, timeout)` 으로 대체했다. `output()` 과
  **동일한 stdio 구성**(stdin null · stdout/stderr piped)을 명시적으로 세팅하므로, 관측 가능한
  차이는 "죽는다"뿐이다.
- 파이프는 **각자 스레드**가 EOF 까지 드레인하고 종료 상태만 폴링한다
  (`GIT_COMMAND_POLL_INTERVAL_MS` 20ms). 폴링만 하고 파이프를 비우지 않으면 `git push`/`pull` 의
  진행 출력이 파이프 버퍼(보통 64KiB)를 채워 자식이 막히고, 타임아웃이 오히려 교착을 만든다 —
  이 실패 모드를 테스트로 고정했다.
- **kill 경로에서는 리더 스레드를 join 하지 않는다**: `kill` 은 `git` 프로세스에만 닿고 자식이
  남긴 손자(ssh·credential helper)가 쓰기 끝을 쥐고 있으면 read 가 계속 막혀, 타임아웃이 막으려던
  바로 그 행을 만든다. 스레드는 마지막 writer 가 닫힐 때 스스로 끝나고 버퍼는 읽히지 않은 채
  버려진다(그 경로에서 출력을 원하는 소비자가 없다).
- `GIT_COMMAND_TIMEOUT_SECS = 300`. 느린 원격의 `push`/`pull`/`fetch` 와 큰 워킹트리의
  `commit`(`add -A`)이 정당하게 분 단위를 쓰고, `pull` 을 통합 도중에 죽이는 것 자체가 파괴적이라
  넉넉하게 잡았다 — 이 상한의 목적은 "느린 것"이 아니라 감사가 지적한 **무한**(도달 불가 원격·막힌
  자격증명 프롬프트)을 끊는 것이다. `git_pull` 의 전체 guard 유지와 `push_fetch_lock` 은 계약대로
  **불변**이며, 타임아웃만으로 그 락들의 보유 시간이 앱 수명만큼 늘어나는 경로가 사라진다.
- 실패는 `error.git.commandTimedOut`(args: `command`·`seconds`). 추가 테스트 3(모두 `#[cfg(unix)]`,
  기존 `lsp_proc`·`ide/lockfile` 테스트의 관례를 따름):
  `상한을_넘긴_서브프로세스는_강제_종료되고_none을_돌려준다`(30초 sleep 을 200ms 상한으로) ·
  `상한_안에_끝난_서브프로세스는_stdout과_stderr를_그대로_돌려준다` ·
  `파이프_버퍼보다_큰_출력도_교착_없이_수집된다`(512KiB).

**(5) §4-B B11 Rust 부분 — diff 의 원경로(beforePath) 축**

- `diff_file` 이 `before_path: Option<&str>` 를 받아 **원본(왼쪽) 측만** 그 경로로 읽는다. 오른쪽은
  항상 `path` 다. `IndexVsHead` 는 HEAD blob 을, `WorkdirVsIndex` 는 인덱스 blob 을 원경로에서
  읽으므로 스테이지된 개명(HEAD 항목이 옛 경로)과 스테이지되지 않은 개명(인덱스 항목이 옛 경로)
  양쪽이 같은 축 하나로 해결된다. `None` 이면 기존 동작 그대로다.
- `git_diff_file` 커맨드에 `before_path: Option<String>` 인자 순증 → `bindings.ts` 재생성
  (`gitDiffFile(projectId, path, mode, beforePath)`), `remote/dispatch.rs` 에 `arg!(args,
  "beforePath")` 1줄. 커맨드 이름·개수·반환 타입 불변이라 dispatch 파리티·원격 정책·커맨드 수(41)
  변동 없음.
- FE 는 계약대로 **표면까지만**: `entities/git/git.ipc.ts` 의 `getGitDiffFile` 이 선택 인자
  `beforePath?: string | null` 을 받아 `?? null` 로 넘긴다. `gitDiffFileQueryOptions`·
  `QUERY_KEY.GIT.DIFF`·`diff-pane.tsx` 는 손대지 않았다(F3 몫 — §5 에 F3 가 지켜야 할 것 기록).
- 추가 테스트 2: `스테이지된_개명의_diff는_원경로를_주면_head_blob을_원본으로_쓴다`(원경로 없이는
  왼쪽이 빈 문자열 = B11 증상 자체를 함께 고정) · `스테이지되지_않은_개명의_diff는_원경로의_
  인덱스_blob을_원본으로_쓴다`.

**부수**: `bindings.ts` 재생성이 S1a·S2 가 커맨드 doc 을 고친 뒤 갱신하지 않고 남긴 분량
(`fileOpen`/`fileSave`/`fileCopy`/`searchReplace` 등의 doc, `OpenedFile.encodingLossy`,
`ReplaceSkipReason`/`ReplaceSkippedFile`, `searchRun` 의 `Channel<SearchFileMatches>`)을 함께
반영했다 — 생성물이라 부분 재생성이 불가능하고, 재생성 결과가 정본이다.

**검증**: `cargo fmt --all -- --check` 통과, `cargo clippy --workspace --all-targets -- -D warnings`
무경고, `cargo test --lib` 1130 전부 통과(git 97 · locale 16 · remote 133), `bun test`
(entities/git · widgets/git-panel · app/providers) 70 통과. `bun run typecheck` 는 git 관련 오류 0 —
동시 진행 중인 d-51 F2 가 `features/search/search-panel.tsx` 를 편집 중이라 그 파일의 오류만 남아
있고, 이 스테이지가 만진 표면과는 무관하다(전체 사다리는 최종 검증 스테이지).

### S4 terminal (2026-08-29)

파일: `src-tauri/src/domain/terminal/{commands.rs,service.rs}` · `src-tauri/src/infra/pty.rs` ·
`src/shared/api/bindings.ts`(재생성 — `ptyWrite` doc 1건) · `docs/ipc-contract.md` ·
`docs/features/terminal.md`.

**(1) 버그5 §4-A-5 — `pty_attach` 리플레이·브로드캐스트 원자성**

- 근본 원인은 **락이 둘이라는 것 자체**였다. reader 는 `ring_buffer.lock()` → `subscribers.lock()`
  을 차례로 잡고(append 후 broadcast), `pty_attach` 는 같은 순서로 잡았다(snapshot 후 push).
  스냅샷 이후·등록 이전에 도착한 청크는 리플레이에도 브로드캐스트에도 없어 **유실**되고, 반대
  인터리빙(append→snapshot→push→broadcast)에서는 **중복** 도착한다. 탭 전환마다 재-attach 가
  일어나므로 실사용 빈도가 낮지 않다.
- `SessionOutput{ scrollback, subscribers, next_subscription_id }` 를 신설해 **세 상태를 하나의
  `Mutex` 아래**로 합쳤다. `append_and_broadcast()`(reader 경로)와 `attach()`(리플레이+등록)가 각각
  하나의 임계구역이라, 구독자가 받는 바이트열은 항상 `리플레이 ++ 이후 전량` 이고 경계에서 중복도
  유실도 없다. `next_subscription_id` 는 `Arc<AtomicU32>` 에서 락 아래 평범한 `u32` 로 내려왔다
  (같은 락이 이미 순서를 정하므로 원자 변수가 불필요 — `fetch_add` 의 wrap 의미는 `wrapping_add`
  로 보존).
- 락 유지 범위에 `Channel::send` 가 들어오지만 비용 증가는 없다 — `send` 는 웹뷰 IPC 로 큐잉만
  하고 JS 드레인을 기다리지 않는다(기존 doc 이 이미 근거로 삼던 성질). 락 순서도 안전하다:
  reader 는 `SessionOutput` 락을 놓은 **뒤에** `report_cwd_change` 가 `TerminalStore` 락을 잡고,
  attach/detach 는 `TerminalStore` → `SessionOutput` 순으로만 잡아 사이클이 없다.
- 재현 실측: 분리 락 형태를 스크래치 바이너리로 재구성해 1바이트 청크 4,096개 스트림 중간에
  attach 를 끼워 넣으면 64회 중 3회 "받은 바이트 합 ≠ 전체 스트림"(4,092 vs 4,096 — 청크 4개
  유실)이 나왔다. 실제 코드는 리플레이가 최대 2MB 를 `send` 하므로 창(window)이 이보다 훨씬 넓다.
- 추가 테스트 4: `attach_중에는_reader_의_출력이_끼어들_수_없다`(attach 가 락을 쥔 동안 reader
  경로의 append 가 진행되지 못함을 스레드로 확정 — 원자성의 결정적 근거) ·
  `스트리밍_중_attach_해도_전체_스트림이_정확히_한_번씩_재구성된다`(경합 소크 16회 × 4,096청크,
  받은 바이트 이어붙이기 == 전체 스트림) · `attach_는_직전까지의_스크롤백만_재생하고_이후_출력은_
  브로드캐스트로_잇는다` · `스크롤백이_비어_있으면_attach_는_아무것도_재생하지_않는다`.
  기존 `detach_은_해당_구독_id_만_제거하고_나머지는_유지한다` 는 손으로 `retain` 하던 것을 실제
  `SessionOutput::attach`/`detach` 를 쓰도록 바꿔 커버리지가 올라갔다.

**(2) §2 M-5 — 링버퍼 `Vec::drain` 쓰기 증폭**

- `ring_buffer_append(&mut Vec<u8>, ..)` 자유 함수를 `ScrollbackRing`(`VecDeque<u8>` + capacity)
  타입으로 대체했다. 상한(2MB)에 닿은 뒤에는 유입 64KB 마다 남은 2MB 전량을 `memmove` 하던 것이,
  head 전진으로 **유입 크기에만 비례**하는 비용이 된다.
- 용량 이상인 단일 청크는 append 전에 꼬리만 잘라 담는다 — 담았다가 버리면 버퍼가 순간적으로
  상한을 넘게 되고, 그게 바로 상한을 두는 이유(대량 출력 스파이크)와 정면으로 어긋난다.
- 리플레이는 `as_slices()` 의 두 조각을 앞→뒤로 보낸다(**최대 2청크** — 되감기기 전에는 1청크).
  `make_contiguous()` 로 1청크를 유지할 수도 있었지만 그것은 attach 마다 2MB rotate 이고, 소비부
  (xterm)는 reader 배칭 때문에 이미 write 경계를 넘는 파서 상태를 보존한다(`terminal.md` §12.3
  반증 항목)이라 2청크가 관측 가능한 손해가 없다. 계약은 `docs/ipc-contract.md` "d-50 S4" 에 명기.
- 추가 테스트 2(+ 기존 2건은 새 타입 기준으로 이관): `용량보다_큰_청크는_꼬리만_남긴다` ·
  `되감긴_링도_두_조각을_이어_붙이면_순서가_보존된다`(13바이트 × 16회로 실제 wrap 을 만든 뒤
  두 조각 이어붙이기 == 마지막 capacity 바이트).

**(3) §2 M-6 — `pty_write` 자식 stdin 블로킹 쓰기**

- 쓰기를 `tauri::async_runtime::spawn_blocking` 으로 옮겼다. writer 핸들(`Arc<Mutex<..>>`)을 먼저
  복제해 두므로 클로저가 `State`(`'static` 아님)를 잡지 않고, **커맨드 시그니처가 그대로**라
  bindings·dispatch·원격 정책은 변동 없다(생성물에는 doc 주석 1건만 반영).
- `TerminalStore` 락은 예전 그대로 "핸들 조회까지만" 이다(T0 감사 #20 결정 불변). 이번 변경은
  그 위에 "블로킹 대기가 async 워커 스레드도 점유하지 않는다"를 더한 것.

**(4) §2 L-2 — flusher 5ms 상시 wakeup**

- `AtomicBool draining` + `sleep(5ms)` 무조건 루프를 `FlushSignal`(parking_lot `Mutex<FlushState>`
  + `Condvar`, `PauseGate` 와 같은 관용구)로 교체했다. reader 는 매 read 후
  `set_pending(batch.has_pending())` 로 **배치에 남은 게 있을 때만** 깨우고, 스스로 flush 한
  경우(크기·시간 임계 도달)에는 플래그를 내려 헛 wakeup 을 만들지 않는다. 유휴 세션은 condvar 에
  주차해 비용 0(이전: 세션당 초당 200회).
- 지연 규약은 불변이다 — 깨어난 뒤 `OUTPUT_FLUSH_TICK_MS`(5ms) 를 기다렸다가 flush 하므로 버스트가
  read 단위로 쪼개지지 않는다. 종료는 reader 가 `stop()`(stopped=true + `notify_all`) 후 마지막
  배치를 직접 flush 하고, 플러셔는 다음 대기에서 `false` 를 받아 루프를 끝낸다 — `PtySession::drop`
  의 "일시정지된 채 죽어도 스레드가 새지 않는다" 보장은 그대로다(그 doc 의 `draining` 언급만 갱신).
- 플러셔 루프 본문을 `run_flusher(signal, batch)` 로 빼내 pty 를 스폰하지 않고 테스트한다.
  추가 테스트 4: `대기중인_데이터가_없으면_플러셔는_깨어나지_않는다`(L-2 의 요점 자체 — 대기 스레드가
  60ms 안에 깨어나지 않고, `set_pending(true)` 후에는 즉시 깨어난다) · `대기는_pending_플래그를_
  소비한다` · `reader_가_끝나면_플러셔_루프도_끝난다` · `배치에_남은_청크는_플러셔가_내보낸다`.

**검증**: `cargo fmt --all -- --check` 통과, `cargo clippy --workspace --all-targets -- -D warnings`
무경고, `cargo test --lib` 1140 전부 통과(terminal 22 · pty 16), `bun run typecheck` 오류 0,
`bun run format:check` 통과, `bun test`(entities/terminal · widgets/terminal-pane) 17 통과.

### S5 lsp (2026-08-29)

파일: `src-tauri/src/domain/lsp/{service.rs,commands.rs}` · `src-tauri/src/infra/lsp_proc.rs` ·
`docs/ipc-contract.md` · `docs/features/lsp.md`. **TS 변경 0 · IPC 표면 변경 0**(커맨드·인자·반환·
이벤트·타입 전부 불변 → `bindings.ts` 재생성 불요, dispatch·원격 정책·로케일 키 변동 없음).

**(1) 버그7 §4-A-7 — workspace folder URI 미인코딩**

- 근본 원인은 표기 규약이 **두 곳에서 따로** 만들어진다는 것이다. 프론트는
  `shared/lib/lsp/initialize-params.ts` 가 `monaco.Uri.file(root).toString()` 로 `initialize` 의
  `rootUri`/`workspaceFolders` 를 만들고, Rust 는 `workspace_folder_json` 이 원시
  `format!("file://{root}")` 로 `workspace/didChangeWorkspaceFolders` 를 만들었다. 서버는 폴더 집합을
  URI **문자열**로 식별하므로 `file:///w/my project`(Rust) 와 `file:///w/my%20project`(FE) 는 서로
  다른 폴더다 — 멀티루트에서 추가 루트가 중복 인덱싱되고, 제거는 대상이 없어 무시된다.
- `service::workspace_folder_uri(root)` 신설이 monaco 표기를 재현한다. 인코딩은 RFC 3986 퍼센트
  인코딩 — `unreserved`(ALPHA·DIGIT·`-`·`.`·`_`·`~`)와 경로 구분자 `/` 만 원문 유지, 나머지 바이트는
  UTF-8 대문자 16진 `%XX`. 표준의 `pchar` 보다 좁게(하위 구분자 `!$&'()*+,;=` 와 `:@[]` 까지 escape)
  잡은 이유는 monaco 의 `encodeTable` 이 정확히 그것들을 escape 하기 때문이다 — 목적이 "표준 준수"가
  아니라 **양쪽 표기 일치**라서 monaco 를 기준으로 삼았다(퍼센트 인코딩된 하위 구분자도 RFC 3986 상
  같은 리소스). monaco 의 나머지 두 경로 보정도 함께 재현했다: 상대 경로에 루트 `/` 부여, Windows
  드라이브 문자 소문자화(monaco 는 플랫폼과 무관하게 소문자화하므로 cfg 게이트 없음). 역슬래시→
  슬래시만 monaco 가 `isWindows` 로 가르므로 `cfg!(windows)` 로 맞췄다.
- **실측 대조**: `node_modules/monaco-editor/esm/vs/base/common/uri.js` 를 직접 실행해
  `URI.file()` 의 출력을 뽑고(공백·한글·`+&,;=`·`%`·`[]@:`·이모지·드라이브 경로), 그 문자열을 Rust
  테스트의 기대값 리터럴로 고정했다. 수정 전 값이 `file:///workspace/my project`(미인코딩)라는 것도
  같은 방식으로 확인했다.
- 추가 테스트(Rust 5): `공백과_한글이_있는_루트는_프론트와_같은_퍼센트_인코딩_uri가_된다` ·
  `인코딩한_루트는_원래_경로로_되돌아온다`(테스트 전용 역디코더로 6경로 왕복 — 바이트 유실·이중
  이스케이프 방지) · `예약되지_않은_문자와_경로_구분자는_그대로_둔다` ·
  `하위_구분자와_예약_문자도_프론트처럼_인코딩한다` ·
  `드라이브_문자는_소문자로_맞추고_상대_경로는_루트를_붙인다` · (commands 축)
  `워크스페이스_폴더_알림의_uri는_퍼센트_인코딩된다` — 알림 JSON 전문에서 added/removed 양쪽 확인.

**(2) 버그11 §4-A-11 — `Content-Length` 없는 헤더 블록의 영구 wedge**

- `try_take_message` 는 `parse_content_length` 가 `None` 이면 **버퍼를 그대로 둔 채** `None` 을
  돌려줬다. 그 헤더 블록은 버퍼 맨 앞에 남고, 이후 모든 read 는 같은 블록의 `\r\n\r\n` 을 다시 찾아
  또 `None` 을 돌려준다 — 그 시점부터 **세션이 영구 무응답**이 되고 버퍼는 서버가 말하는 동안 계속
  커진다. 재현: 수정 전 함수를 스크래치 바이너리로 떼어내 `X-Server-Note: ...\r\n\r\n` 뒤에 정상
  프레임을 붙여도 `None`(버퍼 56바이트 잔류)이 나오는 것을 실측했다.
- `MessageBuffer::take_message` 가 그 블록만 버리고(`consumed` 를 블록 끝으로 전진) **다음 프레임
  탐색을 이어간다**. 잡음의 현실적 출처는 서버가 stdout 에 흘리는 패닉·로그이고, 버리는 비용은
  잡음 자신뿐이다. **바디가 UTF-8 이 아닌 프레임도 같은 규칙으로 건너뛴다** — 기존 코드는 그 프레임을
  소비하고도 `None` 을 돌려줘 리더의 드레인 루프가 끊겼고(다음 read 가 올 때까지 뒤 메시지가 정체),
  이제는 같은 루프에서 다음 메시지로 넘어간다.
- 추가 테스트 4: `content_length_없는_헤더_블록은_버리고_다음_프레임을_계속_찾는다`(수정 전 실패 —
  잡음 뒤 정상 프레임이 영원히 안 나오는 것이 증상 자체) · `불량_헤더가_같은_읽기에_섞여_와도_
  뒤따르는_메시지를_꺼낸다` · `불량_헤더는_앞뒤의_정상_메시지를_잡아먹지_않는다` ·
  `utf8가_아닌_바디는_건너뛰고_다음_메시지를_꺼낸다`.

**(3) §2 L-3 — `try_wait` 50ms 폴링 · 수신 버퍼 drain memmove**

- **대기**: `loop { try_wait; sleep(50ms) }` 을 `tokio::select!{ child.wait(), kill_signal.notified() }`
  로 교체했다. 유휴 서버는 wakeup 비용 0(이전: 서버당 초당 20회)이고 실제 종료는 즉시 관측된다.
  `Child::wait` 은 cancel-safe 라 kill 분기가 이겨 future 가 드롭돼도 잃는 것이 없고, 그 분기는
  `start_kill()` 후 다시 `wait().await` 로 자식을 거둔다. `kill_requested: AtomicBool` 은
  `kill_signal: Arc<Notify>` 로 대체 — `Notify` 는 대기자가 없을 때 permit 을 저장하므로 `kill()` 이
  태스크가 park 하기 전에 호출돼도 알림이 유실되지 않는다. `kill()` 이 `sysinfo` 로 자기 스택에서
  OS kill 을 보내는 기존 계약(앱 종료 시 고아 방지)과 `exited` 가드(PID 재사용 방지)는 불변이다.
- **수신 버퍼**: `try_take_message(&mut Vec<u8>)` 자유 함수를 `MessageBuffer{ data, consumed }` 로
  대체했다. 메시지마다 `drain(0..body_end)`(= 남은 전량 memmove)하던 것이 정수 전진이 되고, 실제
  이동은 `compact()` 에서 **소비 프리픽스가 옮길 바이트 수 이상일 때만** 일어난다(바이트당 상환
  O(1)). 흔한 경로는 아예 이동이 없다 — read 를 끝까지 드레인하면 `consumed == len` 이라 버퍼를
  비운다. 리더 태스크는 `buffer.extend(&read_buf[..n])` + `while let Some(m) = buffer.take_message()`
  로 형태만 바뀌었다.
- 추가 테스트 2: `부분_프레임이_남은_채로_메시지를_꺼내도_이어지는_바이트를_잃지_않는다`(부분 프레임을
  남긴 채 커서 전진·compaction 을 넘나들며 64회 왕복 — 커서 rebase 오류를 잡는 축) ·
  `kill_이후에는_wait_태스크가_프로세스를_거두고_is_exited를_세운다`(select 의 kill 분기 end-to-end —
  두 번째 `wait()`/`store` 를 빠뜨리면 종료된 서버가 영원히 "실행 중"으로 보고된다).
  기존 프레이밍 테스트 6건은 `MessageBuffer` 기준으로 이관하되 "버퍼 무변경" 단언은 **"나머지가
  도착하면 그 메시지가 나온다"** 는 행위 단언으로 바꿔 커버리지를 유지했다(내부 상태 접근자를 테스트
  전용으로 새로 노출하지 않기 위해서다).

**검증**: `cargo fmt --all -- --check` 통과, `cargo clippy --workspace --all-targets -- -D warnings`
무경고, `cargo test --lib` 1152 전부 통과(lsp 필터 100 — lsp 도메인·lsp_proc·lsp_install 포함).
TS 변경이 없어 `bun` 사다리는 이 스테이지의 대상이 아니다(전체 사다리는 최종 검증 스테이지).

### S6 infra 소형 (2026-08-29)

파일: `src-tauri/src/infra/watcher.rs` · `src-tauri/src/infra/range_file.rs` ·
`src-tauri/src/infra/asset_protocol.rs`(테스트 1) · `src-tauri/src/domain/file/capability.rs`(호출부) ·
`src-tauri/src/domain/git/watch.rs`(호출부 1줄) · `src-tauri/src/domain/agent/{commands.rs,service.rs}` ·
`docs/ipc-contract.md` · `docs/architecture.md`(1문장) · `docs/features/{explorer-sidebar,agent-integration}.md`.
**IPC 표면 변경 0**(커맨드·인자·반환·이벤트·타입 전부 불변 → `bindings.ts` 재생성 불요, dispatch·
원격 정책·로케일 키 변동 없음). **TS 변경 0.**

**(1) 버그4 §4-A-4 — 워처 무시 판정이 파일명·git ref 성분까지 먹던 문제**

- 이전 수정(T0 §2.4 #1)이 "루트 **위**의 조상"만 판정에서 뺐고, 남은 두 구멍이 이번 건이다:
  ① 판정이 `components()` 전체라 **마지막 성분**(이벤트 대상 자신)까지 이름 비교 대상이었다 —
  무시 목록은 *디렉토리* 이름인데 트리는 `read_children` 에서 **디렉토리일 때만** 그 이름을 감추므로,
  루트의 `build`·`dist` 라는 **파일**은 화면에 보이면서 변경 이벤트만 영원히 드롭됐다.
  ② `.git` 을 루트로 도는 git 워처에는 이 목록이 애초에 의미가 없다 — `refs/heads/**` 는 브랜치
  이름이라 `build/login`·`node_modules` 같은 평범한 브랜치의 ref 이벤트가 통째로 사라졌다
  (루트 하위 첫 성분만 보는 게 아니라 **모든** 성분을 봤으므로 `refs/heads/build/login` 도 드롭).
- 수정 ①: `is_ignored_path` 가 `strip_prefix(root)` 결과의 **`parent()`**(=조상 디렉토리 성분)만
  훑는다. 마지막 성분이 디렉토리인지 파일인지는 경로만으로 알 수 없고(이미 삭제됐을 수 있다),
  무시 디렉토리 **자신**의 생성/삭제 이벤트 1건이 새는 것이 파일 모양 구멍을 남기는 것보다 싸다 —
  그 하위 경로는 종전대로 전부 걸러지므로 `node_modules` 설치 폭풍은 그대로 막힌다. 루트 밖 경로
  (심볼릭 링크 등)의 전체 경로 fallback 은 유지했다.
- 수정 ②: `WatchScope{ Project, GitDir }` 를 신설해 `start_watch(root, scope, on_change)` 가 받고,
  `GitDir` 은 이름 필터를 **아예 적용하지 않는다**. 잡음 차단은 하류의 `classify_git_change`
  (`index`·`HEAD`·`refs/**` 만 통과, `objects/**` 제외)가 이미 하고 있어 이중 필터가 불필요하다.
  호출부는 `domain::file::capability`(=`Project`)·`domain::git::watch`(=`GitDir`) 둘뿐이다.
- 추가 테스트 4(모두 수정 전 실패를 실측): `루트의_무시_이름_파일_변경은_그룹에_남는다` ·
  `git_워처는_무시_이름_성분이_있는_브랜치_ref_도_그룹에_남긴다`(둘 다 `group_relevant_changes`
  end-to-end — 증상 자체) · `무시_판정은_마지막_성분을_보지_않는다`(조상 성분이면 그대로 무시되는
  것까지 같이 고정) · `git_워처는_무시_목록을_적용하지_않는다`. 기존 4건은 새 시그니처로 이관하되,
  루트가 `.git` 인 케이스를 다루던 `감시_루트_자체가_git_디렉토리면...` 은 더 강한 `GitDir` 축
  테스트로 대체했다.

**(2) 버그9 §4-A-9 — `Range: bytes=-N` 접미 범위 오해석**

- `start_raw.is_empty()` 를 "start = 0" 으로 읽어 `bytes=-500` 이 **앞에서 501바이트**를 돌려줬다.
  RFC 7233 §2.1 의 `suffix-byte-range-spec` 은 **마지막 N 바이트**이고, 이게 mp4 의 `moov` atom 이
  파일 끝에 있는 경우 미디어 엘리먼트가 가장 먼저 보내는 요청이라 프리뷰 탐색이 깨진다.
- `parse_range` 를 접미/일반 두 분기로 갈랐다: 접미는 `start = total.saturating_sub(N)`,
  `N > total` 이면 전체(RFC 대로), `N == 0` 은 **만족 불가**(`None` → 416; RFC 가 유일하게 오류로
  못박는 접미 형태). 상한 적용(`RANGE_CHUNK_LIMIT`·`total-1`)은 두 분기가 **같은 한 줄**을 공유하도록
  묶어 접미 범위에도 청크 상한이 그대로 걸리게 했다(응답이 요청보다 짧을 수 있고, 그 사실은
  `Content-Range` 가 말한다 — 클라이언트가 이어서 요청하는 표준 동작). `total == 0` 가드를 함수
  앞으로 올려 `total - 1` 이 두 분기 모두에서 안전하다.
- 두 서빙 경로(`domain::remote::serving::file_range` · `infra::asset_protocol::respond`)는 같은
  `parse_range` 를 쓰므로 **호출부 수정 0**.
- 추가 테스트 5: `접미_범위는_파일의_마지막_n_바이트를_가리킨다` ·
  `접미_길이가_파일보다_크면_파일_전체를_가리킨다` · `접미_길이가_0이면_만족할_수_없는_범위다` ·
  `접미_범위도_한_청크_상한을_넘지_않는다` · (응답 축) `접미_범위_요청은_파일의_마지막_바이트들을_반환한다`
  (`asset_protocol` 에서 206 바디와 `Content-Range: bytes 7-9/10` 까지 확인 — 기존 `bytes=2-4` 테스트와
  같은 모양).

**(3) §2 M-3 — 에이전트 폴링의 pid 당 `ps` fork**

- unix 프로브가 pid 마다 `ps` 를 fork 했다(폴링 1s × 프로젝트 × pty 세션 수). `resolve_process_info(pid)`
  를 `resolve_process_infos(&[u32]) -> HashMap<u32, ProcessInfo>` 로 바꿔 `ps -o pid=,state=,comm=,args=
  -p pid1,pid2,...` **1회**로 합쳤다. pid 열을 추가한 이유가 곧 배칭의 전제다 — 죽은 pid 는 출력에서
  그냥 빠지므로 위치로 zip 할 수 없고 **맵 조회**로 가려내야 한다(실측: `ps -p 65000,1` → 살아 있는
  1만 출력·exit 0, 전부 죽었으면 exit 1 + 빈 출력). 그래서 `status.success()` 를 보지 않는다 —
  일부만 죽은 배치의 생존자를 버리게 된다.
- 파싱은 순수 함수 `service::parse_ps_process_infos(stdout)` 로 분리해 플랫폼과 무관하게 테스트한다.
  해석 불가 행(예: `ps` 가 뱉는 오류 줄)은 그 행만 건너뛰고 배치를 유지한다.
- **세션 0 조기 반환**: `detect_agents_for_pids_blocking` 이 빈 pid 목록이면 `spawn_blocking` 디스패치
  없이 즉시 빈 결과를 돌려준다(터미널을 연 적 없는 프로젝트 = 흔한 경우). `poll_agents` 자체는 손대지
  않았다 — 거기서 `continue` 하면 "세션이 사라져 에이전트가 0이 됐다"는 **변화 자체가 `agents.diff` 에
  도달하지 못해** 배지가 켜진 채 남는다. unix `detect_agents_for_pids` 에도 같은 가드를 둬 windows
  분기와 형태를 맞췄다.
- 추가 테스트 5: (service) `ps_배치_출력은_pid별로_분해된다`(실제 macOS 출력 형태 — 패딩·args 공백
  포함, `detect_agent_name` 까지 이어서 확인) · `해석할_수_없는_행이_섞여도_나머지_pid는_남는다` ·
  `이미_종료된_pid는_결과에_없다` · (commands) `프로세스_조회는_한_번의_ps_호출로_살아있는_pid를_해석한다`
  (실제 `ps` 를 자기 pid 로 호출 — 포맷 문자열·중복 pid 축) · `pty_세션이_없으면_프로세스를_조회하지_않는다`.

**검증**: `cargo fmt --all -- --check` 통과, `cargo clippy --workspace --all-targets -- -D warnings`
무경고, `cargo test --lib` 1165 전부 통과(watcher 10 · range_file 10 · asset_protocol 9 · agent 92).
TS 변경이 없어 `bun` 사다리는 이 스테이지의 대상이 아니다(전체 사다리는 최종 검증 스테이지).

### S7 tree (2026-08-29)

파일: `src-tauri/src/domain/tree/{service.rs,commands.rs}` · `docs/ipc-contract.md` ·
`docs/features/explorer-sidebar.md`. **IPC 표면 변경 0**(커맨드 이름·인자·반환 타입·이벤트 전부
불변 → `bindings.ts` 재생성 불요, dispatch·원격 정책·로케일 키 변동 없음). **TS 변경 0.**

**(1) §2 H-4 (계약 불변 범위) — 락 밖 선행 `read_dir`**

- 근본 형태는 "읽기와 삽입이 같은 임계구역에 있다" 였다. `read_children` 은 동기 `read_dir` +
  하위 디렉토리마다 `has_children` 용 `read_dir` 한 번이라, 차가운 폴더 하나를 펼치는 동안
  **전역 `begin_mutation` guard + `TreeStore` 쓰기 락**이 잡힌 채 async 워커 스레드까지 점유했다
  (파일·git 뮤테이션 전체가 그 뒤에 줄을 섰다).
- 커맨드를 **계획 → 선행 읽기 → 잠금** 3단으로 갈랐다. ① `plan_reads` 가 스토어 **읽기 락만**
  잠깐 잡고 이번 호출이 읽어야 할 디렉토리를 고른다(`service::plan_root_read`/`plan_toggle_reads`/
  `plan_reveal_reads`/`plan_refresh_reads` — 각 연산의 실제 읽기 조건을 그대로 미러). ②
  `prefetch_listings` 가 `spawn_blocking` 으로 `service::read_directories` 를 돌려
  `DirectoryListings` 를 만든다(빈 계획이면 디스패치조차 하지 않는다 — 접는 토글·캐시 히트 스크롤이
  흔한 경로다). ③ guard + 쓰기 락을 잡고 `ensure_entry`/`toggle_expand`/`reveal`/`invalidate` 가
  `take_children` 으로 **미리 읽은 목록을 꺼내 꽂기만** 한다.
- **계획은 힌트이지 정확성 입력이 아니다.** 읽기에 실패했거나 계획 이후 상태가 바뀌어 목록에 없는
  디렉토리는 잠금 안에서 예전처럼 그 자리에서 읽는다 — 그래서 오류 표면(존재하지 않는 디렉토리 등)이
  기존과 완전히 같다. 이 성질을 테스트로 고정했다.
- `tree_rows` 의 미스 경로도 같은 선행 읽기를 쓴다. 원래도 모든 락 **밖**이었지만 async 워커에서
  동기 루트 스캔을 돌리는 것은 같았다. 히트(스크롤 핫패스)는 계획이 비어 추가 비용 0이다.
- 추가 테스트 3: `미리_읽은_목록은_잠금_안에서_디스크를_다시_읽지_않는다`(선행 읽기 후 디렉토리를
  **삭제**하고 `expand` — 잠금 안에서 디스크를 만졌다면 실패한다. 선행 읽기를 무력화하면 실제로
  실패하는 것을 실측) · `미리_읽지_못한_디렉토리는_잠금_안에서_같은_오류로_이어진다` ·
  `읽기_계획은_캐시에_없는_디렉토리만_고른다`(접는 토글 = 빈 계획, 펼쳐지지 않은 디렉토리의 갱신 =
  빈 계획까지 함께 고정).

**(2) §2 H-4 — `flatten` 창(window) 순회**

- `rows_page` 는 전체 트리를 `TreeRow` 로 만든 뒤 슬라이스해 버렸다 — 행마다 `path`·`name`
  `String` 2개를 할당하고 창 밖은 그대로 폐기했다. `PageCursor{start, end, index, rows}` 를 들고
  도는 `push_page_rows` 가 **창 안에서만** `TreeRow` 를 만든다.
- **완전 조기 종료는 하지 않는다**: `TreeRowPage::total` 이 전체 가시 행 수라 순회 자체는 끝까지
  가야 한다. 창 밖 행의 비용은 하강 판단에 어차피 필요한 `expanded` 조회뿐으로 줄었다(할당 0).
- `flatten`/`full_page` 는 `rows_page(state, 0, None)` 위의 얇은 래퍼가 되어 평탄화 구현이 하나로
  합쳐졌다(옛 `push_children` 제거).
- 추가 테스트: `rows_page_는_창_밖_행을_만들지_않고도_전체_수를_센다`(펼쳐진 하위 트리 **중간**에서
  시작하는 창의 내용·깊이와 `total` 을 함께 고정 — 기존 경계 테스트는 루트 축만 봤다).

**(3) §2 H-4 — `entry_order` 정렬 키 사전 계산**

- 비교마다 양쪽 이름을 `to_lowercase()` 하던 comparator(`sort_by`)를 `sort_by_cached_key` +
  `entry_sort_key(entry) -> (u8, String)` 로 바꿨다. 항목 n개짜리 목록의 할당이 ~2·n·log n 에서
  n 으로 내려간다. 순서 규약은 그대로다 — `(kind_rank, lowercase_name)` 은 "디렉토리 먼저, 그다음
  대소문자 무시 이름"과 동치이고, `sort_by_cached_key` 도 안정 정렬이라 동률의 `read_dir` 순서
  보존까지 같다(기존 정렬 테스트 무수정 통과).

**(4) §4-B C2 — 폴더 삭제·개명 시 prefix 하위 정리**

- 증상은 "동명 재생성 시 유령 자식" 이지만 근본은 **갱신이 그 디렉토리의 행 목록 하나만 갈아 끼운다**
  는 것이었다. 폴더를 지우면 그 폴더의 캐시된 자식 목록과 **펼침 표시가 옛 경로에 그대로 남고**,
  같은 이름으로 폴더를 다시 만들면 `flatten` 이 `expanded` 히트 → 옛 `cache` 히트로 이어져 새 폴더가
  **펼쳐진 채 지운 폴더의 자식들로** 그려졌다. 남은 항목이 앱 수명 내내 두 맵에 쌓이는 문제도 같다.
- `invalidate` 가 새 목록을 꽂기 **전에** `prune_stale_descendants(state, dir, &fresh)` 를 돌린다.
  판정은 새 목록만으로 끝나 **추가 syscall 이 0** 이다:
  ① 캐시된 목록은 **자식이 남아 있는 자식 디렉토리 아래에서만** 살아남는다 — 지금 비어 있는 항목이
  캐시된 하위를 가질 수는 없으므로, 두 갱신 사이에 비워지거나 지웠다 다시 만들어진 폴더는 옛 행을
  더 이상 내놓지 못한다. ② 펼침 표시는 **존재하는 자식 디렉토리 아래면 빈 폴더라도** 살아남는다 —
  빈 폴더도 펼칠 수 있고, 표시를 지우면 사용자가 연 폴더가 조용히 접힌다(①이 이미 옛 목록을
  없앴으므로 비어 보일 뿐이다).
- 접두사 판정은 문자열이 아니라 `Path::strip_prefix` + 첫 컴포넌트(`child_of`)다 — `src` 를 지워도
  형제 `src-old` 는 하위가 아니다(테스트로 고정).
- 추가 테스트 4(앞 3건은 정리 코드를 빼면 실제로 실패하는 것을 실측):
  `삭제_후_동명_재생성된_폴더에는_옛_자식이_남지_않는다`(증상 자체 — 삭제 후 펼침 정리 확인 →
  동명 재생성 → 다시 펼치면 새 자식만) · `비워진_폴더의_캐시된_자식은_상위_갱신에서_버려진다`
  (①의 축, 펼침 표시는 유지되는 것까지) · `개명된_폴더의_옛_경로_상태는_상위_갱신에서_사라진다` ·
  `이름이_접두사인_형제는_정리_대상이_아니다`(과잉 정리 방지 축).

**검증**: `cargo fmt --all` 후 `cargo clippy --workspace --all-targets -- -D warnings` 무경고,
`cargo test --lib` 1173 전부 통과(tree 필터 21). TS·바인딩 변경이 없어 `bun` 사다리는 이 스테이지의
대상이 아니다(전체 사다리는 최종 검증 스테이지).

### S8 layout (2026-08-29)

파일: `src-tauri/src/domain/layout/{types.rs,service.rs,commands.rs}` · `src-tauri/src/lib.rs`(커맨드
등록 1줄) · `src-tauri/src/domain/remote/dispatch.rs`(구현 목록·허용 목록·match arm·개수 주석) ·
`src/shared/api/bindings.ts`(재생성) · `src/entities/layout/{layout.ipc.ts,layout.query.ts,
tab-path-change.ts(신규),tab-path-change.test.ts(신규)}` · `src/entities/file/file.query.ts` ·
`src/entities/editor/{model-registry.ts,model-registry.test.ts(신규)}` · `docs/ipc-contract.md` ·
`docs/features/{tabs.md,editor.md,explorer-sidebar.md}`. 로케일 키 변동 없음(신규 사용자 문구 0).

**(1) §4-B A3 — 신규 커맨드 `layout_apply_path_change(projectId, change) → TabPathChangeResult`**

- 근본은 "탭의 경로를 갱신하는 코드가 코드베이스에 아예 없다" 였다. 그래서 개명 후 탭은 사라진
  경로를 계속 가리켰고, 그 탭에서 ⌘S 하면 `write_atomic` 의 `create_dir_all` 이 옛 폴더를 되살려
  같은 문서가 두 경로로 갈라졌다(조용한 fork). 같은 파일을 새 이름으로 다시 열면 중복 탭이 됐다.
- 입력은 태그 union `TabPathChange`(`renamed{from,to}` / `deleted{path}`) 하나다. 커맨드를 둘로
  쪼개지 않은 이유: 두 경우 모두 "디스크에서 이미 끝난 경로 변경을 탭에 반영" 이라는 한 가지 일이고,
  dispatch 등재·원격 정책 분류·계약 기재가 커맨드 수만큼 늘어난다.
- 매칭은 **경로 성분 단위**(`Path::strip_prefix`)다. 문자열 접두사로 하면 `src` 개명이 형제
  `src-old` 를 끌고 간다(S7 의 `child_of` 판정과 같은 이유·같은 규칙). 디렉토리를 지목하면 하위
  전체가 대상이라 폴더 개명이 자동으로 커버된다.
- **재canonicalize 하지 않는다** — 개명이 끝난 시점에 `from` 은 디스크에 없다. 탭 경로도 `from` 도
  같은 출처(트리 행 = Rust 가 만든 정규 경로)라 문자열이 일치한다.
- 대상은 **`TabKind::File` 뿐**이다(§5 에 경계 기록). 열린 탭뿐 아니라 **닫은 탭 스택**도 함께
  옮긴다 — 재열기가 사라진 경로를 여는 것을 막는다.
- `deleted` 는 해당 경로(및 하위) 파일 탭을 **닫는다**(계약 채택 결정). 닫기는 기존 `close_tab` 을
  그대로 태워 닫은 탭 스택·활성 탭 인계·페인 정규화가 손으로 닫은 것과 동일하다.
- **대상이 없으면 무동작**: revision 증가·`layout:changed` 발행 모두 없다(빈 `moved`/`closedPaths`
  와 현재 레이아웃을 그대로 반환). 탭이 하나도 안 열린 프로젝트에서 개명할 때마다 전 창이 레이아웃을
  재조회하는 낭비를 막는다.
- 표면 등재: `lib.rs` `collect_commands!` · `IMPLEMENTED_JSON_COMMANDS` · `REMOTE_ALLOWED_COMMANDS`
  (형제 레이아웃 커맨드와 동일하게 허용 — 원격은 이미 `file_rename`/`file_delete`·`layout_close_tab`
  를 쓸 수 있고 이 커맨드는 그 조합보다 넓은 권한을 주지 않는다) · `dispatch` match arm ·
  허용 테이블 개수 주석(157→158). 커맨드 총수 178 → **179**. `bindings.ts` 재생성
  (`cargo test --lib typescript_바인딩을_생성한다`), `docs/ipc-contract.md` 에 "d-50 S8" 절 신설.
- 추가 테스트(Rust 7, `domain::layout::service`): `파일_개명은_열린_탭의_경로와_제목을_따라간다` ·
  `폴더_개명은_하위_열린_탭_전체의_경로를_치환한다`(보조 창 트리 포함) ·
  `이름이_접두사인_형제는_개명_대상이_아니다` ·
  `같은_파일이_두_페인에_열려_있으면_한_번만_보고하고_dirty_를_합친다` ·
  `개명은_닫은_탭_스택의_경로도_따라간다` · `삭제는_해당_경로와_하위_파일_탭만_닫는다` ·
  `대상_탭이_없으면_revision_을_올리지_않는다`.

**(2) A3 프론트 배선 — 경로가 들고 있던 상태 전부를 새 경로로 이관**

- 호출 지점은 위젯이 아니라 **`entities/file/file.query.ts` 의 `useRenameEntry`/`useDeleteEntry`
  `onSuccess`** 다. 인라인 개명·**잘라내기→붙여넣기(이동)** 가 모두 이 mutation 을 지나므로 한 곳에
  붙이면 전부 커버된다(F4 가 만진 `use-explorer-*` 는 무수정). `onSuccess` 를 async 로 둬 TanStack
  이 그 promise 를 기다리게 했다 — 탐색기의 `await renameEntryAsync` 가 트리 갱신으로 넘어가기 전에
  탭 이관이 끝난다. 실패는 삼킨다(파일은 이미 개명됐다 — 여기서 reject 하면 성공한 개명이 실패로
  보고되고, 붙여넣기 재시도 루프가 그 오류를 막다른 토스트로 띄운다).
- 신규 `entities/layout/tab-path-change.ts` 가 이관을 담당한다(`deps` 기본값 주입 seam —
  `workspace-edit-applier.ts` 와 같은 방식이라 monaco·백엔드 없이 테스트된다):
  ① **monaco 모델** — `retargetModel(from,to)`. uri 는 불변이라 in-place 이동이 불가능해
  새 모델 생성 + 옛 모델 폐기이며, **그 모델을 띄우던 에디터를 폐기 전에 새 모델로 갈아 끼우고**
  각자의 viewState 를 복원한다(monaco 는 `onWillDispose → setModel(null)` 로 자기 보호만 할 뿐이라
  그대로 두면 화면이 한 커밋 비고 커서·스크롤이 날아간다). 목적지에 남아 있던 낡은 모델(지워진
  파일의 탭·peek 프리로드)은 먼저 폐기한다 — `createModel` 은 중복 uri 에 예외를 던지므로 이걸
  빼면 개명이 하드 실패한다.
  ② **핫엑시트 미러** — `moved[].dirty` 면 **라이브 모델 내용**(미러는 500ms 디바운스만큼 뒤처진다)
  을 새 경로에 쓰고 옛 경로 미러를 지운 뒤 `FILE.MIRRORS` 캐시를 한 번에 재작성한다. 이게 미저장
  편집이 개명을 건너 살아남는 **유일한** 통로다: `EditorPane` 은 경로가 바뀌면 렌더 단계에서 자기
  dirty·syncedContent 를 초기화하므로, 미러가 없으면 곧이어 도는 디스크 sync 가 새 경로의 디스크
  내용으로 편집 버퍼를 덮는다. 미러가 먼저 있으면 그 페인의 미러 복원 이펙트가 draft 를 되살린다.
  ③ **`FILE.CONTENT` 캐시** — 옛 경로 항목으로 새 경로를 먼저 seed 해 페인이 "로딩" 분기(=
  `CodeEditor` 언마운트)로 떨어지지 않게 하고, 이어서 `file_open(to)` 결과로 교체하며 그
  `languageId` 를 `applyModelLanguage` 로 모델에 반영한다(확장자가 바뀌는 개명의 하이라이팅).
  옛 경로에 캐시가 없으면(=에디터가 읽은 적 없는 프리뷰 탭) 두 단계 모두 건너뛴다 — 개명한 30MB
  PDF 를 굳이 다시 읽지 않기 위해서다.
  ④ **"reopen with" 오버라이드** — 이관하지 않으면 개명 직후 그 탭이 조용히 프리뷰 렌더러로 바뀐다.
- 순서 계약: **모든 경로 이관이 끝난 뒤에** 호출부가 `applyFreshLayout` 으로 새 레이아웃을 캐시에
  쓴다. 모델 이관은 커맨드 응답 직후 동기로 일어나므로 어떤 refetch 보다도 앞선다.
- 추가 테스트: `tab-path-change.test.ts` 9케이스(모델·FILE 캐시·언어 적용 / 라이브 draft 미러 이관 /
  모델 없을 때 미러 내용 대체 / 폴더 개명 다중 경로 / 새 경로 재조회 실패 폴백 / 삭제 시 경로별 정리 /
  다른 페인에 남으면 dispose 안 함 / 대기 마커 반납 / 미조회 미러 목록에 부분 목록 미주입),
  `model-registry.test.ts` 7케이스(가짜 monaco 로 개명·에디터 재부착·중복 uri·언어 재적용·dispose).

**(3) 보고서 §1-6 — 닫힌 파일 탭의 monaco 모델 dispose**

- `useCloseTab` 의 per-path 정리를 `releaseClosedFileTabPath` 로 뽑아 **닫기와 삭제가 같은 함수**를
  쓰게 하고, 거기에 `disposeModel(path)` 를 추가했다. 조건은 **`collectAllPaneTabs` 로 본 모든
  창·페인에 그 파일 탭이 더는 없을 때**(스플릿·보조 창에 남아 있으면 모델은 공유물이라 유지).
- **undo 스택 보존 요구는 dispose 채택으로 갈음한다**(지시대로 계약에 명기). 실측 근거: 닫을 때
  `clearMirror` 로 복구 데이터를 이미 지우고, 닫은 탭을 재열기해도 페인이 clean 으로 마운트돼
  `applyExternalContent` 가 모델을 디스크 내용으로 덮으므로, **누수된 모델이 들고 있던 미저장 내용은
  원래도 되살아나지 않았다.** 즉 이번 dispose 로 실제로 잃는 것은 undo 히스토리뿐이고, 얻는 것은
  세션 중 연 모든 파일 본문이 앱 종료까지 메모리에 남던 누수의 제거다.
- 부수 수정 1건(같은 결함의 다른 반쪽): 닫힌 탭을 `findPaneTab(previous.root, …)` 로 **메인 트리에서만**
  찾고 있어, 보조 창에서 파일 탭을 닫으면 미러·오버라이드 정리(그리고 새로 붙인 dispose)가 통째로
  건너뛰어졌다. `collectAllPaneTabs(previous)` 로 바꿔 모든 창을 본다.

**검증**: `cargo fmt --all` 후 `cargo fmt --all -- --check` 통과, `cargo clippy --workspace
--all-targets -- -D warnings` 무경고, `cargo test --lib layout` **65 통과**(신규 7 포함),
`cargo test --lib remote` **136 통과**(바인딩↔dispatch 파리티·허용/거부 완전분할 테스트 포함).
`bun run typecheck`(내 변경 파일 오류 0 — 동시 진행 중인 다른 스테이지의 `document-symbol-session-
waiters` 오류만 존재) · `bun run lint` 0 error(경고 8, 기존과 동일) · `bun run format:check` 통과 ·
`bun test` **1693 pass / 0 fail**(이 스테이지 신규 16 포함).

## §4 검토·수정 기록

### 렌즈 검토 판정표 (2026-08-29)

> 렌즈 A(데이터 안전·상태 일관성) · B(Rust 커맨드·계약) · C(프론트 UX) 의 발견 중 **이 계약 소관**
> 분. major 는 실물 코드로 적대적 재검증(반증 시도) 후 확증분만 수정했고, minor 는 비용 대비 명백한
> 것만 손댔다. 프론트 전용 발견은 d-51 §4 에 있다.

| # | 렌즈 | 발견 | 판정 | 조치 |
|---|---|---|---|---|
| 1 | A major | lossy(비 UTF-8) 저장 차단이 `EditorPane` 한 곳뿐 — `workspace-edit-applier`·`ide-sync-provider` 우회 | **확증** | 두 진입점 모두 `readOnly` 가드 신설 (아래 (1)) |
| 2 | A major | 다중 창 개명이 다른 창의 미저장 draft 를 파괴, "재시작 복구는 된다" 기록이 거짓 | **확증** | 개명 **전** 미러 재조회로 초안 확보 + §5 S8 문구 정정 (아래 (2)) |
| 3 | B major | `layout_apply_path_change` 가 닫은 탭 스택 개명을 계산만 하고 버림 (§3 S8 미이행) | **확증** | `TabPathChangeOutcome.layout_changed` 신설 + 조기 반환 조건 축소 (아래 (3)) |
| 4 | A minor | 대소문자만 다른 개명에서 `clearMirror(from)` 이 방금 쓴 미러를 삭제 | **확증** | `migrateMirror` 순서 반전(clear → write) (아래 (4)) |
| 5 | B minor | `layout_apply_path_change` 가 루트 가드 없는 자유 경로를 받고 원격 허용 | **확증** | `ensure_change_within_root` 신설 (아래 (5)) |
| 6 | B minor | `run_git` 타임아웃이 파이프 드레인(join)에 미적용 — 성공 경로 무한 대기 | **확증** | 리더를 채널로 바꾸고 `GIT_PIPE_DRAIN_TIMEOUT_SECS` 적용 (아래 (6)) |
| 7 | B minor | `pty_write` spawn_blocking 이관으로 동일 틱 다중 쓰기 순서 붕괴 | **확증**(단, 이관 이전에도 완전 보장은 아니었음) | 프론트 세션별 직렬화 + `ipc-contract.md` 에 순서 규약 명기 (아래 (7)) |
| 8 | A minor | `file.query.ts` 의 "모든 rename 이 이 mutation 을 통과한다" 주석이 사실과 다름 | **확증** | JSDoc 정정(LSP 경로 예외 명시) — (2) 에 포함 |
| 9 | B minor | `state.rs` 교착 여유 근거가 신규 블로킹 풀 점유원을 반영하지 않음 | **확증**(문서 결함) | `begin_mutation_blocking` doc 갱신 (아래 (8)) |
| 10 | B minor | B11(개명 diff)은 배선만 끝났고 상태 행에 `beforePath` 소스가 없음 | **확증** | 감사 정본 §4-B 에 "부분 이행" 처분 정정 기재 |
| 11 | B info | `run_git` 폴링·타임아웃의 커맨드 단위 누적(commit=3회) 미기재, `push_fetch_lock` doc 이 사실과 다름 | **확증** | `push_fetch_lock` doc 정정 + §5 S3 에 커맨드 단위 상한 1줄 |
| 12 | B info | `ipc-contract.md` d-50 S1a 절이 `useSearchRun` 반환을 옛 값으로 기재 | **확증** | 해당 문단 현행화(F2 확장 참조 추가) |
| 13 | B minor | 워처 마지막-성분 예외로 무시 디렉토리 자신의 이벤트가 루트 `tree_refresh` 유발 | **부분 확증 · 코드 수정 기각** | §5 S6 문구 정정만 (아래 (9)) |
| 14 | B minor | 병렬 검색의 파일별 예산 스냅샷이 전이 메모리 상한을 워커 수만큼 키움 | **확증 · 이월** | §5 S1a 에 트레이드오프 기록 |
| 15 | B info | 삭제로 닫힌 탭이 닫은 탭 스택에 남아 재열기가 사라진 파일을 엶 | **확증 · 이월** | §5 S8 기록 + `docs/backlog.md` 등재 |
| 16 | A info | 워처 마지막-성분 예외 / `RenameFile.overwrite` 실패 / `layout_apply_path_change` 락·revision 규약 | **정보 — 회귀 아님** | (13) 과 같은 축의 문구 정정 외 무조치, §5 유지 |

### (1) §4-A-3 lossy 저장 차단을 나머지 두 진입점에 확장

파일: `src/shared/lib/lsp/workspace-edit-applier.ts` · `src/app/providers/ide-sync-provider.tsx`
(+ 테스트 `workspace-edit-applier.test.ts` 1건).

- S2 는 "lossy → `read_only` 강제 + 표식" 으로 §4-A-3 을 닫았고, §5 는 그 근거를 "에디터가
  read-only 파일에 draft 를 만들지 않는다" 로 적었다. 실제 차단은 `use-editor-file-persistence.ts`
  `handleSave` 의 한 줄뿐이었고, **디스크에 쓰는 다른 두 경로는 그 가드를 지나지 않는다.**
  - `applyTextEditsToUri` 의 모델 없는 분기: `openFile` → `applyTextEditsToContent` → `saveFile`.
    가드가 `tier === 'refused'` 하나라 `encodingLossy`(그리고 대용량 `readOnly`)가 통과했다. 프로젝트
    전역 rename-symbol 이 EUC-KR/Latin-1 소스를 건드리면 U+FFFD 본문이 원본을 영구 대체한다.
  - `ide:save-requested` 핸들러: `tab.dirty` 면 `model.getValue()` 를 그대로 저장한다. read-only
    파일도 미러 복원·백그라운드 모델 편집으로 dirty 가 될 수 있고 `readOnly` 검사가 없었다.
- 조치: 전자는 `file.readOnly` 를 `failureReason` 으로 반려(대용량 read-only 도 같이 닫힌다),
  후자는 저장 전에 `QUERY_KEY.FILE.CONTENT(path)` 캐시의 `readOnly` 를 보고
  `resolveIdeSave({ saved: false })` 로 반려한다.
- **열린 모델 분기는 막지 않았다**(경계): 이미 열린 파일의 LSP 편집은 모델에 적용되고 dirty 가 되지만,
  그 draft 가 디스크로 나가는 출구는 위 두 곳과 `handleSave` 뿐이고 셋 다 이제 막혀 있다. 여기서
  추가로 막으려면 편집 핫패스마다 `file_open` 왕복이 필요하다(§5 S2 에 기록).
- 테스트: `읽기 전용(비 UTF-8 lossy) 파일은 U+FFFD 로 치환된 본문을 디스크에 쓰지 않는다`.

### (2) 다중 창 개명 — 초안 확보 경로를 개명 **전** 으로 옮김

파일: `src/entities/file/file.query.ts`(`useRenameEntry.onMutate` 신설·JSDoc 정정).

- 반증 시도 결과 발견이 그대로 성립했다: 이관할 draft 는 `readModelContent(from)`(그 창의 모델
  레지스트리)와 `FILE.MIRRORS` 캐시(그 창의, 프로젝트 활성화 시점 스냅샷) 두 출처뿐인데 **둘 다
  창 로컬**이다. 보조 창에서만 편집 중이던 파일은 양쪽 모두 비어 draft 가 `null` 이 되고, 그 뒤
  `clearMirror(from)` 이 백엔드 미러까지 지운다.
- 계약이 적어 둔 "백엔드 미러로만 남는다(재시작 복구는 된다)" 도 **거짓**임을 코드로 확인했다:
  `list_mirrors` 는 `current_modified_ms(&mirror.path)` 가 `None` 인(원본이 사라진) 미러를 건너뛰고,
  `prune_mirrors`(keep = 열린 탭 = 새 경로)가 다음 활성화에서 지운다.
- 조치: `useRenameEntry` 에 `onMutate` 를 두어 **개명이 일어나기 전에** `FILE.MIRRORS` 를 강제
  재조회한다(`fetchQuery({ ...fileMirrorsQueryOptions(projectId), staleTime: 0 })`). 그 시점의
  `from` 은 아직 디스크에 있으므로 `list_mirrors` 가 다른 창이 쓴 미러를 정상적으로 돌려주고,
  기존 `cachedMirrorContent` 폴백이 그것을 새 경로로 옮긴다. 신규 IPC 표면 0.
  - 창을 옮기는 동작 자체가 보조 창의 `blur` → 미러 flush 를 일으키므로, 이 재조회가 보는 미러는
    대개 마지막 타이핑까지 반영돼 있다.
  - **남는 것**: 그 창의 *화면 버퍼* 는 여전히 새 경로의 디스크 내용으로 리셋된다(창 간 이관
    브로드캐스트가 필요 — 백로그). 즉 이 수정으로 회복되는 것은 계약이 원래 주장한 **재시작 복구**다.
    §5 S8 의 해당 문장을 그에 맞게 정정했다.
- 같은 함수의 JSDoc 에서 "any future caller all funnel through this one mutation" 을 삭제하고,
  `workspace-edit-applier.ts` 의 직접 `commands.fileRename` 호출이 예외로 남아 있다는 사실과 §5
  이월 참조를 명시했다(발견 8).

### (3) `layout_apply_path_change` — 닫은 탭 스택 치환의 소실

파일: `src-tauri/src/domain/layout/{service.rs,commands.rs}`.

- 재현 경로를 코드로 확인했다: `retarget_file_tabs` 는 `closed_tabs` 를 갱신하지만 `closed_paths` 는
  Renamed 축에서 항상 비고 `moved` 도 비면 `outcome.is_empty()` 가 참이 되어, 커맨드가
  `*state.layouts.write() = layouts` **앞에서** 조기 반환한다. 클론에만 적용된 치환이 통째로 버려져
  ⇧⌘T 가 사라진 경로를 연다(§3 S8 이 명시한 요구의 미이행). 덤으로 응답 layout 과 백엔드 상태가
  갈라졌다.
- 조치: `TabPathChangeOutcome` 에 **`layout_changed`** 를 신설해 "프론트가 할 일이 있는가"
  (`is_empty`)와 "레이아웃이 바뀌었는가"를 분리했다. 조기 반환은 유지하되(§3 S8 의 "대상 없으면
  리페치 낭비 방지" 근거 보존 — revision 미증가·이벤트 미발행), `layout_changed` 면
  `dirty_layouts` 표시와 스토어 write-back 은 수행한다. 응답도 write-back 되는 것과 같은 스냅샷이다.
- 테스트(Rust): `열린_탭이_없어도_닫은_탭_스택의_경로는_따라가고_레이아웃_변경으로_보고된다` ·
  기존 `대상_탭이_없으면_revision_을_올리지_않는다` 에 `layout_changed == false` 단언 추가.

### (4) 대소문자만 다른 개명의 미러 자멸

파일: `src/entities/layout/tab-path-change.ts`.

- `file_mirror_dirty`/`file_clear_mirror` 는 둘 다 `ensure_within_root` → `canonicalize_lenient` →
  `std::fs::canonicalize` 로 정규화한 경로를 해시해 미러 파일 키를 만든다. 대소문자 비구분 파일
  시스템에서 `readme.md` → `README.md` 개명 뒤 `canonicalize("/repo/readme.md")` 는 **성공하고**
  `/repo/README.md` 를 돌려주므로, 나중에 실행되는 `clearMirror(from)` 이 방금 쓴 `to` 의 미러를
  지운다(세션 중 복구는 `rewriteMirrorCache` 덕에 살지만 그 시점 종료하면 초안 소멸).
- 조치: `migrateMirror` 의 순서를 **`clearMirror(from)` → `mirrorDirty(to)`** 로 뒤집었다. 두 경로가
  같은 엔트리로 정규화되든 아니든 최종 상태가 옳다.
- 테스트: `옛 미러를 먼저 지운 뒤에 새 경로 미러를 쓴다 (대소문자만 다른 개명이 방금 쓴 미러를 지우지
  않도록)` — 호출 순서를 `mirrorOps` 로 고정.

### (5) `layout_apply_path_change` 루트 가드

파일: `src-tauri/src/domain/layout/commands.rs`(+ `mod tests` 신설 2케이스).

- 신설 커맨드가 `from`/`to`/`path` 를 검증 없이 `strip_prefix` 매칭에 쓰고 있었고,
  `IMPLEMENTED_JSON_COMMANDS`·`REMOTE_ALLOWED_COMMANDS` 양쪽에 등재돼 있다. `deleted{path:"/"}` 는
  모든 절대 경로의 조상이라 프로젝트의 파일 탭 전부를 닫고, 프론트가 `closedPaths` 마다
  `releaseClosedFileTabPath` → `clearMirror` 를 돌려 **미저장 초안 전부**를 지운다. §3 S8 의 허용
  근거("`file_rename`+`layout_close_tab` 조합보다 넓지 않다")는 경로 스코프 제한을 전제하는데 그
  제한이 코드에 없었다.
- 조치: `ensure_change_within_root(root, change)` 를 커맨드 진입부(가드 획득 **전**)에 둔다. 판정은
  **성분 단위 `starts_with`** 이며 canonicalize 하지 않는다 — 개명이 끝난 시점에 `from` 은 디스크에
  없다는 §3 S8 의 근거가 그대로 유효하고, 프로젝트 루트는 열 때 canonical 로 기록되며 탭 경로는 그
  루트를 걸어 만들어지므로 두 문자열의 접두사가 같다. 위반은 `Forbidden` +
  `error.path.outsideProjectRoot`(기존 키, 로케일 순증 없음).
- 테스트: `프로젝트_루트_안의_경로_변경은_통과한다` · `루트_밖_경로와_슬래시는_거절된다`(`/` 거절,
  루트 밖 from/to, 이름이 접두사인 형제 디렉토리).

### (6) `run_git` — 성공 경로의 파이프 드레인에도 상한

파일: `src-tauri/src/domain/git/service.rs`.

- kill 경로는 리더 스레드를 의도적으로 join 하지 않는데(doc 에 근거 기재), **성공 경로는 무기한
  join** 했다. `git` 이 남긴 손자(ssh ControlMaster/ControlPersist, credential helper)가 stdout 쓰기
  끝을 쥐고 있으면 `read_to_end` 가 EOF 를 못 받아 join 이 영원히 멈추고, 그동안 `git_pull` 은
  `begin_mutation` 을, `git_push`/`git_fetch` 는 `push_fetch_lock` 을 쥔다 — §2 M-7 이 지목한 증상이
  그대로 남아 있었다(기존 `Command::output()` 과 동일하므로 회귀는 아니나, "상한의 목적은 무한을
  끊는 것"이라는 §3 S3-(4) 의 주장이 이 경로에서 성립하지 않았다).
- 조치: `read_pipe_on_thread` 가 `JoinHandle` 대신 `mpsc::Receiver` 를 돌려주고, 수집 측이
  `recv_timeout` 으로 **자식 종료 후 남은 출력만** 기다린다. 상한은 신규 상수
  `GIT_PIPE_DRAIN_TIMEOUT_SECS = 5` — 리더 스레드가 명령 실행 내내 병행 드레인하므로 종료 시점에
  남은 것은 파이프 버퍼 잔량뿐이다. 초과 시 꼬리 출력만 잃는다(kill 경로가 이미 하는 트레이드와 동일).
- 커맨드 표면·타입 변화 0. `cargo test --lib git` 109 통과(기존 `파이프_버퍼보다_큰_출력도_교착_없이_
  수집된다` · `상한을_넘긴_서브프로세스는_강제_종료되고_none을_돌려준다` 포함).

### (7) `pty_write` 순서 규약

파일: `src/entities/terminal/{session-write-order.ts(신규),session-write-order.test.ts(신규),
terminal.ipc.ts}` · `docs/ipc-contract.md`.

- 적대적 재검증: "이관 전에는 워커가 태스크를 집는 순서가 곧 쓰기 순서였다"는 **엄밀히는 참이 아니다**
  (멀티스레드 런타임에서 spawn 순서 = 첫 poll 순서가 보장되지 않는다). 그러나 이관으로 각 호출이
  블로킹 풀 디스패치라는 **독립 스케줄링 지점을 하나 더** 갖게 되어 재배열 창이 실질적으로 넓어진 것은
  맞고, 재현 형태도 코드에 있다: `terminal-write-bridge.ts` 의 `registerTerminalWriteHandler` 가
  대기 큐를 한 틱에 `for` 로 연속 발사하고 `terminal-session.tsx` 의 `handleWrite` 는 각각을 await
  없이 `void writePty(...)` 로 쏜다(태스크 러너·"Run Selected Text"·스폰 전 입력).
- 조치: 백엔드 대신 **프론트에서 세션 단위 직렬화**(가장 작은 변경)를 택했다.
  `enqueueSessionWrite(sessionId, run)` 가 세션별 프라미스 체인을 잇고 `writePty` 가 그것을 통과한다.
  실패한 쓰기가 큐를 막지 않도록 다음 쓰기는 성공/실패 양쪽에 이어 붙이고, 꼬리 항목은 정착 후 자신을
  맵에서 지운다. `docs/ipc-contract.md` "d-50 S4" 절에 **"`pty_write` 자체는 순서를 보장하지 않는다 /
  같은 세션 순서는 프론트가 보장한다"** 를 계약으로 명기했다(원격 dispatch 등 다른 클라이언트가 직접
  호출하면 스스로 직렬화해야 한다는 단서 포함).
- 테스트 3케이스: 같은 세션 순차 실행 / 다른 세션 비차단 / 실패한 쓰기가 뒤를 막지 않음.

### (8) `state.rs` 교착 여유 근거 갱신

`begin_mutation_blocking` 의 doc 에, 여유 마진이 **스레드 수** 라서 이 guard 를 건드리지 않는 블로킹
작업에도 소모된다는 점과 이번 배치가 늘린 점유원(파일 4종·git 조회 7종·트리 프리페치, 그리고 상한이
없는 `pty_write`)을 반영했다. `file_save`/`file_copy` 가 **guard 를 쥔 채** 빈 블로킹 스레드를 기다리는
쪽이라 부족이 자기지속적으로 번질 수 있다는 사슬도 명시했다. 512 근처는 실사용에서 아니고(프론트
직렬화로 세션당 대기 쓰기도 1개), 코드 변경은 없다.

### (9) 워처 마지막-성분 예외의 소비 측 증폭 — 코드 수정 기각

- 사실 관계는 확증했다: `<root>/.git`·`node_modules`·`dist` **자신**의 create/remove 는 이제 통과하고,
  `ipc-sync-provider` 가 `parentDirOf(path)` = 프로젝트 루트로 `tree_refresh(root)` 를 돌리므로 이벤트
  1건이 루트 전체 재평탄화 + `prune_stale_descendants` 1회로 증폭된다.
- **기각 사유**: 제안된 소비 측 필터(마지막 성분이 무시 이름인 create/remove 를 트리 축에서 제외)는
  경로만으로 파일/디렉토리를 구분할 수 없다는 §4-A-4 의 근본 사실을 그대로 되밟는다. 루트에 있는
  `dist` 라는 **파일**의 생성/삭제가 트리에 반영되지 않게 되어, 이번 배치가 고친 바로 그 결함을
  부분적으로 되돌린다. 빈도(디렉토리 자신 이벤트)가 낮은 대신 되돌아오는 결함은 상시적이다.
- 대신 §5 S6 의 "이벤트 양의 증가는 사실상 없다" 를 **소비 비용까지 포함해** 정정했다.

## §5 이월·미해결

### S1a search

- **10,000 매치 상한에 걸릴 때 "어떤" 매치가 남는지는 비결정**이 됐다(상한 자체는 정확히 유지).
  순차 워크는 워크 순서상 앞선 파일이 항상 살아남았지만, 병렬 워크에서는 예산을 먼저 claim 한
  파일이 살아남는다. 상한에 걸리는 것 자체가 이미 "결과가 잘렸다"는 상태라 표시 정책(C10 FE 의
  잘림 표시 — d-51 F2)으로 다루는 것이 옳다고 판단해 결정성 복원(정렬·직렬 fallback)은 하지 않았다.
- **`collect_project_files`/`list_project_files` 는 순차 유지.** H-1 의 대상은 검색 스캔이고,
  치환 대상 해석은 순차 순서가 곧 치환 적용 순서라 병렬화하면 재현성이 떨어진다. 퀵오픈 인덱스도
  d-42 의 "상한 없는 전체 목록" 계약이 순서와 무관하므로 이득이 작다 → 착수하지 않음.
- **치환 경로에는 취소 검사를 넣지 않았다** — `search_replace` 에는 취소 토큰이 애초에 없다
  (`SearchStore` 는 `search_run` 전용). 취소 가능 치환은 범위 밖(신규 커맨드 표면 필요).
- **C10 의 FE 절반**(스킵 목록·10k 잘림 표시)과 **perf-4**(결과 가상화)는 계약대로 d-51 F2.
- **[검토 추가] 병렬화로 전이(transient) 메모리 상한이 워커 수만큼 커졌다.** 각 워커는 스캔 직전
  `total.load()` 스냅샷으로 `remaining = SEARCH_MATCH_LIMIT - claimed` 를 계산해 넘기고, 예산 확보
  (`claim_match_budget`)는 스캔이 **끝난 뒤** 일어난다. 그래서 N개 워커가 동시에 출발하면 각자
  `claimed ≈ 0` 을 보고 한 파일에서 최대 10,000건까지 모은 뒤에야 잘린다 — 전이 최대는
  `N × (10,000 × SearchLineMatch)` + `N × 최대 50MB 파일 바이트`이고, 순차 구현에서는 전체 10,000건이
  상한이었다(`SearchLineMatch` 는 preview + before/after 컨텍스트 문자열을 든다). **최종 결과의 상한은
  정확히 유지**되므로 정확성 문제는 아니다. 상수화하려면 스캔 전 낙관적 예약(청크 claim → 반납) 또는
  워커당 상한(`SEARCH_MATCH_LIMIT / threads` + 하한 보정)이 필요한데, 둘 다 예산 회계 방식의 변경이라
  계약 범위 밖으로 남긴다.

### S2 file

- **LSP `WorkspaceEdit` 의 `RenameFile.options.overwrite = true` 가 이제 실패한다.** 목적지 존재
  가드에 예외를 두려면 `file_rename` 에 overwrite 인자를 신설해야 하는데, 이는 계약이 지정한
  "목적지 존재 가드" 범위를 넘고 데이터 파괴 방향으로 문을 다시 여는 일이라 착수하지 않았다.
  현재 동작: `workspace-edit-applier.ts` 의 `applyRenameFile` 이 `failureReason` 으로 보고한다
  (조용한 덮어쓰기 → 눈에 보이는 실패). 서버가 실제로 overwrite 를 요구하는 사례가 나오면 그때
  인자 신설을 별건으로 판단한다.
- **[검토 정정] 프론트 차단 범위**: 이 항목이 원래 근거로 삼은 "에디터가 read-only 파일에 draft 를
  만들지 않는다" 는 `EditorPane` 축에서만 참이었다. §4 (1) 에서 `workspace-edit-applier`(LSP
  `WorkspaceEdit`)와 `ide-sync-provider`(Claude Code 저장 요청)를 함께 닫았다. **남는 경계**: 이미
  열린 모델에 대한 LSP 텍스트 편집은 여전히 적용되어 버퍼가 dirty 가 된다(디스크로 나가는 출구 셋은
  모두 막혀 있으므로 파일은 안전하다). 그 지점까지 막으려면 편집 핫패스마다 `file_open` 왕복이
  필요하다 — 아래 백엔드 하드가드와 같은 판단으로 착수하지 않았다.
- **저장 차단은 "읽기 전용 강제 + 표식"으로만 구현했고, `file_save` 경로에서 디스크 파일을 다시
  읽어 UTF-8 여부를 재검사하지는 않는다.** 매 저장마다 원본 전량을 읽는 비용(H-3 로 블로킹 풀에
  옮겼어도 저장마다 O(파일 크기) 추가 IO)이 감사가 요구한 "최소 수정" 범위를 넘고, 외부 도구가
  방금 비 UTF-8 로 덮어쓴 파일에 정상 버퍼를 저장하려는 경우까지 막는 오탐이 생긴다. 백엔드
  하드가드가 필요하다고 판단되면 인코딩 왕복 지원(백로그)과 함께 다루는 것이 옳다.
- **`file_create`/`file_rename`/`file_delete` 는 `spawn_blocking` 으로 옮기지 않았다** — 계약이
  지정한 H-3 대상 4종(open·save·copy·mirror_dirty)만 이관했고, 나머지는 단일 syscall 급이다
  (`delete_entry` 의 휴지통 이동은 예외적으로 무거울 수 있으나 계약 범위 밖 → 필요 시 별건).
- `file_open` 의 `plugin_service::ensure_loaded`(플러그인 최초 로드 시 디스크 스캔)는 async 쪽에
  남겼다 — 캐시 히트가 정상 경로이고, 옮기려면 `PluginStore` 도 `AppHandle` 경유로 다시 빌려야 해
  변경면이 커진다.

### S3 git

- **[F3 에 전달해야 할 발견] 개명 상태 행은 `path` 가 옛 경로이고 `origPath` 는 구조적으로 항상
  `null` 이다.** B11 의 원경로 축을 검증하다 실측으로 확인했다: git2-rs 의
  `StatusEntry::path()` 는 `head_to_index`(없으면 `index_to_workdir`)의 **`old_file.path`** 를
  돌려준다. 그래서 `collect_status_rows` 의 `StatusRow.path` 는 개명 행에서도 옛 경로이고,
  `orig_path` 는 같은 `old_file.path` 를 `path` 와 비교해 `filter(|orig| orig != path)` 로
  걸러내므로 **어떤 행에서도 값이 채워질 수 없다**. 또 `staged_change_kind` 가 `INDEX_MODIFIED` 를
  `INDEX_RENAMED` 보다 먼저 보므로, 내용까지 바뀐 개명은 `Renamed` 가 아니라 `Modified` 로 온다
  (실측: `flags = INDEX_MODIFIED | INDEX_RENAMED`, `path = "old.txt"`,
  `head_to_index.new_file = "new.txt"`).
  → **결과**: F3 가 git 패널의 스테이지된 행에서 `beforePath` 를 뽑으려 해도 지금은 소스가 없다.
  커밋 diff 축(`CommitFile.origPath` — `commit_files` 가 델타에서 직접 만든다)은 정상이므로 그쪽은
  영향이 없다. 상태 행의 rename 방향 정정은 감사 §2 M-2("양방향 rename")로 이 배치 §0 에서 명시적
  제외·백로그 이관된 항목이라 **여기서 고치지 않았다.** F3 는 (a) 커밋 diff 축만 배선하거나
  (b) M-2 를 선행 별건으로 올려야 한다.
- **`QUERY_KEY.GIT.DIFF` 에 `beforePath` 를 넣지 않았다** — 계약이 지정한 "bindings 표면까지" 를
  넘고, 키 형태 변경은 `ipc-sync-provider.tsx` 의 `fs:changed` 스코프 매칭(키 인덱스 3 = path)과
  같은 소비처를 함께 봐야 하기 때문이다. F3 가 `beforePath` 를 실제로 넘기기 시작하는 순간 **키에도
  반드시 포함**해야 한다(같은 새 경로에 원경로 유무로 다른 결과가 나오므로 캐시가 섞인다). 키 끝에
  덧붙이면 인덱스 3 은 그대로라 위 매칭은 영향받지 않는다.
- **`run_git` 타임아웃은 전역 guard 분리와 무관하다** — 계약 §0 대로 `git_pull` 의 전체
  `begin_mutation` 보유와 `push_fetch_lock` 은 손대지 않았다. **[검토 반영]** 사실과 달라진
  `GitStore::push_fetch_lock` 의 doc 문단("`run_git` 에는 타임아웃도 kill 경로도 없다" + "무한 대기를
  감수한 트레이드오프")은 별건으로 미루지 않고 이번에 정정했다 — 남겨 두면 다음 배치가 "이 락은 무한
  대기한다"는 전제로 판단하게 되기 때문이다. 락 설계 자체(획득 범위·순서·pull 제외)는 그대로다.
- **[검토 추가] 상한은 서브프로세스 1개당이다.** `GIT_COMMAND_TIMEOUT_SECS`(300초)는 `run_git` 호출
  하나의 상한이므로, 여러 번 부르는 커맨드는 그만큼 곱해진다 — `git_commit` 은 `add -A` → `commit`
  → `rev-parse HEAD` 로 3회라 **실효 최대 ~15분** 동안 `begin_mutation` 을 쥘 수 있다. 마찬가지로
  `GIT_COMMAND_POLL_INTERVAL_MS`(20ms) 때문에 짧은 git 호출도 최소 한 번의 sleep 을 지불한다
  (`commit` 은 최대 60ms). 둘 다 실사용상 무해하지만, "타임아웃 300초" 를 커맨드 단위 상한으로 읽지
  않도록 기록해 둔다.
- **[검토 추가] 자식 종료 후 파이프 드레인에는 별도 상한**(`GIT_PIPE_DRAIN_TIMEOUT_SECS` = 5초)이
  붙었다 — §4 (6). 초과하면 꼬리 출력을 잃는다.
- **타임아웃 값 300초는 실측 근거가 아니라 판단이다.** 큰 저장소·느린 원격에서 정당하게 300초를
  넘는 `push`/`pull` 이 있으면 사용자에게는 "중단됨"으로 보인다. 값 조정이 필요하면 상수 하나
  (`GIT_COMMAND_TIMEOUT_SECS`)만 바꾸면 되고, 네트워크 커맨드와 로컬 커맨드를 다른 상한으로 가르는
  것은 계약이 "pull·push·fetch 공통"이라 착수하지 않았다.
- **`git_log`·`git_file_log`·`git_blame_range`·`git_commit_files`·`git_tags`·`git_conflict_sides` 는
  이미 `spawn_blocking` 이었다** — M-1 이 지목한 7종만 이관 대상이었고, 나머지 조회는 손대지 않았다.
- **`stage` 의 삭제 분기(`remove_path`)에는 디렉토리 케이스를 만들지 않았다** — 삭제된 디렉토리는
  상태에 `dir/` 단일 행으로 접히지 않고(접기는 미추적 전용) 파일별 행으로 오므로 재현 경로가 없다.

### S4 terminal

- **리플레이가 최대 2청크로 나뉘는 것은 새 관측 가능 동작이다.** 프론트(`terminal.ipc.ts` →
  `term.write`)는 청크 단위로 그대로 흘려보내므로 코드 수정이 없고 xterm 파서가 write 경계를
  보존하지만(§12.3 반증 항목), 계약이므로 `docs/ipc-contract.md` "d-50 S4" 와
  `docs/features/terminal.md` §3 에 명기했다. 1청크를 고수하려면 attach 마다 2MB rotate 가 필요해
  M-5 의 이득을 attach 경로에서 되돌리게 되므로 채택하지 않았다.
- **`pty_resize`/`pty_set_paused`/`pty_kill` 은 `spawn_blocking` 으로 옮기지 않았다** — 계약이
  지목한 M-6 대상은 `pty_write` 뿐이고, 나머지는 ioctl·시그널 급 단발 호출이다.
- **`OUTPUT_BATCH_MS`(4ms)·`OUTPUT_FLUSH_TICK_MS`(5ms) 값은 손대지 않았다.** L-2 는 "유휴 wakeup
  제거"이지 지연 튜닝이 아니고, 값 변경은 §12.2-A 가 근거로 삼은 VS Code `TerminalDataBufferer`
  대비 규약을 다시 판단하는 일이라 범위 밖이다.
- **`terminal.md` §12.2-C 의 "폭 차이·바이트 단위 절단" 한계는 그대로 남는다** — 이번에 해소한
  것은 같은 문단이 함께 지적한 attach 레이스뿐이다. 절단 문제의 근본 해법은 같은 문서 §12.4 의
  3번(탭 전환 시 unmount 대신 숨김 유지로 replay 경로 자체 제거)이고 그것은 프론트 작업이다.
- **[관측·이 스테이지와 무관] `infra::pty::tests::셸_통합이_비활성이면_기존_default_prog_빌더_
  그대로다` 는 기존부터 플레이키하다.** 이 테스트가 `SHELL_INTEGRATION_ENV_VAR` 를 set 하는 사이
  같은 파일의 다른 3개 테스트(zsh·bash 주입, drop 임시 디렉터리)가 그 변수를 remove 해 주입이
  일어나면 실패한다(프로세스 전역 env 를 병렬 테스트가 공유). 이번 변경분을 `git stash` 로 걷어낸
  깨끗한 트리에서 8회 중 1회 재현했다 — 원인은 env 공유이지 이번 스레드 변경이 아니다. 수정하려면
  env 뮤텍스 도입 또는 `build_command` 에 주입 여부를 인자로 넘기는 리팩터가 필요해 계약 범위 밖
  으로 남긴다.

### S5 lsp

- **`domain/ide/server.rs` 의 같은 형태 `format!("file://{path}")` 는 손대지 않았다** (`:129` MCP
  `fileUrl`, `:374`·`:394` 워크스페이스 폴더/열린 파일, `:416` 진단 URI). 감사 §4-A-7 이 지목한 것은
  `lsp/commands.rs` 한 곳이고, ide 도메인은 상대가 monaco 가 아니라 Claude Code MCP 클라이언트라
  표기 상대·기대 규약을 따로 확인해야 한다(같은 파일 `:224` 는 `strip_prefix("file://")` 로 **디코딩
  없이** 되돌리므로, 인코딩만 켜면 그 축이 깨진다). 별건 판단 대상 — 공백·한글 경로에서 동일한
  불일치가 있을 가능성이 높다는 사실만 기록한다.
- **`workspace_folder_json` 의 `name` 필드 표기 차이는 남는다** — Rust 는 `file_name()` 이 없으면
  `"workspace"`, 프론트(`toWorkspaceFolderName`)는 루트 문자열 자신을 쓴다(루트가 `/` 인 경우에만
  갈린다). 서버가 폴더를 식별하는 축은 `uri` 라 동작에 영향이 없어 이번 범위에서 제외했다.
- **UNC(`//server/share`)·Windows verbatim(`\\?\C:\...`) 경로는 monaco 와 표기가 갈릴 수 있다** —
  monaco 의 `URI.file` 은 선행 `//` 를 authority 로 떼어내 `file://server/share` 를 만드는데,
  `workspace_folder_uri` 는 그 분기를 재현하지 않는다(경로 전체를 path 로 둔다). CI·배포 대상이
  macOS 뿐이고 재현 환경이 없어 착수하지 않았다. Windows 지원을 실제로 켜는 시점에 함께 판단한다.
- **`domain::lsp::commands::wait_for_process_exit` 의 `is_exited` 50ms 폴링은 그대로 두었다** —
  L-3 이 지목한 것은 `lsp_proc` 의 상시 `try_wait` 폴링(유휴 서버당 초당 20회)이고, 이쪽은
  `lsp_stop`/`lsp_restart` 종료 시퀀스에서만 도는 **상한 2초의 유한 루프**다. 알림 기반으로 바꾸려면
  `LspProcHandle` 에 종료 Notify 를 노출해 표면을 넓혀야 하는데 정상 상태 이득이 없다.
- **헤더 구분자(`\r\n\r\n`)가 영원히 오지 않는 순수 잡음 스트림은 여전히 버퍼를 키운다.** 이번
  수정은 "헤더 블록은 왔는데 `Content-Length` 가 없는" 경우를 드레인하며, 구분자 없는 무한 스트림에
  상한을 두려면 "헤더 최대 길이" 정책을 새로 정해야 해서(정상 프레임을 자르지 않을 값 선택 + 잘랐을
  때의 사용자 통지) 계약 범위 밖으로 남긴다. 실제 서버에서 관측된 형태는 아니다.

### S6 infra 소형

- **무시 디렉토리 *자신*의 이벤트 1건은 이제 통과한다**(예: `node_modules` 폴더 생성/삭제). 마지막
  성분이 파일인지 디렉토리인지는 경로만으로 판정할 수 없고(삭제 이벤트면 stat 도 불가), stat 을
  거는 것은 디바운스 콜백에 syscall 을 들이는 일이라 채택하지 않았다. 하위 경로는 종전대로 전부
  걸러진다.
  - **[검토 정정] "이벤트 양의 증가는 사실상 없다" 는 소비 비용을 빠뜨린 서술이었다.** 그 1건은
    `ipc-sync-provider.tsx` 에서 `parentDirOf(path)` = **프로젝트 루트**가 되어 `tree_refresh(root)`
    를 부르고, S7 의 `invalidate` 가 루트 갱신에서 `prune_stale_descendants` 로 트리 전체를 훑는다.
    즉 이벤트 1건 = 루트 전체 재평탄화 + 정리 1회다(`rm -rf dist && mkdir dist`, `npm install` 등).
    빈도가 낮아(FSEvents file-events 모드에서 디렉토리 자신 이벤트는 드물다) 수용하지만, "싸다" 는
    근거는 이벤트 수가 아니라 빈도다.
  - 제안된 소비 측 필터(마지막 성분이 무시 이름인 create/remove 를 트리 축에서 제외)는 **기각**했다 —
    §4 (9). 루트에 있는 `dist` 라는 *파일*의 생성/삭제가 다시 트리에 반영되지 않게 되어, 이 스테이지가
    고친 §4-A-4 를 부분적으로 되돌린다.
- **git 워처가 `objects/**` 를 워치 자체에서 빼지는 않는다** — `WatchScope::GitDir` 이 이름 필터를
  끈 만큼 `classify_git_change` 에 도달하는 경로 수는 늘 수 있으나(패킹·fetch 시 `objects/**`),
  그 판정은 문자열 비교 몇 번이고 이벤트 발행으로는 이어지지 않는다. notify 등록 단계에서 하위
  디렉토리를 제외하려면 `RecursiveMode` 를 버리고 수동 등록으로 가야 해 범위 밖이다.
- **접미 범위에 청크 상한을 걸면 "마지막 N 바이트"의 앞부분만 돌려준다**(예: 2MB 꼬리 요청 →
  앞 1000KB). HTTP 상 정상이고 `Content-Range` 가 실제 구간을 말하지만, 꼬리 쪽을 우선 주는 정책이
  더 낫다는 판단이 서면 상한 적용 방향만 바꾸면 된다 — 지금은 일반 범위와 **같은 규칙**(시작점 기준
  절단)을 유지하는 쪽을 택했다.
- **`ps` 배치는 인자 하나가 거부되면 배치 전체가 빈 결과가 된다**(macOS 는 `process id too large`
  로 즉시 실패). pid 는 OS 가 준 실제 자식 pid 라 실현 경로가 없다고 보고 방어를 넣지 않았다 —
  넣는다면 per-pid fallback 이 되어 M-3 이 없애려던 fork 를 되살린다.
- **windows 분기는 손대지 않았다** — `sysinfo::System::new_all()` 이 이미 1회 전체 스냅샷이라
  M-3 이 지목한 fork 증폭이 없다. 세션 0 조기 반환만 공통 경로에서 함께 적용된다.
- **`poll_agents` 의 프로젝트 루프는 여전히 프로젝트당 1회 `ps`** 다. 프로젝트 전체를 한 번에 묶으면
  `agents.diff`/`AgentStateChanged` 가 프로젝트 단위라는 계약 때문에 결과를 되쪼개야 하고, 열린
  프로젝트 수는 한 자릿수라 이득이 작다.

### S7 tree

- **응답 형태(전체 트리 재직렬화)는 손대지 않았다** — 계약 §0·§1 대로 `tree_toggle`/`tree_reveal`/
  `tree_refresh` 는 여전히 `full_page` 를 돌려준다. 행 수가 많은 프로젝트에서 토글 한 번이 전체
  `TreeRow` 를 다시 만드는 비용은 그대로이며, 이 재설계는 백로그(§7)다.
- **선행 읽기는 "guard 를 잡은 시점의 디스크" 보장을 내려놓는다.** 이전에는 `read_dir` 이 전역
  mutation guard 아래에서 돌아 다른 뮤테이션이 진행 중이 아닌 상태를 봤다. 지금은 guard 전에 읽으므로
  ① 진행 중인 복사·삭제의 중간 상태를 볼 수 있고 ② 같은 디렉토리에 대한 두 갱신이 겹치면 나중에
  도착한 호출이 더 오래된 목록을 쓸 수 있다. 둘 다 뒤따르는 `fs:changed` → `tree_refresh` 로 자기
  교정되며(같은 파일의 `rows_page_from_store` 미스 경로가 이미 무가드 스캔이라는 선례),
  `ipc-sync-provider.tsx` 가 이미 "한 호출의 스냅샷은 그 호출 이전 무효화만 반영한다"고 적어 둔
  성질과 같은 축이다. 결정성을 되찾으려면 디렉토리 단위 버전(또는 mtime) 비교로 "더 오래된 목록은
  꽂지 않기"가 필요한데, 이는 캐시 설계 변경이라 계약 범위 밖이다.
- **삭제와 동명 재생성이 같은 디바운스 창 안에서 끝나고, 새 폴더가 비어 있지 않으면 옛 자식이 한
  틱 남을 수 있다.** 정리 판정은 "새 목록에 없다" 또는 "있지만 자식이 없다" 두 가지이고, 지웠다 다시
  만든 폴더가 이미 자식을 가진 채 관측되면 두 조건 모두 벗어난다. 동일성(inode·생성시각) 비교를
  들이면 확정적으로 막을 수 있으나 `Entry` 에 신원 필드를 얹고 플랫폼별 취득 경로를 붙이는 일이라
  범위 밖이다. 실제로는 `remove_dir_all` 이 자식 삭제 이벤트도 내보내 그 폴더 자신에 대한
  `tree_refresh` 가 함께 오므로 다음 갱신에서 사라진다.
- **`expand`/`ensure_root_loaded` 경로에는 정리를 걸지 않았다** — 계약이 지목한 것은 "폴더 삭제·개명
  시" 이고 그 신호는 부모 디렉토리의 갱신(`invalidate`)이다. 펼치기는 아직 캐시가 없는 디렉토리를
  처음 읽는 경로라 버릴 하위 상태가 없다.
- **갱신 대상이 접힌 디렉토리면(= 재조회 조건 미충족) 정리도 하지 않는다.** 목록을 읽지 않으므로
  무엇이 사라졌는지 알 수 없고, 읽기를 강제하면 "접힌 디렉토리는 캐시만 버린다"는 기존 계약이
  바뀐다. 그 디렉토리가 다시 보이는 시점에는 조상 갱신이 같은 정리를 수행한다.
- **`file_delete`/`file_rename` 커맨드에서 트리 상태를 직접 건드리지는 않았다** — 트리 캐시의 소유는
  `TreeStore` 이고 파일 도메인이 그것을 알면 도메인 간 직접 결합이 생긴다. 정리는 지금처럼 워처
  이벤트 → `tree_refresh` 라는 기존 단일 경로에 얹었다.

### S8 layout

- **추종 대상은 `TabKind::File` 로 한정했다.** `Diff`/`ClaudeDiff` 는 비교 뷰라 제목이 문맥을 담고
  (`a.ts (diff)` · `a.ts vs b.ts` · `a.ts @ 1a2b3c4`) 짝(`rev`/`parentRev`/`beforePath`/`compareWith`)
  이 개명으로 이전되지 않는다 — 경로만 바꾸면 "옛 이름표가 새 파일을 가리키는" 더 나쁜 상태가 된다.
  또 커밋 diff 의 `path` 는 그 리비전 시점 경로라 작업트리 개명과 무관하다. **잔여**: 개명·삭제된
  파일의 작업트리 diff 탭은 다시 열기 전까지 오류 상태로 남는다.
- **미저장 편집이 있는 탭을 개명하면 "복구됨" 배너가 뜬다.** 이관 경로가 핫엑시트 미러 복원이기
  때문이다(내용·dirty 는 정확히 보존된다). 개명 전용 문구를 두려면 로케일 키와 `restoreNotice`
  변종이 하나 더 필요해 이번 범위에서 제외했다.
- **모델 이관·닫기 dispose 둘 다 undo 스택을 잃는다.** monaco 에 uri 이동 API 가 없고, 보존하려면
  `ITextModel` 밖에 편집 히스토리를 따로 들고 다니는 자체 스택이 필요하다(§3 (3)의 근거대로
  "닫기 후 재열기 시 미저장 내용 복원"은 원래도 되지 않았다).
- **개명 직후 페인의 디바운스 미러가 옛 경로로 한 번 더 써질 수 있다.** 페인의 flush 는 자기 렌더가
  잡은 옛 `path` 를 쓰기 때문이다. 그 미러는 어떤 탭도 가리키지 않는 무해한 잔여이고, 다음 프로젝트
  활성화의 `file_prune_mirrors`(keepPaths = 열린 탭) 스윕에서 제거된다. 확정적으로 막으려면 개명
  전에 그 탭의 flush 를 강제 호출해야 하는데, 그러려면 커맨드가 tabId 까지 돌려주고 프론트가
  `mirror-flush-registry` 를 태워야 해 범위를 넘는다.
- **다른 OS 창(보조 창)의 모델·미러 캐시는 그 창이 스스로 처리하지 못한다.** 창마다 JS 모듈
  인스턴스(모델 레지스트리·쿼리 캐시)가 따로라, 개명을 실행한 창만 이관을 수행한다. 다른 창은
  `layout:changed` 로 탭 경로만 따라간 뒤 새 경로의 모델을 새로 만든다 — **그 창의 화면 버퍼는 새
  경로의 디스크 내용으로 리셋된다.** 창 간 이관 브로드캐스트는 별도 채널 설계가 필요해 백로그다
  (`docs/backlog.md`).
  - **[검토 정정]** 이 문단은 원래 "draft 는 백엔드 미러로만 남는다(재시작 복구는 된다)" 라고 적었으나
    그것은 **거짓이었다**: 옛 경로의 미러는 `list_mirrors` 가 `current_modified_ms` 가 `None` 인(원본이
    사라진) 항목을 건너뛰어 복구 목록에 뜨지 않고, 다음 활성화의 `prune_mirrors`(keep = 열린 탭 =
    새 경로)가 삭제한다. §4 (2) 에서 `useRenameEntry.onMutate` 가 **개명 전에** `FILE.MIRRORS` 를 강제
    재조회하도록 고쳐 다른 창의 초안을 새 경로 미러로 이관하게 했고, 그 결과 **이제 이 문장이 참이 됐다**
    (재시작 복구 가능 · 화면 버퍼는 여전히 리셋).
  - 여전히 남는 창 경계 결함 2건: ① `move.dirty` 는 백엔드 탭 플래그 하나라 `layout_set_tab_dirty`
    IPC 가 착지하기 전(첫 타이핑 직후)에 개명이 들어오면 이관 대상에서 빠진다. ② 개명 후 보조 창의
    pane 은 렌더 단계 리셋으로 **로컬 dirty 만** false 가 되고 `layout_set_tab_dirty` 는 부르지 않아,
    탭에 dirty 점이 남은 채 내용은 clean 인 불일치가 남는다. 둘 다 창 간 채널 설계와 함께 다룬다.
- **`FILE.RAW` 캐시는 이관하지 않았다** — 프리뷰 탭은 새 경로로 다시 읽으면 되고(짧은 로딩), 바이너리
  본문을 캐시 사이에 복사할 이유가 없다.
- **`shared/lib/lsp/workspace-edit-applier.ts` 의 LSP rename/delete 는 여전히 추종하지 않는다.**
  그 경로는 `commands.fileRename`/`fileDelete` 를 직접 호출해 `useRenameEntry`/`useDeleteEntry` 를
  거치지 않는다(기존에도 캐시 무효화조차 하지 않던 경로 — d-51 F1 §5 에 같은 취지로 기록돼 있다).
  같은 mutation 훅으로 통일하는 것이 옳지만 LSP 적용기의 호출 형태 변경을 수반해 범위 밖이다.
- **개명이 확장자를 바꿀 때의 언어 재적용은 이 스테이지에서 `applyModelLanguage` 로 직접 처리했다.**
  같은 결함 축(모델이 이미 있으면 `getOrCreateModel` 이 언어를 다시 적용하지 않는다)의 다른 절반인
  §4-B B7(플러그인 설치 후 기존 파일 재하이라이트)은 d-51 F6 소관이라 손대지 않았다.
- **에이전트 대기 마커(`agent-wait-marker-registry`)는 개명 시 옮기지 않았다** — 마커는 "이 경로에
  대한 사용자 조치를 기다린다"는 에이전트 쪽 상태라, 삭제(=탭 닫기)에서는 기존 규약대로 반납하고
  개명에서는 그대로 둔다. 옮길지 여부는 에이전트 프로토콜 쪽 결정이 필요하다.
- **[검토 추가] 삭제로 닫힌 파일 탭이 닫은 탭 스택에 그대로 남는다.** `close_file_tabs_under` 는 각
  탭을 기존 `close_tab` 으로 닫아 스택·활성 탭 인계·페인 정규화를 손으로 닫은 것과 같게 맞추는데
  (§3 S8 의 명시적 선택), 그 부작용으로 ⇧⌘T 가 이미 지워진 경로를 연다. 개명 축은 스택을 새 경로로
  옮기면서 삭제 축은 죽은 경로를 남기는 비대칭이다. 스택에서 빼는 것이 옳은지(재열기 대상이 아님),
  아니면 재열기 시점에 존재 검사를 붙이는지는 동작 결정이라 `docs/backlog.md` 로 이관했다.
- **[검토 추가] `layout_apply_path_change` 의 락·revision 규약은 형제 커맨드와 동일함을 재확인했다** —
  `begin_mutation` 보유 구간이 read-clone 부터 writeback 까지 이어지고, 보고할 것이 없으면 revision
  미증가·이벤트 미발행이다(§4 (3) 이후에도 그대로이며, 닫은 탭 스택만 바뀐 경우의 write-back 만
  추가됐다). 다만 프론트 `isLayoutEchoAlreadyInCache` 의 `>=` 비교는 "emitting mutation 은 반드시
  revision 을 올린다"는 불변식 위에 서 있다 — 앞으로 `finish_mutation` 호출자가 revision 을 올리지
  않으면 다른 창의 동기화가 조용히 끊긴다. 코드로 강제(`finish_mutation` 내부 증가 또는 debug assert)
  하는 것은 13개 형제 커맨드의 revision 증가 위치를 모두 옮기는 일이라 이번 범위 밖으로 남긴다.
