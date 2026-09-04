# 성능 기준선 측정 체크리스트 — 8지표 (사용성 배치 4 ③, 2026-09-04)

> 계약 `acknowledge/2026-09-04-usability-batch4-contract.md` §C, 사용자 결정
> `acknowledge/2026-09-04-usability-batch4-user-decisions.md` §2·2차 §7, 조사 원문
> `research/2026-09-04-batch4-topics1-5-research.md` 주제 3a·3b.
>
> **왜 체크리스트인가**: 8지표의 실측은 **앱을 실제로 띄워야** 나온다. 자동 검증으로 잠근 것은
> 계측 자체의 동작(`src/shared/lib/perf-mark.test.ts`)과 연산 횟수 예산(§5)뿐이고, 밀리초 수치는
> 이 문서의 절차대로 사용자가 직접 재서 §3 표에 적는다. 수치를 적기 전에는 **어떤 성능 주장도
> 하지 않는다** — 이 배치의 조사에서 "코드만 보면 유력했던 병목" 2건(키맵 디스패치·검색 flatten)이
> 실측으로 기각됐다.

## 1. 계측 켜기

계측은 **프로세스 하나에 게이트 하나**다. Rust `infra::perf` 가 환경변수 `TAIDE_PERF` 로 게이트를
정하고, 프론트는 부팅 직후 `perf_snapshot` 으로 그 값을 읽어 자기 계측에 그대로 적용한다
(`src/entities/app/perf.ipc.ts`). 그래서 dev·릴리스 어느 쪽이든 **양쪽이 함께 켜지고 함께 꺼진다.**

| `TAIDE_PERF` | Rust | 프론트 |
|--------------|------|--------|
| 미설정 | dev on / 릴리스 off | 같음 |
| `1`·`true`·`on` | on | on |
| `0`·`false`·`off` | off | off (dev 빌드에서도 꺼진다 — 계측 오버헤드 배제 대조용) |

```bash
bun run tauri dev                                              # dev = 기본 on
TAIDE_PERF=1 /Applications/TAIDE.app/Contents/MacOS/TAIDE       # 설치본 실측
```

- [ ] 앱을 띄우고 devtools 콘솔에서 `await window.__TAURI_INTERNALS__.invoke('perf_snapshot')` 이
      `enabled: true` 를 돌려주는지 확인한다
- [ ] 커맨드 팔레트(⌘⇧P)에 **`App: Show Performance Snapshot`** 이 보이는지 확인한다
      (게이트가 꺼져 있으면 이 커맨드는 아예 나타나지 않는다)

## 2. 측정 절차 (지표 1건마다 동일)

1. **초기화** — `await window.__TAURI_INTERNALS__.invoke('perf_reset')` (Rust 누적치 0으로).
   프론트 누적치는 창을 새로 고치면(⌘R) 0에서 시작한다.
2. **조작** — §3 의 "조작" 열에 적힌 동작을 **1회만** 한다. 여러 번 하면 `count` 가 늘고
   평균(`totalMs / count`)으로 읽어야 한다.
3. **읽기** — 팔레트 `App: Show Performance Snapshot`(프론트 표 2개) + 콘솔
   `invoke('perf_snapshot')`(Rust `entries`·`counters`). 두 표의 이름이 §3 의 "읽는 곳" 이다.
4. **기록** — §3 표의 빈칸에 `ms` 를 적고 체크박스를 채운다. 같은 기기·같은 저장소에서
   **3회 반복해 중앙값**을 적는다.

> 기준 픽스처: 파일 5,000개 이상 · 커밋 1,000개 이상인 실제 저장소 1개(예: 이 저장소 자체)와,
> 1KB·1MB 두 파일. 기기·저장소가 바뀌면 이전 수치와 비교하지 않는다.

## 3. 8지표

프론트 이름은 `src/shared/lib/perf-mark.ts` 의 `PERF_MEASURE`·`PERF_COUNTER`, Rust 이름은
`src-tauri/src/infra/perf.rs` 의 `SpanSlot`·`CounterSlot` 이다. **이름은 계약** — 바꾸면 이 표에
적힌 과거 수치와 비교할 수 없게 된다. 계측 지점의 JSDoc 은 이 표의 번호를 `metric N`(영어 JSDoc
규약)으로 가리킨다.

| # | 지표 | 조작 | 읽는 곳 (프론트 / Rust) | 측정값 | 체크 |
|---|------|------|------------------------|--------|------|
| 1 | 부팅 → 첫 페인트 | 앱을 완전히 종료했다 다시 실행 | `boot.reveal` / `setup.main_window`·`setup.locale_warm`·`setup.state_restore`·`setup.deferred_restore` | ______ ms | [ ] |
| 2 | 프로젝트 전환 | 사이드바에서 **다른** 프로젝트 클릭(이번 실행에서 처음 여는 것) | `project.switch` / `project_open`·`project_activate` | ______ ms | [ ] |
| 3-a | 파일 열기 (소, 1KB) | 탐색기에서 1KB 파일을 **처음** 연다 | `file.open` / `file_open` | ______ ms | [ ] |
| 3-b | 파일 열기 (대, 1MB) | 탐색기에서 1MB 파일을 **처음** 연다 | `file.open` / `file_open` | ______ ms | [ ] |
| 4-a | 팔레트 열기 | ⌘⇧P | `palette.open` / — | ______ ms | [ ] |
| 4-b | 팔레트 입력 응답 | 팔레트에서 4글자 입력 | `palette.filter` / — | ______ ms | [ ] |
| 5 | 트리 펼침 | 파일 200개 이상인 디렉터리를 펼친다 | `tree.toggle` / `tree_toggle` | ______ ms | [ ] |
| 6 | git status | 변경 20건 이상인 상태에서 git 뷰를 연다 | — / `git_status` | ______ ms | [ ] |
| 7 | 전역 검색 | 200건 이상 매치되는 단어를 검색 | `search.results` / `search_run`·`search_list_files` | ______ ms | [ ] |
| 8 | 터미널 출력 처리량 | 터미널에서 `seq 2000000` 을 실행하고, 실행 전후로 스냅샷을 1회씩 (경과 초와 함께) | `terminal.output-bytes`·`terminal.output-chunks` / `pty.output_bytes`·`pty.output_chunks` | ______ MB/s | [ ] |
| 9 | 메모리 | 파일 20개를 열었다 모두 닫은 뒤 | devtools Memory 스냅샷 + `monaco.editor.getModels().length` + `queryClient.getQueryCache().getAll().length` | ______ MB / ______ 모델 | [ ] |

### 3.1 각 지표를 읽을 때 주의할 것

- **지표 1** — 프론트 `boot.reveal` 은 `main.tsx` 모듈 평가 끝 → 창 `show()` 구간이다. 그 앞의
  Rust 부팅은 `setup.*` 4구간이 따로 있으므로 **둘을 더해야** 사용자가 체감하는 시간에 가깝다.
  창을 2개 이상 여는 세션에서 두 번째 창은 이 값을 기록하지 않는다(시작 마크가 첫 측정에 소비된다).
- **지표 2·3** — 캐시가 이미 있으면 0에 수렴하는 것이 **정상**이다. 프로젝트/파일을 그 실행에서
  처음 열 때만 유효한 수치가 나온다. `file.open` 은 "탭 활성 → monaco 인스턴스 준비" 구간이라,
  이미 열려 있던 탭으로 되돌아가는 전환은 측정 대상이 아니다.
- **지표 4** — 마크 지점은 `command-palette.tsx` 에 **배선 완료**다(`PALETTE_OPEN_REQUESTED`/
  `PALETTE_QUERY_CHANGED` 마크 → 결과 목록 커밋 이펙트에서 측정). 이미 열려 있는 팔레트의 모드 전환은
  "열기" 로 세지 않으므로, 4-a 는 닫힌 상태에서 ⌘⇧P 로만 측정한다.
- **지표 6** — 프론트 계측 없이 Rust `git_status` 만 본다. **캐시 도입 후 `count` 는 히트까지 세므로
  호출 횟수 자체는 예전과 같이 늘어난다** — 떨어지는 것은 `totalMs` 와 평균이다(`totalMs / count`).
  "저장마다 전량 재조회하는가" 를 보려면 횟수가 아니라 **평균**을 본다: 캐시가 살아 있으면 평균이
  마이크로초대(= `GitStatus` clone 비용)로 내려앉고, 매번 무효화되면 밀리초대에 머문다.
- **지표 8** — 두 카운터의 차이가 **재부착 리플레이 비용**이다. Rust `pty.output_bytes` 는 셸이
  낸 전량, 프론트 `terminal.output-bytes` 는 이 창의 xterm 이 실제로 그린 양이다. 배경 탭의
  터미널은 프론트 쪽이 늘지 않고, 탭으로 돌아오면 스크롤백 리플레이만큼 한 번에 늘어난다.
- **지표 9** — 메모리는 상주 계측이 없다. devtools Memory 패널이 정본이고, `FILE.CONTENT`/`FILE.RAW`
  캐시 회수(계약 §C.2-4 M3)의 효과는 닫은 뒤 쿼리 캐시 항목 수로 확인한다.

## 4. 순수 함수 벤치 (수동)

```bash
bun run bench      # = bun run scripts/bench-frontend.ts
```

- **`bun test`·CI 에서는 절대 돌리지 않는다.** 벽시계 임계는 러너 부하로 깨진다(조사 3a 위험 항목).
- 케이스: `fuzzyFilter`(5k·20k·50k 경로) · `findMatchingKeymapEntry` · `appendSearchFileMatches`
  (2,000배치×5) · 키바인딩 충돌 인덱스(223행 전수 조회). 코퍼스는 고정 시드라 실행 간 동일하다.
- 참고 실측(2026-09-04, Apple Silicon dev 머신, bun 1.4.0 — **다른 기기와 비교 금지**):

  | 케이스 | ms/round (구현 직후) | ms/round (웨이브 2 종료 재측정) | 조사 시점(수정 전) |
  |--------|--------------------|------------------------------|-------------------|
  | `fuzzyFilter` 5,000 | 0.76 | **0.642** | 3.96 |
  | `fuzzyFilter` 20,000 | 2.43 | **2.295** | 12.68 |
  | `fuzzyFilter` 50,000 | 5.98 | **5.560** | **31.23** |
  | `findMatchingKeymapEntry` (23엔트리) | 0.007 | **0.007** | 0.002 (기각축) |
  | `appendSearchFileMatches` (2,000×5) | 0.54 | **0.546** | 0.33 (기각축) |
  | 키바인딩 충돌 인덱스 + 223조회 | 0.074 | **0.074** | 0.41 ×2/render |

  두 열의 차이는 **같은 기기의 실행 간 잡음**이다(코드 변경 없음). 조사 시점 열의 출처는
  `research/2026-09-04-batch4-topics1-5-research.md` 주제 3a §2·§5.

- [x] 성능 관련 코드를 고친 뒤 `bun run bench` 를 돌려 위 표와 비교했다 (2026-09-04 웨이브 2 통합)

## 5. 연산 횟수 예산 테스트 (자동, `bun test`)

벽시계 대신 **연산 횟수**를 단언해 CI 에서도 결정적으로 도는 회귀 가드다.

| 파일 | 잠근 계약 |
|------|-----------|
| `src/shared/lib/perf-budget.test.ts` | `fuzzyFilter` 는 후보당 라벨을 1회만 읽는다 · 키바인딩 충돌 조회는 카탈로그 크기와 무관하게 맵 조회 1회(전수 조회는 행 수에 선형) |
| `src/entities/search/search-result-budget.test.ts` | `appendSearchFileMatches` 는 배치당 맵 조회 1회·기록 1회 — 누적 그룹 수와 무관(감사 §1-3 O(n²) 재그룹 회귀 가드) |
| `src/shared/lib/perf-mark.test.ts` | 계측 자체의 계약 — 게이트 off 시 무기록, 시작 마크 소비, 세션 발행 상한(`PERF_MARK_LIMIT`) 초과 시 발행만 중단 |

카운팅 도구는 `src/shared/testing/counting-map.ts`(`CountingMap`) 하나를 공유한다.

Rust 쪽도 벽시계가 아니라 횟수로 잠갔다.

| 테스트 | 잠근 계약 |
|--------|-----------|
| `domain::theme::service` `번들_요약은_list_themes_를_반복_호출해도_한_번만_파싱된다` | 번들 테마 38종(약 1.1MB JSON)의 요약 역직렬화가 **프로세스당 1회**(`summary_parse_count() == 1`) |
| `domain::git::commands` `무효화_한_번마다_status_계산은_한_번뿐이다` | 무효화 1회 + 조회 5회 → 워크트리 워크 **1회** |
| `domain::git::commands` `매번_무효화되면_계산_횟수는_캐시_도입_전과_같다` | 조회마다 무효화 → **5회**(캐시가 실제 변경을 삼키지 않음) |
| `infra::perf` 게이트 off 5축 | 게이트 off 면 span·counter 어느 쪽도 기록하지 않는다 |
| `infra::watcher` `무시_디렉토리_생성은_그_하위를_다시_인덱싱하지_않는다` | `npm install` 류가 전 트리 재인덱싱을 유발하지 않는다 |

## 5.1 Rust 실측 — `cargo test` 픽스처 기준 (참고, 2026-09-04)

**실기 수치가 아니다.** 앱을 띄우지 않고 각 함수를 `cargo test` 안에서 직접 호출해 잰 값이며,
`cargo test` 는 debug(unoptimized) 프로파일이라 절대값이 릴리스보다 크다. **비교는 같은 프로파일
안의 전/후만** 유효하다. 픽스처는 측정 후 제거했으므로 재현하려면 다시 만들어야 한다. §3 의 8지표를
대신하지 않는다 — 그 표는 여전히 사용자가 실기로 채운다.

| 계약 항목 | 대상 | 전 | 후 |
|----------|------|-----|-----|
| C.2-5 ① | `list_themes` (20회 평균, 사용자 테마 0) | 13.443 ms | **0.744 ms** (약 18배) |
| C.2-5 ③ | `read_children` 엔트리 타입 판정 (2,000 엔트리 1회 순회, 10회 평균) | `metadata()` 2.685 ms | **`file_type()` 1.233 ms** (−54%) |
| C.2-5 ④ | `file_open` 읽기 (2,288,890 B 텍스트, 20회 평균) | sniff + 전량 재읽기 150.185 µs | **단일 리더 108.033 µs** (−28%) |
| C.2-5 ② | `flush_dirty_layouts` (더러운 프로젝트 8개, 5회 평균) | 41.364 ms/flush (async 워커 점유) | 같은 41.364 ms — **`spawn_blocking` 스레드로 이동**(총 작업량 불변) |
| C.2-6 ① | `project_open` 이 전역 guard 를 쥐는 구간 | open IO + capability attach **전량**(워처 walk 포함, 이 저장소 stat 워크 2.5~10 s) | **등록 절반만**(맵 insert 3회, µs 대) |
| C.2-6 ② | 워처 인덱싱 대상 엔트리 (이 저장소, warm) | 557,474 | **1,461** (약 382배) |
| C.2-6 ② | 워처 stat 워크 / 상주 메모리 | 3.79 s / 약 105 MB per project | **0.01 s / 약 0.2 MB** |
| C.2-6 ③ | `git_status` 캐시 히트 (release 빌드, 98행 dirty) | 웜 계산 8.48 ms | **히트 20.3 µs** (약 417배) |

- C.2-5 ② 는 **동작 불변이 목적**이다 — 2초마다 async 런타임 워커를 41 ms 붙잡던 fsync 가 blocking
  스레드로 옮겨졌을 뿐 총 작업량은 같다. 이 항목만 "빨라짐" 이 아니라 "블로킹 해소" 로 읽는다.
- C.2-6 ① 도 `project_open` **응답 시간은 의도적으로 불변**이다(attach 를 await). 줄어든 것은
  그 사이 `file_save`·`git_*`·`layout_*`·`tree_toggle` 이 정지하던 시간이다.
- FE 쪽 전/후는 §4 표(`fuzzyFilter` 50k 31.23 → 5.56 ms, 충돌 조회 0.41 ×2 → 0.074 ms)를 본다.

## 6. 계측 자체의 비용

- **off**: `perfCount` 는 완전 no-op, `perfMark` 는 `performance.now()` 1회 + `Map.set` 1회(저빈도
  7지점뿐). 이 비대칭은 의도된 것이다 — 릴리스 빌드에서 `TAIDE_PERF=1` 로 켠 세션의 **부팅 마크가
  게이트 응답보다 먼저** 찍히기 때문에, 마크 시각만은 게이트와 무관하게 남겨야 지표 1이 측정된다.
  자세한 근거는 `src/shared/lib/perf-mark.ts` 의 `perfMark` doc.
- **on**: 지표당 `performance.mark`/`measure` 1건. 세션 누적 `PERF_MARK_LIMIT`(500)건을 넘으면
  타임라인 발행만 멈추고 누적 통계는 계속 쌓인다(40KB 회전 로그·devtools 버퍼 보호).
- 프론트는 파일 로그로 아무것도 흘리지 않는다. dev 빌드에서만 측정 1건당 `console.debug` 한 줄이
  나간다.

## 7. 남은 항목

- [x] 지표 4(팔레트) 마크 지점이 `command-palette.tsx` 에 배선됐는지 확인 (2026-09-04 — 배선 완료)
- [x] 계약 §C.2-6 대형 3건(Rust) 전후 수치 기록 → §5.1 (**`cargo test` 픽스처 기준**. 실기
      `perf_snapshot` 전/후는 아래 항목으로 남는다)
- [ ] **실기 8지표 측정** — §2 절차대로 앱을 띄워 §3 표의 빈칸을 채운다 (사용자 몫)
- [ ] 실기에서 §5.1 의 대형 3건을 `perf_snapshot` 으로 재확인 — `project_open`(가드 대기 체감) ·
      워처 attach 후 RSS · `git_status` 평균(§3.1 지표 6 주석)
- [ ] 계약 §C.2-7 FE 가상화 묶음 전후로 지표 5·7 재측정
- [ ] monaco 지연 로딩(별도 배치)이 들어오면 지표 1 재측정
