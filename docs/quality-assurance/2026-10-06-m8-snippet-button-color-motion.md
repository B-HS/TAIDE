# M8 Snippets Button 색상 전환

## 대상 파일

- `native/taide-native-ui/src/button-color-motion.rs`
- `native/taide-native-ui/src/lib.rs`
- `native/taide-native-ui/src/snippet-editor.rs`
- `native/taide-native-ui/tests/snippet-editor.rs`

## 리포트

후속 최종 소스는 애니메이션 property를 편집기 lifetime namespace로 격리하고 Drop에서 그 owner의 Motion/ID 목록을 회수합니다. context 변경 시 이전 owner를 회수한 뒤 새 context에 연결합니다. 같은 widget id를 연속 프레임에 다른 편집기로 교체해도 이전 색상을 상속하지 않습니다. 기존 UI auto id·focus/AX 계약은 변경하지 않았습니다. Back ghost는 비활성 프레임의 배경을 즉시 숨기는 SDK 옵션 대신 투명색을 target으로 사용해 이탈150ms 페이드를 유지합니다. Primary/Outline/Ghost/Destructive의 실제 Rust variant로 최종 버튼을 한 번만 구성합니다.

원본 `src/shared/ui/button.tsx`의 transition-all과 설치된 Tailwind 기본 duration150ms/cubic-bezier(0.4,0,0.2,1)를 배경·글자/아이콘 색상에 연결했습니다. 일반/작은 outline의 배경과 글자/아이콘, primary/destructive hover 알파 변경을 공용 경계로 처리합니다. destructive Confirm은 일반 버튼을 먼저 만든 뒤 덮어쓰지 않으며 동일 property의 두 target을 한 프레임에 등록하지 않습니다.

실제 상태는 부동소수점 premultiplied gamma-sRGB 채널로 보존하고 도형을 만들 때만 Color32로 반올림합니다. 원본 hex의 legacy 보간과 같은 색의 알파 변경 범위입니다. 임의의 modern wide-gamut 색상 간 Oklab 보간 완료로 주장하지 않습니다. 전환 중 target 변경은 현재 보간 값에서 시작하고, 되돌림은 CSS reversing shortening factor를 사용합니다. 동일 프레임 다중 pass는 시간을 재시작하지 않습니다. 초기 표시와 한 프레임 이상 미표시 뒤 재등장은 target부터 표시하며 완료 후 추가 repaint를 요청하지 않습니다.

공식 근거: [CSS Color 4 색상 보간](https://www.w3.org/TR/css-color-4/#interpolation-space)의 legacy sRGB/알파 premultiplication, [CSS Transitions 1 전환 시작 및 반전](https://www.w3.org/TR/css-transitions-1/#starting)의 current value·reversing-adjusted start value·shortening factor를 적용했습니다. 설치된 egui Context.read_response/Color32의 실제 구현을 확인했습니다.

## 검증

명령의 공통 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, Cargo 실행 파일은 `/Users/hyunseokbyun/development/rust/cargo/bin/cargo`, target-dir은 `/private/tmp/taide-m8-menu-build.j6Efnw`입니다.

1. `cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --target-dir ... --lib button_색상은 -- --nocapture`: 최초 1 PASS, build2.36초/suite0.00초·filtered50. 150ms 곡선의 midpoint, 연속 반전 두 번과 단축 시간, 완료 상태, 투명 알파 보간, 미표시 뒤 target 재설정을 확인했습니다.
2. `cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir ... --test snippet-editor snippet_button_hover는 -- --nocapture`: 최종 1 PASS, build0.67초/suite0.11초·filtered13. actual outline 배경이 hover 입력 프레임에 점프하지 않고 중간값을 거쳐 완료하는 것, 이탈 전환과 실제 destructive Confirm의 알파 변경을 확인했습니다.
3. `cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir ...`:0.70초 exit0/경고0.

후속 변경의 별도 위험만 추가 검사했습니다.

1. `cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --target-dir ... --lib button_전환은 -- --nocapture`: 최종1 PASS, build0.90초/suite0.01초·filtered51. 실제 Context clock에서 같은 button id/다른 owner의 연속 프레임 격리와 Drop 때 해당 cache만 제거되고 다른 owner의 cache는 유지되는 것을 확인했습니다. 최초 명령 filter 오타는0 tests였으며 성공 증거에서 제외했습니다. 실제 첫 실행 실패는 SDK가 요구하는 unapplied texture delta를 fixture가 clear하지 않은 것입니다. 기존 Scene과 같은 회수 계약으로 정정했습니다.
2. `cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir ... --test snippet-editor snippet_ghost_back은 -- --nocapture`: 최초1 PASS, build2.63초/suite0.11초·filtered14. 실제 Back 도형이 투명→hover→포인터 이탈에도 hover→중간 알파→투명으로 전환하는 것을 확인했습니다.
3. 같은 normal Canvas Wasm 명령의 변경 후 검사0.98초 exit0/경고0입니다. 첫 성공 명령을 동일 소스로 반복한 것이 아니라 후속 owner/ghost 생산 변경의 portable 경계를 덮습니다.

실제 UI 검사의 최초 compile 실패는 fixture가 private Settings Appearance에서 Snippets 전용 색상을 읽은 것입니다. 생산 계약을 노출하지 않고 기존 public presentation::color와 실제 theme 키로 기대값을 읽도록 정정했습니다. 성공한 검사는 반복하지 않았습니다. 이전 Chrome 수명 결과와 focus-visible 검사는 당시 소스의 성공으로 재사용하며 최신 Wasm bindings/Chrome raster 성공으로 바꾸어 표기하지 않습니다.

## 남은 범위

- focus border/ring·disabled opacity·shadow의 전체 transition-all, 전체 원본 UI/theme/DPI/글꼴·전역 focus graph는 미완료입니다.
- 실제 자동완성 UI/삽입·전체 native shutdown·나머지 Settings/App/assets·최종 gate가 남습니다.

Snippets1/4(25%)·전체363/433(83.83%)·최종0/8을 유지합니다. 전체 ETA는 산정 보류이며 전체 M8 완료 전 commit/push하지 않습니다.
