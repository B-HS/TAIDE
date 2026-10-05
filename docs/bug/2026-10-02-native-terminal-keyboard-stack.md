# Native terminal keyboard stack 포화 시 잘못된 title 삭제

대상은 격리 `native/taide-native-terminal/vendor/alacritty-terminal/src/term/mod.rs`의 `push_keyboard_mode`입니다. 설치 Alacritty 0.26.0 원본은 keyboard mode stack의 4,096개 상한에서 `title_stack.remove(0)`을 호출했습니다. title stack이 비면 panic, 비지 않으면 무관한 제목 이력을 손상시키는 source 오류입니다.

`native/taide-native-terminal/tests/retained.rs`의 `keyboard_stack의_상한은_title_stack을_건드리지_않는다`가 kitty keyboard를 켠 실제 Term에 4,097회 push합니다. 원본 한 줄 상태에서 `cargo test --manifest-path native/taide-native-terminal/Cargo.toml --test retained keyboard_stack --locked --offline --target-dir experiments/native-shell-spike/target -- --nocapture`는 `removal index (is 0) should be < len (is 0)` panic으로 1 FAIL(exit 101, suite 0.00초)입니다.

삭제 대상을 `keyboard_mode_stack`으로 수정한 뒤 같은 최소 검사는 1 PASS(compile 0.92초/suite 0.00초)입니다. 실제 retained capacity가 정확히 4,096개 mode 증가에 머무르고 후속 pop/push가 동작합니다. 안전 검사·상한을 끄지 않았습니다. 변경은 native source fork에만 적용하며 기존 registry package·제품/root·보호 실기 앱은 유지합니다. 전체 terminal/M8 완료 판정은 아닙니다.
