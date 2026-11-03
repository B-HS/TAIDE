# native lane terminal-agent 묶음 5/5: commands-ui-evidence

> 작성일: 2026-11-02 · lane: `terminal-agent` (worktree `TAIDE-lanes/terminal-agent`, 브랜치 `lane/terminal-agent`)
> 기준: `docs/research/2026-10-28-lane-commands-ui-spec.md` 1.4절 W1-B, 2.1절(20·21), 2.3절(52·54), 2.6절(90·91·101), 3절 묶음 W1-B. 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 결정은 추천안: 결정 3 A = 21 not-applicable, 결정 4 A = 91 not-applicable)

## 대상 ID와 판정

- [x] commands-ui-20 `terminal.copyImeDebug` → complete (코드 변경 없음, 팔레트 UI 검사 1건 + 실제 App 검사 1건)
- [x] commands-ui-21 `app.showPerfSnapshot` → not-applicable (결정 3 A, 대응표만)
- [x] commands-ui-52 cmdk `command-score` → complete (코드 변경 없음, UI crate 점수표·같은 점수 순서·빈 검색어 검사)
- [x] commands-ui-54 최근 항목(MRU) → not-applicable (원본 미구현)
- [x] commands-ui-90 use-reveal-window → complete (코드 변경 없음, 첫 프레임 테마·언어 실제 App 검사)
- [x] commands-ui-91 use-tauri-event 등 hook → not-applicable (결정 4 A)
- [x] commands-ui-101 프로젝트 아이콘 카탈로그 → complete (코드 변경 없음, 원본 목록 drift 검사, "67종" → 56종 정정)

## 원본 동작 근거

ID별 근거는 사양 2절을 따르고, 이번에 직접 다시 확인한 것만 적습니다.

- 20: `src/shared/lib/command-catalog.ts` 131-139 `isEnabled: isImeDebugEnabled`. `src/shared/lib/ime-debug.ts` 18-30의 `imeDebugEnabled`는 `false`로 시작하고 `setImeDebugEnabled`로만 켜집니다. `rg "setImeDebugEnabled\(" src` 결과는 `command-catalog.test.ts`·`ime-debug.test.ts`의 검사 호출 6건뿐이라 원본 앱에서도 항상 비활성 행입니다.
- 21: `command-catalog.ts` 140-158. `titleDefaultValue: 'Show Performance Snapshot'`, `isEnabled: isPerfEnabled`, 실행은 `printPerfReport()`로 webview 개발자 console에 프런트엔드 계측표만 출력합니다(주석: Rust 쪽 snapshot은 같은 console의 `invoke('perf_snapshot')`). 사용자 화면이 없는 개발자 계측 도구라 결정 3 A대로 not-applicable입니다. native 행은 기존 `Enablement::WebviewDiagnostics`(항상 비활성)로 남고 UI 검사 `enabled_판정은_원본_조건과_실행경로_유무를_함께_적용한다`가 단언합니다. 성능 수치 화면은 settings-54입니다.
- 52: 원본 `node_modules/cmdk/dist/chunk-NZJY6EH4.mjs`의 `commandScore`(상수 `U=1, Y=.9, H=.8, J=.17, p=.1, u=.999, $=.9999, k=.99`, 경계 정규식, 전치 분기)와 `UI/command-score.rs`가 같은 상수·분기임을 대조했습니다. native 소비 표면은 `UI/project-display-dialog.rs` `icon_matches`와 `UI/settings-code-view.rs` Picker이고 둘 다 점수 0을 숨기고 `sort_by`(안정 정렬)로 같은 점수의 원래 순서를 지킵니다. 빈 검색어는 cmdk처럼 필터하지 않습니다(`icon_matches`는 카탈로그 전체, Picker는 점수 1.0).
- 54: `docs/features/command-palette.md` 92 "MRU 는 미구현이다". `rg -i "mru|recent" src/widgets/command-palette src/features/command-palette` 0건.
- 90: `src/shared/hooks/use-reveal-window.ts` 10-33(창은 숨긴 채 만들고 테마·로케일 조회가 끝나면 `show()`). native는 `main.rs` → `eframe::run_native` 생성 closure → `NativeApplication::new`가 첫 프레임 전에 테마·로케일을 동기 적재하고(`application.rs` `presentation_inputs.load` → `presentation::apply_visuals`) vendored eframe이 창을 숨긴 채 만든 뒤 첫 프레임 뒤에 보입니다(`experiments/native-shell-spike/vendor/eframe/src/native/wgpu_integration.rs` 1113 `.with_visible(false)`, 보조 viewport 1060, `epi_integration.rs` 407-412 `post_rendering`의 첫 프레임 `set_visible(true)`).
- 91: 원본 hook 4종은 구현 기술입니다. native 대응: 이벤트 구독 → `application.rs` `PaintSink`(AppEvent sink, shell-93), Blob URL → 미리보기 texture cache(preview 행), monaco marker → `diagnostics.rs` `counts()`·문제 패널(editor-39), 원격 연결 세대 → 동결 `taide-remote-web`.
- 101: `src/shared/icons/project-icon-registry.ts` 74-131 `PROJECT_ICON_NAMES` 56종, 기본·모르는 id는 `folder`. native `UI/glyph-icons.rs` `PROJECT_ICONS: [(&str, Glyph); 56]`.

사양과 다르게 한 점

- 52: 사양은 "같은 점수 순서 보존(`icon_matches`)"을 `command-score.rs` 검사에 두라고 적었지만 `icon_matches`는 `project_display_dialog` 모듈의 비공개 함수라 그 자식 검사 모듈인 `UI/project-display-dialog-tests.rs` 끝에 검사를 추가했습니다(파일 끝 추가만). 점수표·빈 검색어·같은 점수 값 자체는 `command-score.rs`의 새 `#[cfg(test)] mod tests`에 있습니다.
- 20: 사양은 팔레트 UI 검사 1건만 요구했지만 작업 규칙의 "실제 앱 경로 도달 확인"을 위해 `application-commands-tests.rs`에 실제 App 팔레트 검사 1건을 더했습니다.

## 구현

코드(비검사) 변경은 없습니다. 근거 검사만 더했습니다.

- UI `native/taide-native-ui/src/command-score.rs`: 파일 끝 `#[cfg(test)] mod tests`
    - `점수표는_원본_cmdk_command_score와_같다`: remote-web `tests/settings-resources.rs` 749-772의 14개 사례(값은 원본 cmdk 기준, 오차 1e-15)
    - `빈_검색어는_모든_값을_남기고_같은_접두사_일치는_같은_점수다`: 빈 검색어 = `PREFIX`(0.99, 빈 값은 `CONTINUOUS`), `git-branch`/`git-fork`·`code`/`cloud` 같은 점수, 정확 일치 > 접두사
- UI `native/taide-native-ui/src/project-display-dialog-tests.rs` 끝: `아이콘_검색은_빈_검색어면_카탈로그_전체를_보이고_같은_점수는_카탈로그_순서를_지킨다`(빈 검색어 = 56종 전체 순서, `c`·`git`·`s` 검색에서 점수 내림차순·같은 점수 카탈로그 순서·점수 0 숨김, 각 검색에 같은 점수 쌍이 실제로 있음을 단언)
- UI `native/taide-native-ui/src/glyph-icons-tests.rs`: `프로젝트_아이콘_id는_원본_카탈로그와_같은_순서의_같은_목록이다`(`include_str!("../../../src/shared/icons/project-icon-registry.ts")`의 `PROJECT_ICON_NAMES = [` … `] as const` 사이 따옴표 이름 = native id 순서. 선례 `theme-editor-tests.rs` 275)
- UI `native/taide-native-ui/src/command-palette-tests.rs` 끝: `ime_디버그_복사_명령은_원본처럼_켜는_경로가_없어_항상_비활성_행으로_보이고_실행되지_않는다`(프로젝트 있는 메인 창, 라벨 "Terminal: Copy IME Debug Log" 검색 → 첫 행·목록 안·비활성·비선택, 클릭 → `Output::default()`·팔레트 열림 유지)
- APP `native/taide-native-app/src/application-commands-tests.rs`: 상수 3개(`REVEAL_THEME`, `REVEAL_LANGUAGE`, `IME_DEBUG_COMMAND`)와 검사 2건
    - `실제_app_첫_프레임은_창을_보이기_전에_설정한_테마_배경과_언어로_그려진다`(90): 설정 `vscode-light-modern`·시스템 테마 따르기 끔·`ko`로 저장 → `launch` 직후(첫 프레임 전) `global_style().visuals.panel_fill` = 그 테마 `app.background`, `dark_mode == false`, `locale.id == "ko"`. 첫 프레임 출력의 사각형 채움에 테마 배경이 있고 egui 기본 `panel_fill`은 한 번도 없음
    - `실제_app_팔레트_ime_디버그_복사는_비활성_행이라_클릭해도_실행되지_않는다`(20): 실제 App 팔레트에서 라벨 검색 → 행 비활성·비선택, 행 클릭 → 팔레트 열림 유지, `palette_commands` 비어 있음, toast 없음
- 실제 앱 경로: 20은 `main.rs` → `NativeApplication::new` → `domains/commands.rs` `show_palette` → `UI/command-palette.rs`(registry `Command::is_runnable(CommandContext)`가 `WebviewDiagnostics`에서 거짓)이고 검사는 앱의 `commands_state.palette`와 앱 프레임(`step`)을 그대로 씁니다. 90은 `main.rs`의 `eframe::run_native` 생성 closure가 부르는 같은 `NativeApplication::new`를 `launch`가 부릅니다.

## 검증

모든 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/terminal-agent`, 앱 lib는 `--test-threads=2`로 실행했습니다.

- UI lib `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib -- command_score 프로젝트_아이콘 아이콘_검색은 ime_디버그`: 6 passed(새 5건 + 기존 `프로젝트_아이콘_카탈로그는_…`)
- 앱 lib `-- 실제_app_첫_프레임은 실제_app_팔레트_ime`: 2 passed
- 형식: 첫 `--check`에서 나온 줄바꿈 차이(UI 2곳, 앱 1곳)를 Edit로 반영한 뒤 `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check`, `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` 통과
- 형식 수정 뒤 UI lib 전체 `--features native-host,inspection --lib`: 174 passed(앞 묶음 169 + 새 5)
- 형식 수정 뒤 앱 lib `-- domains::commands::tests`: 6 passed(앞 묶음 W1-A 4건 + 새 2건)
- `cargo check --quiet --all-targets`: UI(`--features native-host,inspection`)·앱 모두 오류 없음. 남은 경고(UI `tests/workbench.rs` 1816 `unused mut`, 앱 `editor-code-actions.rs` `is_applying`, `zen.rs` 543, `application-inlay-hints-tests.rs` `fixture_with_emmet_peek`, vendor `wry-preview`)는 이 묶음이 고치지 않은 파일의 기존 경고입니다
- 동결 remote-web `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib`: 통과(기존 dead_code 경고만). UI 변경은 모두 `#[cfg(test)]` 검사 코드라 Wasm 표면은 바뀌지 않습니다
- UI 하네스 `--test workbench`는 하네스·비검사 코드 변경이 없어 실행하지 않았습니다
- `Cargo.toml`·`Cargo.lock` 변경 없음(새 의존성 없음)

## 대응표

`docs/quality-assurance/2026-10-09-native-function-matrix.md`에서 commands-ui-20·21·52·54·90·91·101 일곱 행만 갱신했습니다. 상단 합계 표는 바꾸지 않았습니다.

- 20 `unwired` → `complete`, 52 `partial` → `complete`, 90 `partial` → `complete`, 101 `partial` → `complete`(기능 이름의 "67종"을 "56종"으로 정정)
- 21 `unwired` → `not-applicable`(결정 3 A), 54 `missing` → `not-applicable`(원본 미구현), 91 `partial` → `not-applicable`(결정 4 A, 비고에 대응 행)

## 남은 범위

- 90의 실제 OS 창 표시 시점은 사양 4.2-5(배치 31 실기) 허락 항목이라 하지 않았습니다. 자동 검사는 첫 프레임 전 테마·언어 적재와 vendored eframe의 숨김 생성·첫 프레임 표시 코드 근거까지입니다.
- 실패 검사 선행: 일곱 ID 모두 동작이 이미 있거나(20·52·90·101) 재분류(21·54·91)라 구현 단계가 없습니다. 새 검사는 처음부터 통과했고, 기대값은 원본(cmdk 값표, TS 아이콘 목록, 원본 활성 조건)에서 가져왔습니다.

## 수정 (lane 리뷰 반영, 2026-11-03)

### 90: complete 유지, 근거 보강

- 리뷰 지적: 창을 숨긴 채 만들고 첫 프레임 뒤에 보이는 부분은 vendored eframe 코드를 읽은 근거뿐이고, 자동 검사는 첫 프레임 전 테마·언어 적재까지입니다.
- 보강한 소스 추적
    - `main.rs` 40-49는 `NativeOptions.viewport`에 크기·타이틀바 설정만 넣고 visible을 지정하지 않습니다. `window_builder` hook(`window-state.rs`)에도 visible 지정이 없습니다(`rg visible` 0건).
    - `main.rs` 54 `eframe::create_native` → vendored `lib.rs` 376 → wgpu 경로(앱 feature는 `wgpu`만, `Cargo.toml` 56) `create_window`(`wgpu_integration.rs` 1098-1113)가 `viewport_builder(…, native_options, …)` 결과, 즉 앱 옵션과 hook이 반영된 builder에 마지막으로 `.with_visible(false)`를 붙입니다. 앱 설정이 이를 덮을 수 없습니다.
    - 첫 그리기(`wgpu_integration.rs` 825-836의 paint 직후) `integration.post_rendering(window)`가 `epi_integration.rs` 407-412에서 `is_first_frame`을 한 번만 꺼내 `set_visible(true)`를 부릅니다. 첫 그리기는 생성 closure(`NativeApplication::new`, 테마·로케일 동기 적재) 뒤입니다.
- 판정을 낮추지 않은 이유: 원본 hook의 목적(테마 배경이 준비되기 전 다른 배경이 보이지 않음) 중 앱이 책임지는 부분(첫 프레임 전 테마·언어 적용, 첫 프레임에 egui 기본 배경 없음)은 실제 App 검사로 고정돼 있습니다. 창 숨김·표시는 앱이 덮을 수 없는 라이브러리 동작이고, OS 창을 만드는 자동 검사는 GUI 실행 금지라 이 환경에서 만들 수 없습니다. OS 표시 시점 실기는 배치 31(사양 4.2-5)에 남습니다. 대응표 90 행에 위 추적을 덧붙였습니다.

### 101: 레거시 TS 파일 의존 기록

- `glyph-icons-tests.rs`의 `프로젝트_아이콘_id는_원본_카탈로그와_같은_순서의_같은_목록이다`는 `include_str!("../../../src/shared/icons/project-icon-registry.ts")`로 레거시 TS 파일을 읽습니다(선례 `theme-editor-tests.rs` 275). `src/` 트리를 정리하거나 옮기는 작업은 이 검사를 함께 옮기거나 목록을 native 쪽 고정값으로 바꿔야 합니다. 그 전에 지우면 UI crate 검사 빌드가 컴파일되지 않습니다.

### lane 공통: 병합 충돌 예상 지점

모두 추가 위주지만 shell lane 등과 같은 줄을 건드립니다. 메인 병합 때 확인합니다.

- `native/taide-native-app/src/host.rs`: `HostIntegrations.agent_cli` 필드와 생성 리터럴 3곳, 검사 전용 `connect_with_agent_cli`.
- `native/taide-native-app/src/shell_keymap.rs`: import 줄과 intent 처리 앞쪽 match.
- `native/taide-native-app/src/command-dispatch.rs`: 검사 목록.
- `native/taide-native-app/src/application.rs`: 검사 모듈 목록 끝(`terminal_agent_tests`, 이번 수정으로 연결 조건이 `cfg(all(test, target_os = "macos"))`), `State` 리터럴의 `command_colors`·`cli_status`, `background_tick`의 한 줄.
- `native/taide-native-ui/src/intents/terminal_agent.rs`: `Cli` 변형을 더하면서 `NewTerminal` 변형이 여러 줄로 바뀌었습니다. 수동 재포맷이 아니라 현재 rustfmt가 요구하는 형태입니다(`cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` 차이 없음).

### 수정 검증

- 이 묶음은 문서·대응표만 바뀌었습니다. UI crate 서식 확인으로 `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check`를 실행했고 차이가 없었습니다. 그 밖의 검사는 앞 검증 절 결과를 재사용합니다.
