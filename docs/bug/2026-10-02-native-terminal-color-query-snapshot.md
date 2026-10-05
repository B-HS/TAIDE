# Native terminal 색상 query 시점 override 유실

## 대상·원인

Alacritty vendor `term/mod.rs`의 native ColorQuery와 app `terminal_dispatch.rs`입니다. native query는 index/prefix/terminator만 전달했고 consumer가 항상 theme base palette를 읽었습니다. OSC 4/10/11 등의 동적 색상을 query 응답에 반영하지 못했습니다. 소비 시점의 Core 색상을 읽는 수정도 같은 batch의 query 후 reset·다음 output과 경쟁하므로 query 시점 값과 다를 수 있습니다.

## 해결·검증

parser가 query를 생성할 때 `ColorQuery.override_color`에 당시 Core 색상을 담고, Dispatcher가 해당 값을 우선 사용합니다. override가 None인 query는 기존 palette port를 사용합니다. 기존 typed retained graph에 inline Option<Rgb>가 포함되며 Core lock을 async writer await에 유지하지 않습니다.

`cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test terminal-dispatch color_query -- --nocapture`: 1 PASS(suite 0.00초). 실제 parser의 set→query→reset→query를 한 batch로 처리하고 Core 최종 override None에서도 두 실제 writer receipt의 응답이 각각 override·base임을 확인했습니다. 최초 테스트 compile의 잘못된 supervisor 메서드는 실제 shutdown으로 수정했습니다. RED runtime을 먼저 실행한 것은 아니며 이를 RED→PASS 증거로 주장하지 않습니다. 실제 PTY/OS 픽셀의 색상 실기는 이 검사의 범위가 아닙니다.
