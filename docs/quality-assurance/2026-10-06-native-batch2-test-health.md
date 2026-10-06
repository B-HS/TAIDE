# 전환 배치 2 단계 1 — 기존 실패 테스트의 원인 규명과 수정 (2026-10-06)

## 결론

- H1 tooltip 4건: 제품 결함이 아니라 테스트 하네스가 낡은 것이었습니다. 네 테스트가 실제 앱처럼 tooltip provider의 frame 시작·종료를 호출하도록 고쳤고 기대값은 바꾸지 않았습니다.
- H2 병렬 전용 실패 3건: 원인은 OS 파일 감시(FSEvents) 스트림 등록이 프로세스 안에서 한 번에 하나씩 처리되고 1회에 약 0.6~0.9초가 걸리는 것입니다. 시간 상한은 그대로 두고, 실제 watcher를 시작하는 테스트끼리 등록 구간이 겹치지 않도록 격리했습니다.
- H3 `tests/projects.rs`의 낡은 테스트: 기대하던 세 계약이 모두 폐기됐거나 lib 테스트가 이미 덮고 있어 삭제했습니다.
- H4 `openpty` code -6: XNU 커널 내부 의사 오류 `EREDRIVEOPEN`이 PTY master 할당 경합에서 사용자 공간으로 새는 것입니다. `taide_infra::pty::spawn`의 `openpty` 호출에 이 오류에만 반응하는 한정 재시도를 두었습니다.
- 검증 계약 V1~V5는 전부 exit 0입니다. 회차별 결과는 아래 표에 있습니다.

## H1. tooltip 테스트 4건

### 원인

- `native/taide-native-ui/src/tooltips.rs`의 `Provider::show`는 tooltip을 즉시 그리지 않고 `Viewport::pending`에 쌓기만 합니다. 실제 그리기는 `Provider::finish_frame`의 `render_pending`에서 일어나고, `begin_frame`은 새 pass마다 `pending`을 비웁니다.
- 실제 앱은 `native/taide-native-app/src/application.rs`의 `ui()` 한 번 안에서 `self.tooltips.begin_frame(&context)`(3755행) → `self.shell.show(ui, &snapshot, &mut surfaces)`(3855행, 상태바와 problems 패널이 여기서 그려집니다) → `self.tooltips.finish_frame(&context)`(4286행) 순서로 구동합니다. `problems::Views`와 `status_ide::Icons`는 앱의 provider를 공유합니다(383행, 388행).
- 실패하던 네 테스트는 `show_status`·`show_panel`·`status_ide::show`만 호출하고 `finish_frame`을 호출하지 않아 tooltip이 한 번도 그려지지 않았습니다. 같은 파일의 `problems_tooltip은_키보드_focus에_즉시_열리고_escape와_blur에_닫힌다`, `problems와_ide_status는_실제_caller에서_tooltip_skip을_공유한다`는 이미 두 호출을 하고 있어 통과하고 있었습니다.

### 변경

| 파일 | 테스트 | 변경 |
| --- | --- | --- |
| `native/taide-native-app/src/problems-tests.rs` | `problems_tooltip은_ax_role과_trigger_description을_열린_동안만_연결한다` | frame 클로저 앞뒤에 `views.tooltips.begin_frame`·`finish_frame` 추가 |
| 같은 파일 | `problems_tooltip은_테마색과_원본_글꼴_여백_테두리를_렌더한다` | 동일 |
| 같은 파일 | `problems_버튼은_원본_모서리와_높이_툴팁방향을_렌더한다` | 동일 |
| `native/taide-native-app/src/status-ide.rs` (`mod tests`) | `ide_tooltip은_dark_light_원본_테마와_12px_글꼴을_사용한다` | frame 클로저 앞뒤에 `icons.tooltips.begin_frame`·`finish_frame` 추가 |

제품 코드(`problems.rs`, `status-ide.rs` 본문, `tooltips.rs`, `tooltip-trigger.rs`)는 바꾸지 않았습니다.

### 기대값의 TS 근거

- `src/shared/ui/tooltip.tsx` 27행: `rounded-md`(6px), `border border-tooltip-border`, `bg-tooltip-background`, `px-3 py-1.5`(12px·6px), `text-xs`(12px·줄 높이 16px), `text-app-foreground`. 테스트의 반지름 6, 여백 12·6, 글꼴 12, 줄 높이 16, 색 토큰 `tooltip.background`·`tooltip.border`·`app.foreground`와 일치합니다.
- `src/app/providers/app-providers.tsx` 7행·20행: `TOOLTIP_DELAY_MS = 400`인 `TooltipProvider` 하나가 앱 전체를 감쌉니다. native가 provider 하나를 frame 단위로 구동하는 구조와 대응합니다.
- `src/features/window/status-bar.tsx` 87~135행: problems 토글(`problems.toggleAriaLabel`, `side='top'`)과 IDE 상태(`ide.title`, `TooltipContent side='top'`, 미실행이면 tooltip 없음).

## H2. 병렬 실행에서만 실패하던 3건

### 관찰

- 세 실패는 모두 단언 실패가 아니라 5초 상한의 `Elapsed`였습니다(`projects-tests.rs` 테스트 전체 상한, `remote-projects-tests.rs`의 `control.entered` 대기, `remote-dispatch-tests.rs`의 `Fixture::call`).
- lib 테스트 중 실제 OS watcher를 시작하는 곳은 `projects.rs`의 `build_files`·`build_git`뿐이고, 이를 타는 테스트는 `projects::tests`, `remote_projects::tests`, `remote_dispatch::tests` 세 모듈뿐입니다.
- 세 모듈 9건만 병렬로 실행해도 실패했습니다(8 통과 1 실패, 9.03초). 322건 전체의 CPU 부하가 원인이 아니라는 뜻입니다. `projects::tests::원본처럼…` 단독 실행은 0.69초에 통과했습니다.

### 측정

`taide_infra::watcher::start_watch`와 `WatcherHandle::stop`의 소요 시간을 빈 합성 임시 디렉터리에서 쟀습니다. 측정에는 `projects-tests.rs`에 임시 진단 테스트를 넣어 두 번 실행한 뒤 삭제했고 최종 diff에는 남아 있지 않습니다.

| 조건 | start_watch | stop |
| --- | --- | --- |
| 순차 5회, 1차 | 1.70초, 0.84초, 0.61초, 0.62초, 0.64초 | 7~70ms |
| 순차 5회, 2차 | 0.61초, 0.58초, 0.62초, 0.60초, 0.60초 | 19~85ms |
| 스레드 4개 동시 | 1.14초, 2.04초, 2.94초, 3.87초 | 2.73초, 1.83초, 0.92초, 0.02초 |
| 스레드 8개 동시 | 0.87초부터 약 0.6~0.9초 간격으로 5.52초까지 | 마지막 start가 끝날 때까지 대기 |

- 시작 1회는 약 0.6초(프로세스의 첫 호출은 1.7~2.1초)이고, 동시에 호출하면 완료 시각이 등차로 늘어납니다. 즉 등록은 프로세스 안에서 한 번에 하나씩 처리됩니다.
- 정지는 그 자체로는 빠르지만 진행 중인 시작이 모두 끝날 때까지 밀립니다.
- 세 모듈의 테스트가 동시에 프로젝트를 열면 watcher 시작이 서로의 뒤에 줄을 서고, 앞에 6건 이상이 쌓이면 5초 상한을 넘습니다. 어느 테스트가 넘는지는 실행 순서에 따라 달라져 실패 테스트가 매번 달랐습니다.
- 느린 구간이 notify 8.2.0 FSEvents 백엔드의 스트림 생성·시작(`fsevent.rs`의 `run`) 안이라는 것까지만 확인했습니다. `ScopedIdCache`는 빈 디렉터리에서 할 일이 없고 debouncer tick은 75ms입니다. 그 안의 어느 호출이 0.6초를 쓰는지는 확정하지 못했습니다.

### 변경

상한은 hang 방지용이며 성능 단언이 아닙니다. 상한을 늘리지 않고, 직렬화된 OS 자원을 쓰는 테스트끼리 겹치지 않게 했습니다. 등록이 어차피 직렬이라 전체 실행 시간은 늘지 않습니다.

- `native/taide-native-app/src/projects-tests.rs`: `static OS_WATCH_REGISTRATION: tokio::sync::Mutex<()>`와 테스트 전용 `NativeProjects::exclusive_os_watch_registration()`을 추가했습니다. 세 테스트 모듈이 모두 `NativeProjects`를 이미 가져다 쓰므로, 범위 밖 파일의 모듈 공개 범위를 바꾸지 않고 공유할 수 있습니다. 이 파일은 `#[cfg(test)]`에서만 컴파일됩니다.
- 프로젝트를 실제로 여는 7개 테스트의 첫 줄에서 이 guard를 잡습니다: `projects-tests.rs` 3건, `remote-projects-tests.rs` 2건(`실제23명령…`, `열기3진입점…`), `remote-dispatch-tests.rs` 2건(`실제_project_file_raw…`, `실제_pty_spawn…`). `원본처럼…` 테스트는 5초 상한 바깥에서 잡습니다.
- 한 테스트가 fixture를 여러 개 겹쳐 만들기 때문에 fixture가 아니라 테스트 단위로 잡았습니다. `await`를 가로질러 들고 있으므로 std가 아닌 tokio mutex를 썼습니다.

제품 코드는 바꾸지 않았습니다. 경쟁 조건은 테스트끼리의 것이고 제품의 프로젝트 열기는 blocking worker에서 실행됩니다.

## H3. `tests/projects.rs`의 낡은 테스트

삭제한 테스트는 `미연결_hook는_열기_전_거절하고_실패한_attach는_등록을_회수한다`입니다. 세 부분이 각각 다음과 같습니다.

| 기대하던 동작 | 현재 계약 | 현재 계약을 검증하는 테스트 |
| --- | --- | --- |
| hooks가 켜져 있으면 열기를 거절 | 2026-10-04에 제거(`docs/bug/2026-10-04-native-project-hooks-enabled-rejection.md`). 거절 없이 열고 reconcile을 기다리지 않음 | `projects::tests::hooks_enabled_프로젝트는_거절없이_열리고_guard밖_reconcile을_기다리지_않는다` |
| IDE lockfile 준비 실패 시 열기 실패와 등록 회수 | 경고 후 계속(`docs/quality-assurance/2026-10-04-m8-native-project-attachment-errors.md`). 원본 근거는 `src-tauri/src/domain/file/capability.rs` 27·56행, `src-tauri/src/domain/git/watch.rs` 90·128행의 `Option` 반환과 `log::warn!` | `projects::tests::원본처럼_watcher와_lockfile_준비_실패는_프로젝트_commit을_거절하지_않는다` |
| 감독자 정지 뒤 열기 거절, 등록 없음, 추적 0 | 유지 | `projects::tests::hooks_off_생산용_factory는_서버없이_restore하고_종료후_새_attach를_거절한다` |

같은 파일에서 실제 watcher를 시작하는 두 테스트(`실제_project_open_restore…`, `실제_os_watcher는…`)는 H2와 같은 이유로 4건의 등록이 줄을 서서 5초 상한에 가까웠습니다(삭제 직후 실행 3.94초). 통합 테스트는 lib의 `#[cfg(test)]` 항목을 볼 수 없어 파일 안에 `static OS_WATCH_REGISTRATION: Mutex<()>`와 `exclusive_os_watch_registration()`을 따로 두고 두 테스트에서 잡습니다.

## H4. terminal-host의 간헐적 `openpty` 실패

### 재현

수정 전 `--test terminal-host` 1회 실행에서 52건 중 2건이 `Internal("failed to openpty: Os { code: -6, kind: Uncategorized, message: \"Unknown error: -6\" }")`로 실패했습니다. 실패한 두 테스트는 가장 먼저 끝난 묶음에 있었습니다.

### 원인

- `portable-pty-0.9.0/src/unix.rs` 22~47행: `libc::openpty`가 0이 아니면 `io::Error::last_os_error()`를 문자열에 넣어 `bail!`합니다. 즉 -6은 `openpty`가 남긴 errno입니다.
- XNU `bsd/sys/errno.h`: `#define EREDRIVEOPEN (-6) /* redrive open */`. 커널 내부 전용 의사 오류입니다.
- XNU `bsd/kern/tty_ptmx.c`: `ptmx_clone`은 비어 있는 가장 낮은 minor를 예약 없이 고릅니다. `ptmx_get_ioctl`은 그 사이 다른 open이 같은 minor를 차지했으면 "we've been raced" 주석과 함께 -1 표식을 돌려줍니다.
- XNU `bsd/kern/tty_dev.c`의 `ptcopen`: 그 표식을 `EREDRIVEOPEN`으로 바꿉니다.
- XNU `bsd/vfs/vfs_vnops.c`의 `vn_open_auth`: "EREDRIVEOPEN: means that we were hit by the tty allocation race" 주석과 함께 `max_retries = 10`까지 다시 시도하고, 넘으면 오류를 그대로 돌려줍니다. 6번째부터는 재시도마다 `nretries * 10ms`씩 쉽니다.
- 테스트 바이너리는 시작과 동시에 테스트 스레드 수만큼 `/dev/ptmx`를 엽니다. 모두 같은 최저 minor를 고르므로 한 번에 하나만 이기고, 11번 연속으로 진 스레드가 errno -6을 받습니다. 실패가 첫 묶음에 몰린 것과 맞습니다.
- PTY·child 미회수가 원인이 아닙니다. 그 경우는 `ptmx_max` 한도에서 `ENXIO`(+6)가 나옵니다.

### 변경

대상 파일은 `crates/taide-infra/src/pty.rs`입니다.

- `XNU_REDRIVE_OPEN_ERRNO = -6`, `PTY_OPEN_REDRIVE_LIMIT = 3`, `open_pty_redriving_raced_allocation`을 추가하고 `spawn`의 `openpty` 호출을 이 함수로 감쌌습니다.
- 실패 문구에 `io::Error::from_raw_os_error(-6)`의 Debug 표현이 들어 있을 때만 최대 3회 다시 엽니다. 커널이 이미 자체 backoff를 거친 뒤 실패를 돌려주므로 사용자 공간에서는 따로 쉬지 않습니다.
- portable-pty가 errno를 문자열로만 전달해 구조화된 판별이 불가능합니다. 비교 대상 문자열은 하드코딩하지 않고 표준 라이브러리로 같은 표현을 만들어 씁니다.

Tauri 앱 영향: 같은 `spawn`을 쓰므로 함께 적용됩니다. -6이 아닌 실패는 시도 1회 뒤 이전과 같은 `AppError::Internal(error.to_string())`을 돌려주고, 성공 경로는 그대로입니다. 달라지는 것은 이전에 거짓으로 실패하던 경우뿐입니다. 한도를 넘으면 마지막 오류를 이전과 같은 값으로 돌려줍니다.

추가한 단위 테스트(같은 파일 `mod tests`):

- `할당_경합으로_밀린_pty_열기는_한도_안에서_다시_열어_성공한다`
- `다시_열기_한도를_넘긴_할당_경합은_마지막_오류를_그대로_돌려준다`
- `할당_경합이_아닌_pty_열기_실패는_다시_열지_않는다`

`tests/terminal-host.rs`는 바꾸지 않았습니다.

## 실행한 명령과 실제 결과

모든 cargo 명령은 계약의 형태에 출력 축약용 `--quiet`만 덧붙였습니다. 실행 대상과 판정은 같습니다.

### 수정 전 재현과 중간 확인

| 명령 | 결과 |
| --- | --- |
| `cargo test … --lib -- problems::tests::problems_tooltip problems::tests::problems_버튼 status_ide::tests::ide_tooltip` (수정 전) | 1 통과, 4 실패, 0.04초, exit 101 |
| 같은 명령 (H1 수정 후) | 5 통과, 0.06초, exit 0 |
| `cargo test … --lib` (H1만 적용) | 320 통과, 2 실패(`projects::tests::원본처럼…`, `remote_projects::tests::열기3진입점…`, 둘 다 `Elapsed`), 8.41초, exit 101 |
| `cargo test … --lib -- projects::tests remote_dispatch::tests remote_projects::tests` (H2 수정 전) | 8 통과, 1 실패, 9.03초, exit 101 |
| `cargo test … --lib -- projects::tests::원본처럼` | 1 통과, 0.69초, exit 0 |
| 3개 모듈 명령 (H2 수정 후) | 9 통과, 12.77초, exit 0 |
| `cargo test … --test projects` (낡은 테스트 삭제 직후, 격리 전) | 4 통과, 3.94초, exit 0 |
| `cargo test … --test terminal-host` (H4 수정 전) | 50 통과, 2 실패(`openpty` code -6), 1.73초, exit 101 |

### 검증 계약

| 항목 | 명령 | 결과 |
| --- | --- | --- |
| V1 1회차 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir experiments/native-shell-spike/target` | 322 통과, 0 실패, 13.10초, exit 0 |
| V1 2회차 | 같은 명령 | 322 통과, 0 실패, 13.59초, exit 0 |
| V1 3회차 | 같은 명령 | 322 통과, 0 실패, 12.23초, exit 0 |
| V2 | `cargo test … --test projects …` | 4 통과, 4.53초, exit 0 |
| V3 1~5회차 | `cargo test … --test terminal-host …` | 매 회 52 통과 0 실패, 1.69초·1.73초·1.53초·1.70초·1.72초, 전부 exit 0 |
| V4 | `cargo test --manifest-path Cargo.toml -p taide-infra --locked --offline` | lib 296 통과(42.62초), 통합 3 + 3 통과, exit 0 |
| V5 | `cargo check --manifest-path native/taide-native-app/Cargo.toml …` | exit 0 (vendored wry 경고 17건만 출력) |
| V5 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 출력 없음, exit 0 |
| V5 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 출력 없음, exit 0 |

각 항목은 해당 입력의 마지막 수정 뒤에 실행했습니다.

### 계약 밖 추가 검사

| 명령 | 결과 |
| --- | --- |
| `cargo fmt --manifest-path crates/taide-infra/Cargo.toml -- --check` | 출력 없음, exit 0 |
| `cargo clippy … --test projects … -- -D warnings` | exit 0 |
| `cargo clippy … --all-targets … -- -D warnings` | exit 101. `src/terminal-input-tests.rs:267`의 `clippy::await_holding_lock` 1건. 이 단계에서 수정하지 않은 파일이며 lib test 타깃의 오류는 이 1건뿐입니다 |

## 실패했다가 고친 내역

- 같은 가정으로 반복 실패한 수정은 없습니다. H1·H2·H4 모두 수정 전 실패를 먼저 재현하고 한 번의 수정으로 통과했습니다.
- H2는 처음에 322건 전체의 부하를 의심했으나 9건만으로 재현되어 가정을 버리고 시작·정지 시간을 직접 쟀습니다.

## 범위 밖에서 발견한 것

- 삭제한 낡은 테스트는 `agent_hooks_enabled = true`인 채로 생산용 reconcile(`NativeProjects::new`)을 사용했습니다. 현재 제품에서는 열기가 성공하므로, 단언이 실패해 panic하기 전에 `agent-hooks-attach` 작업이 `taide_infra::home::home_dir_env`의 실제 HOME을 대상으로 `reconcile_enabled_hooks`(`crates/taide-runtime/src/agent_hook_reconcile.rs` 79~100행)를 실행했을 수 있습니다. 이 경로는 TAIDE 표식이 있는 사용자 레벨 hook 파일을 다시 쓰고 hooks 서버를 띄웁니다. 이 단계에서는 그 테스트를 한 번도 실행하지 않았고 HOME도 읽지 않았으므로, 이전 실행들이 실제로 파일을 바꿨는지는 확인하지 못했습니다.
- `cargo clippy --all-targets -- -D warnings`가 `src/terminal-input-tests.rs:267`에서 실패합니다. 수정 범위 밖이라 고치지 않았습니다.
- watcher 시작 1회가 약 0.6초이고 직렬이라는 측정값은 제품에도 그대로 해당합니다(Tauri 앱도 같은 `start_watch`를 씁니다). 프로젝트 하나를 열면 file·git watcher 두 개가, 프로젝트 여러 개를 복원하면 그 합이 백그라운드에서 순서대로 걸립니다. 이 단계의 목표가 아니어서 손대지 않았습니다.

## 남은 위험과 확인이 필요한 것

- [ ] 실제 HOME의 사용자 레벨 hook 설정이 과거 테스트 실행으로 바뀌지 않았는지 사용자 확인이 필요합니다. TAIDE 앱을 hooks 켠 채로 다시 실행하면 같은 reconcile이 현재 서버 기준으로 다시 맞춥니다.
- [ ] V3의 5회 통과는 재시도가 매 회 실제로 발동했다는 증거가 아닙니다. 수정 전에는 이 단계 1회 중 1회, 직전 작업자 6회 중 4회가 실패했고 수정 후 5회가 모두 통과했다는 통계적 근거입니다.
- [ ] `openpty` 재시도는 portable-pty가 실패 문구에 `io::Error`의 Debug 표현을 넣는 동안만 발동합니다. 그 형식이 바뀌면 조용히 이전 동작(즉시 실패)으로 돌아갑니다. portable-pty 버전을 올릴 때 다시 확인해야 합니다.
- [ ] 앞으로 실제 OS watcher를 시작하는 lib 테스트를 새로 만들면 `NativeProjects::exclusive_os_watch_registration()`을 잡아야 합니다. 잡지 않으면 같은 간헐 실패가 돌아옵니다.
- [ ] 격리 뒤에도 각 5초 상한은 그 테스트 자신의 watcher 시작(1회 0.6~2.1초)을 덮어야 합니다. 이 기기보다 등록이 훨씬 느린 환경에서는 다시 넘을 수 있습니다.
- [ ] 실기 확인: 터미널 탭 여러 개를 한꺼번에 복원할 때 터미널이 모두 열리는지(재시도 대상 경로), 프로젝트를 여러 개 복원할 때 파일 변경 감지가 늦게 시작되는 정도.
- watcher 시작이 느린 지점이 notify FSEvents 백엔드 안의 어느 호출인지는 미확정입니다.
