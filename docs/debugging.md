# 디버깅 가이드 — 정본

> TAIDE 를 진단할 때의 진입점·도구·이 프로젝트에서 실증된 계측 기법을 모은다.
> 원칙: **재현·계측이 먼저, 추측 수정 금지.** 가능하면 결함을 드러내는 실패 테스트를 먼저 만든다.

## 1. 로그

> dev(`bun run tauri dev`)와 설치본(prod)은 identifier 가 갈라진다(계약 d-49) — 로그·
> `app_data_dir`·keyring·window-state 가 서로 다른 디렉토리에 쓰인다. 이 문서의 실행·계측 절차는
> 전부 dev 기준이다. (구 `dev.taide.app`/`dev.taide.app.dev` — 2026-08-29 `net.gumyo.taide`/
> `net.gumyo.taide.dev` 로 개명)

| 항목 | dev (`net.gumyo.taide.dev`) | 설치본/prod (`net.gumyo.taide`) |
|------|------------------------------|----------------------------------|
| 경로 | `~/Library/Logs/net.gumyo.taide.dev/TAIDE.log` | `~/Library/Logs/net.gumyo.taide/TAIDE.log` |
| 특성 | UTC 타임스탬프 · 40KB 회전(tauri-plugin-log) | 동일 |
| 활용 예 | 원격 접속 서버 포트 발견(`원격 접속 서버 기동: port=` 라인 — e2e 하네스도 이 라인을 파싱), 부팅 시퀀스·에러 추적, 프론트 에러(d-48 — `console.error`/`console.warn`·`window` `error`/`unhandledrejection` 포워딩) | 좌동 — devtools 가 없는 릴리스 빌드에서 이 로그가 유일한 진단 창구 |

## 2. 실행·검증 사다리

- dev 실행: `bun run tauri dev`. 프론트는 Vite HMR, **Rust 는 재컴파일·재시작 필요**.
- 검증: `bun run verify` = typecheck → eslint → prettier → bun test → cargo fmt/clippy/test.
  프론트 번들 회귀는 `bunx vite build` 로 추가 확인.
- 검증 없이 "통과했다"고 보고하지 않는다. 실패를 발견하면 고치고 재검증 후에만 완료 보고.

## 3. HMR 이 통하지 않는 것들 (실증된 함정)

| 변경 대상 | 반영 조건 | 근거 |
|-----------|-----------|------|
| 번들 테마·로케일(`src-tauri/resources/`) | 앱 재시작(Rust `include_str` 임베드) | d-46 실기 |
| 앱 아이콘(`src-tauri/icons/`) | 재컴파일 — `build.rs` 의 `rerun-if-changed=icons` 가 트리거(2026-08-28 신설). dev 도크 아이콘은 `generate_context!` 가 컴파일 타임에 icns 를 임베드해 `RunEvent::Ready` 에서 적용하는 구조라, 이 가드 이전엔 아이콘만 바꾸면 영원히 미반영이었다 | `docs/acknowledge/2026-08-28-app-icon-prompt-spark.md` § dev 미반영 |
| Rust 코드 전반 | tauri CLI 가 감지해 재빌드·재시작 | - |

## 4. 실증된 계측 기법 (선례 — 재사용 권장)

모두 실제 결함의 근본 확정에 사용됐다. **계측 코드는 확정 후 반드시 원복한다.**
(§4.1 의 `TAIDE_PERF` 레지스트리만 예외 — 상주 계측이라 원복 대상이 아니다.)

- **원격 페이지 WS 몽키패치**: 원격 접속 페이지에서 WebSocket send/receive 를 몽키패치해
  IPC 트래픽을 관찰 — 저장 직후 구식 캐시 채택(d-43)의 근본을 이걸로 확정했다.
- **프레임 갭 계측**: rAF 델타를 기록해 롱스톨(예: 100ms+)을 검출 — 피커 드래그 프리징(d-45)
  에서 "웹 파이프라인 단독으로는 스톨 미재현(240무브 max 47ms)"을 실측해 네이티브 축을
  특정했다. 스크립트 드래그(pointer 이벤트 합성)와 조합하면 재현이 결정적이 된다.
- **레지스트리 소스 실물 확인**: 의존 크레이트의 실제 소스를 cargo 레지스트리에서 직접 연다
  (`~/development/rust/cargo/registry/src/index.crates.io-*/<crate>-<ver>/`). tao 0.35.3 의
  `set_theme` 무단락 전역 재적용(d-45), tauri 2.11.5 dev 도크 아이콘 임베드 경로(§3)를 이렇게
  확정했다. 문서·기억보다 소스 실물이 정본이다.
- **대조 환경 격리**: 같은 코드를 결함 축이 배제된 환경(예: 네이티브 IPC 가 실효 없는 원격
  페이지)에서 돌려 층을 분리한다 — "웹 무죄 → 네이티브 유죄" 식 소거법.

### 4.1 Rust 상주 계측 — `TAIDE_PERF` (`infra::perf`)

앱에 내장된 계측 레지스트리다. 위 기법들과 달리 **상주**하며(원복하지 않는다), dev·릴리스 양쪽
빌드에 컴파일된다. 설계 근거는 `docs/architecture.md` §2.2, 배치 계약은
`docs/acknowledge/2026-09-04-usability-batch4-contract.md` §C.2-1.

**켜기·끄기**

| `TAIDE_PERF` | 결과 |
|--------------|------|
| 미설정 | dev(`debug_assertions`) 빌드 **on**, 릴리스 빌드 **off** |
| `1` · `true` · `on` (대소문자 무관) | 강제 on |
| `0` · `false` · `off` | 강제 off (dev 빌드에서도 끈다) |
| 그 외 값 | 빌드 기본값으로 되돌아감 |

```bash
TAIDE_PERF=1 bun run tauri dev          # dev 는 어차피 기본 on
TAIDE_PERF=0 bun run tauri dev          # 계측 오버헤드를 배제하고 대조하고 싶을 때
TAIDE_PERF=1 /Applications/TAIDE.app/Contents/MacOS/TAIDE   # 설치본에서 사용자 제보용 수치 뽑기
```

게이트는 프로세스 시작 시 한 번 읽고 고정된다(실행 중 변경 불가). **off 일 때 계측 지점의 비용은
원자적 `load` 1회**다 — `Instant::now()` 도, 어떤 원자적 쓰기도 하지 않는다.

**읽기·초기화**

`perf_snapshot()` / `perf_reset()` 커맨드(app 도메인). 개발 중에는 devtools 콘솔에서 바로 부른다.

```js
await window.__TAURI_INTERNALS__.invoke('perf_snapshot')
await window.__TAURI_INTERNALS__.invoke('perf_reset')   // 새 측정 창을 연다
```

응답은 `{ enabled, entries, counters }` 다. `enabled: false` 면 모든 수치가 0인 것이 정상이며,
"아무 일도 없었다"와 구분하는 유일한 단서다. 둘 다 **원격 거부**(`DesktopProcessDiagnostics`)라
원격 미러에서는 호출할 수 없다 — 레지스트리는 프로세스 전역 1벌이고, 켠 주체는 데스크톱
사용자이기 때문이다.

**`entries` — 구간 시간(밀리초, 슬롯 선언 순서 고정)**

| 이름 | 구간 |
|------|------|
| `setup.main_window` | `create_main_window` |
| `setup.locale_warm` | `warm_builtin_catalogs`(번들 로케일 3종 약 200KB 파싱) |
| `setup.state_restore` | `AppState::new` + `restore_state` + `app.manage` 전량 |
| `setup.deferred_restore` | 보조 창 복원 + 프로젝트 워처 재부착 + 원격 비밀번호 캐시 |
| `project_open` · `project_activate` | 커맨드 전체(**뮤테이션 guard 대기 포함** — guard 보다 먼저 선언해 나중에 drop 된다) |
| `file_open` · `tree_toggle` · `tree_reveal` · `git_status` · `search_run` · `search_list_files` | 커맨드 전체 |

각 항목은 `count`(호출 수) · `totalMs`(누적) · `maxMs`(최대 1회)다. 호출된 적 없는 슬롯도 0으로
항상 나온다 — "무엇이 계측되고 있는가"가 스냅샷만으로 드러나야 하기 때문이다.

**`counters` — 누적 카운터(구간 시간 없음)**

| 이름 | 단위 |
|------|------|
| `pty.output_bytes` · `pty.output_chunks` | 바이트 / 청크 수 (터미널 처리량 = 두 스냅샷 사이 벽시계로 나눈다) |
| `lsp_send` | 호출 수 |
| `command.unlisted` | 커맨드 이름 표에 없던 invoke 수 (tauri 플러그인 커맨드 등) |
| `command.<이름>` | 그 커맨드의 invoke 수 — **한 번이라도 불린 것만** 이름순으로 붙는다 |

pty 리더 루프와 `lsp_send` 에 **구간 시간을 넣지 않은 것은 의도**다. 청크마다 `Instant::now()` 를
두 번 부르면 계측이 측정 대상을 왜곡한다(조사 3b §7). 커맨드별 호출 수는 `invoke_handler` 클로저
한 곳에서 세므로 커맨드 본문에 계측 코드가 없다 — N+1 IPC 탐지에 이걸 먼저 본다.

### 4.2 프로파일러용 빌드 — `[profile.profiling]`

`[profile.release]` 는 `strip = true` 라 릴리스 바이너리에 심볼이 없어 `sample`·Instruments
Time Profiler 가 쓸모없는 스택을 뱉는다. 워크스페이스 루트 `Cargo.toml` 의
`[profile.profiling]`(`inherits = "release"`, `strip = false`, `debug = 1`)이 그 용도다.

```bash
cargo build --profile profiling      # target/profiling/
sample <pid> 10 -f /tmp/taide.sample
/usr/bin/time -l <바이너리>          # max RSS
```

**이 프로필은 어디서도 암묵 참조되지 않는다** — `tauri build` 는 `release`(또는 `--debug`)를 쓰고,
`.github/workflows/{ci,cache-warm,release}.yml` 중 어느 것도 `--profile profiling` 을 지정하지
않는다. 배포 바이너리 특성(계약 9575a33)은 이 프로필 추가로 바뀌지 않는다.

### 4.3 프론트엔드 상주 계측 — `perf-mark.ts`

§4.1 의 짝이다. **게이트는 하나뿐**이다: 프론트는 부팅 직후 `perf_snapshot` 을 1회 호출해
`enabled` 를 읽고 자기 계측에 그대로 적용한다(`src/entities/app/perf.ipc.ts`). 그래서
`TAIDE_PERF=0` 으로 띄운 dev 빌드는 Rust·프론트가 **함께** 꺼지고, 릴리스 빌드에 `TAIDE_PERF=1`
을 주면 함께 켜진다. 원격 미러는 이 커맨드가 `REMOTE_DENIED` 라 조회를 건너뛰고 빌드 기본값
(릴리스 = off)을 유지한다.

**읽기** — 커맨드 팔레트에서 `App: Show Performance Snapshot`(게이트가 켜진 창에만 나타난다).
콘솔에 표 2개를 찍는다.

| 표 | 내용 |
|----|------|
| `measures` | `boot.reveal` · `project.switch` · `file.open` · `tree.toggle` · `search.results` · `palette.open` · `palette.filter` — 각 `count`·`totalMs`·`maxMs`·`lastMs` |
| `counters` | `terminal.output-bytes` · `terminal.output-chunks` — 이 창의 xterm 이 실제로 그린 양 |

이름과 각 구간의 정의, 8지표 측정 절차는 `docs/quality-assurance/2026-09-04-perf-baseline.md`.

**동작 계약**(`src/shared/lib/perf-mark.ts`)

- 시작 마크는 **첫 측정에 소비된다.** 계측 지점이 이펙트라 무관한 이유로도 다시 도는데(리프레시로
  도착한 트리 페이지 등), 소비 규칙이 없으면 낡은 마크의 나이를 그 조작의 비용으로 보고하게 된다.
- 게이트가 꺼져 있어도 **마크 시각만은** 기록한다(`performance.now()` 1회 + `Map.set` 1회).
  릴리스 빌드에서 부팅 마크는 게이트 응답보다 먼저 찍히므로, 이게 없으면 지표 1만 영구히 측정
  불가가 된다. 고빈도 경로용 `perfCount` 는 꺼진 상태에서 완전한 no-op 이다.
- 세션 발행 상한 `PERF_MARK_LIMIT`(500) 초과 시 User Timing 발행만 멈추고 누적 통계는 계속
  쌓인다. 프론트는 파일 로그(§1의 40KB 회전)로 아무것도 흘리지 않으며, dev 빌드에서만 측정 1건당
  `console.debug` 한 줄을 남긴다.

Rust 수치는 같은 콘솔에서 §4.1 의 `invoke('perf_snapshot')` 으로 나란히 읽는다 — `shared` 레이어는
IPC 를 소유한 `entities` 를 참조할 수 없어 팔레트 커맨드가 대신 읽어주지 않는다.

## 5. e2e 하네스

- 실행: `TAIDE_E2E_PASSWORD=<비밀번호> bun run e2e` (비밀번호는 셸 프로필 export 권장.
  로그인 무재시도·5회 실패 시 60초 잠금). 앱은 사전에 떠 있어야 하며 포트는 하네스가
  로그(§1)에서 자동 발견한다. 사용법 정본: `docs/quality-assurance/2026-08-18-e2e-harness.md`.
- **픽스처 루트는 `~/Library/Caches/net.gumyo.taide.dev/e2e-fixtures`** — Vite watch 루트 밖이면서
  FSEvents 감시가 되는 위치여야 한다. `os.tmpdir()`(`/var/folders`)은 macOS FSEvents 미감시로
  기각(실측), 리포 안은 Vite full-reload 유발로 기각. 정본 JSDoc: `e2e/lib/paths.ts`.
- 기지 리스크: 앱 재시작 직후 첫 런은 spec 05 가 vtsls 콜드 인덱싱으로 실패할 수 있다(직후
  격리 재실행 통과). 스위트 2회차부터 후반 스펙 no-shim 연쇄(C8)가 가능하나 서버 무죄가
  라이브 프로브로 확정된 하네스 webkit 열화다(dev 전용·자가 회복 — 파일럿 보고서 §7-C8).
- 단언 약화 금지: 스펙이 앱 결함을 드러내면 스펙을 앱에 맞추지 말고 앱을 고친다.
- webkit `type()` 은 IME 손상 가능 — 텍스트 입력은 클립보드 붙여넣기 방식을 쓴다(하네스에
  이미 반영).

## 6. 흔한 결함 클래스 (이 코드베이스에서 실제 발생 — 수정 시 회귀 주의)

| 클래스 | 요지 | 정본 |
|--------|------|------|
| 캐시-버퍼 역행 | 저장 성공 후 FILE.CONTENT 캐시가 구식이면 렌더 채택 분기가 버퍼를 되돌림 — 저장 성공 지점에서 캐시 동기 패치 필수 | d-43 계약 |
| 외부 변경 무효화 누락 | `.git` 워처는 index/HEAD 만 본다 — 워크트리 파일 변경은 `fs:changed` 에서 git 쿼리 무효화로 이어야 함 | d-44 계약 |
| 네이티브 IPC 홍수 | 고빈도 UI 이벤트(드래그 move)가 네이티브 IPC 를 직결 호출하면 메인 스레드 포화 — 변화 가드·코얼레싱·커밋 시점 지연으로 상한 | d-45 계약 |
| 프리뷰/적용 경합 | monaco·shiki 재적용류는 `runExclusive` 큐 안에서만 — 큐 밖 호출은 홍수 시 경합 | d-45 계약 §3 |

## 7. 진단 팁

- 상태의 단일 출처는 Rust(ADR-0004) — view 이상은 먼저 Rust 상태·이벤트 흐름을 의심한다.
- 커맨드 dispatch 는 커스텀 테이블(`src-tauri/src/lib.rs`) — 커맨드 이름 불일치는 patch 대상
  테이블과 `collect_commands!` 패리티 테스트가 잡는다.
- cargo 환경: `CARGO_HOME=$HOME/development/rust/cargo`, `RUSTUP_HOME=$HOME/development/rust/rustup`
  를 PATH 에 선행 export(셸 프로필에 없으면 수동).
