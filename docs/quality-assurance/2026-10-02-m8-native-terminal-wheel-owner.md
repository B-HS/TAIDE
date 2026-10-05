# M8 native terminal raw-wheel 소유권 checkpoint

> 2026-10-02 후속: mouse tracking과 X10 fallback·pointer의 logic replay는 `2026-10-02-m8-native-terminal-mouse-adapter.md`에서 연결했습니다. 아래의 mouse 미연결/vendor 변경 없음은 이 wheel checkpoint 당시의 기록입니다. 포인터를 raw에 남기는 후속 경로에는 eframe pending-prefix hook을 추가했습니다.

## 대상과 범위

대상은 `native/taide-native-app/src/{terminal_surface,application}.rs`와 `tests/terminal-host.rs`입니다. 기존 wheel 기본 QA의 normalization·partial·live 방향키는 재사용합니다. 이번 checkpoint는 actual egui raw input이 terminal 소유일 때 global wheel smoothing에 들어가지 않도록 하는 경계입니다. 전체 mouse protocol·실제 OS/다중 창·N4/M8 완료가 아닙니다.

## 실제 실패와 수정

- [x] terminal에서 line wheel 후 다른 ScrollArea로 pointer만 이동한 actual headless 재현에서 다른 ScrollArea offset이 100 대신 91.47196으로 변했습니다. 공개 `smooth_scroll_delta`를 비워도 egui 내부 smoothing tail은 남습니다. 비공개 필드나 vendor fork로 지우지 않고 eframe의 실제 `raw_input_hook`에서 terminal 소유의 원본 wheel을 분리했습니다.
- [x] actual `request_discard` 두 UI pass 검사에서 두 번째 pass의 managed route가 true 대신 false인 RED를 확인했습니다. cumulative pass 번호는 프레임 소유권으로 사용할 수 없습니다. 완료 frame 번호로 target/captured/raw-route generation을 일치시켰습니다. assertion을 완화하지 않았습니다.
- [x] lifetime fixture의 bool 매개변수 추론 E0282는 test closure 매개변수에 bool을 명시해 수정했습니다. 생산 코드의 검사기·경고는 끄지 않았습니다.

## 구현 계약

- [x] view key는 `(ViewportId, PaneId, TabId)`이며 clip·global layer transform을 적용한 마지막 실제 UI target과 frame 번호를 저장합니다. 현재 viewport가 다른 raw hook은 보수적으로 입력을 그대로 둡니다. 전체 auxiliary window 경로 검증을 대신하지 않습니다.
- [x] RawInput의 pointer 이벤트 순서를 따라 event-time 좌표를 기록합니다. 직전 frame의 enabled target·rect·최상위 layer·viewport에 정확히 한 view만 대응할 때 vertical-dominant wheel을 분리합니다. 겹친/없는 target·수평·disabled·숨은 target·다른 viewport 입력은 원본에 남깁니다.
- [x] captured queue는 view별 최대 64개이며 초과는 명시적 오류로 거절합니다. packet은 위치와 검증된 MouseWheel variant만 담습니다. actual queue 길이/capacity 상한을 검사했지만 전체 앱 aggregate·peak RSS 게이트는 아닙니다.
- [x] `run_logic` 이후 같은 완료 frame에서 raw hook이 다시 실행돼도 이미 분리한 packet은 보존합니다. 다음 frame에는 stale packet을 정리합니다. 두 UI pass에서도 managed route를 유지하고 한 번 drain한 packet을 다시 쓰지 않습니다.
- [x] 실제 managed view는 분리된 wheel만 읽습니다. unclaimed RawInput wheel을 UI 경로에서 재수집하지 않아 다른 surface 소유권을 훔치지 않습니다. standalone 기본 wheel fixture용 legacy 분기는 유지합니다.
- [x] 현재 enabled·rect·live mode를 UI 처리 시 다시 확인합니다. mouse tracking이 켜진 target은 현재 checkpoint에서 wheel을 분리하지 않습니다. mode 전환 뒤 이전 captured wheel을 방향키로 보내지 않습니다. session 교체/닫힌 탭 retain 정리는 기존 owner 경로를 사용합니다.
- [x] `NativeApplication::raw_input_hook`에서 실제 Views router를 호출하며 실제 host fixture도 같은 hook→UI→bounded writer→child PTY를 사용합니다.

## 실행 증거

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`이며 직렬 실행했습니다.

1. 새 smoothing-tail RED 후 수정한 owner 검사 1 PASS, 최초 수정 compile 3.58초·suite 0.01초입니다. 별도 새 lifetime 검사 1 PASS, compile 2.08초·suite 0.01초입니다.
2. 이후 실제 managed-route 배선과 multi-pass 입력이 바뀌어 관련 검사를 실행했습니다. multi-pass RED 뒤 frame 번호 수정의 최종 `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib wheel_ -- --nocapture`: 3 PASS, compile 1.93초·suite 0.05초입니다. 고유 신규 검사는 owner와 lifetime 2건이며 기존 policy 1건은 변경 영향입니다. 버전별 실행을 고유 검사 수에 중복 합산하지 않습니다.
3. 최종 `cargo test --manifest-path native/taide-native-app/Cargo.toml … --test terminal-host headless_ -- --nocapture`: 2 PASS, compile 5.75초·suite 0.39초입니다. 실제 alt/app-cursor child가 ESC OA·ESC OB를 각각 한 번 받고 exit 0·owned join을 마쳤습니다. 기존 startup/IME/retry/restart 검사는 view key/route 변경의 영향 검사입니다.
4. 최종 `cargo clippy --manifest-path native/taide-native-app/Cargo.toml … --lib --test terminal-host -- -D warnings`: exit 0, 2.57초입니다. 기존 의존 Wry 17 warnings는 authored strict 결과와 구분합니다.
5. authored 위 3파일의 `rustfmt --edition 2024 --check`·추적 `git diff --check`: exit 0입니다. 같은 명시된 untracked Rust 파일의 trailing whitespace 검색은 일치 없음입니다. 같은 상태의 성공 drag/selection/font/Hub·전체 suite/계측은 반복하지 않았습니다.

## 미완료·부채

- [ ] SGR/default 및 tracking mode별 실제 mouse encoder·forced selection·pointer capture/release·mouse wheel writer는 다음 기존 N4-B 단계입니다. 이번 scope의 mouse-mode 입력 보존을 mouse 기능 완료로 계산하지 않습니다.
- [ ] 실제 auxiliary viewport raw hook/레이어·닫힌 window/mount 재사용·layout 이동 직전의 OS 좌표와 modal overlay는 제품 UI 연결 후 검사합니다. 현재 직접 current-viewport guard·hidden frame·중첩 target 검사는 실제 다중 창 실기를 대체하지 않습니다.
- [ ] 최초 UI target이 없을 때는 raw wheel을 그대로 둡니다. 실제 eframe event batching/multi-window·OS trackpad legacy delta/DPR·TUI·global peak/aggregate memory와 GPU/IME/AX는 제품 통합 gate로 남습니다.
- [ ] keyboard keypad/Kitty·context/search/link/project Hub·전체 VT·213 TS view/full cutover·Rust99%/TS 제거·N1~N8 전체는 미완료입니다.

보호 실기 bundle·TS/root/MSRV·사용자 데이터/clipboard·OS 입력기/VoiceOver는 변경하거나 실행하지 않았습니다. M8 전체 완료 전 commit/push하지 않았습니다. 이 checkpoint의 Cargo live handle은 없습니다.
