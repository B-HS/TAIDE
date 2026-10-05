# M8 Snippets 전역 이름 입력·Alert 설명

## 대상·구현

공용 `native/taide-native-ui/src/snippet-editor.rs`와 실제 `tests/snippet-editor.rs`의 기존 Snippets UI 경계입니다. 전역 이름 입력의 14px 글꼴·20px 줄 높이·세로 4px padding·1px border를 연결해 실제 높이 30px를 맞췄습니다. 명시된 egui Frame은 TextEdit margin 설정을 대체하므로 Frame에도 같은 inner_margin을 지정했습니다. Alert 설명의 실제 Label ID를 Dialog AccessKit described_by에 연결하고 기존 labelled_by를 유지합니다.

## 검증

- [x] 실제 Settings→Manage→New→Picker Home/Enter→Global 이름 입력→Escape→파일 삭제 Alert 연속 검사 1건 최종 PASS입니다. 필터 `snippet_전역_이름은_원본_줄높이를_쓰고_alert_설명은_ax로_연결된다`, build .61초/suite .12초, 1 PASS/filtered 8입니다. 실제 입력 focus·높이 30px·state의 same-frame 문자열·다음 frame의 줄 높이 20px·Alert 설명 노드 참조와 제목 참조를 확인했습니다.
- [x] 정상 Canvas Wasm check .55초 exit 0/경고 0입니다. 기존 성공 결과를 반복하지 않습니다.
- [ ] Chrome/OS 물리 입력·VoiceOver·전체 AX 그래프·모든 theme/DPI·픽셀은 이 검사의 통과 범위가 아닙니다. 실제 browser lifecycle 검사가 다음 경계입니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_전역_이름은_원본_줄높이를_쓰고_alert_설명은_ax로_연결된다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

## 실패·정정

최초 실제 높이 18.1px로 실패(1.30초/.12초)했고 custom layouter/줄 높이 연결 후 22px로 실패(1.32초/.12초)했습니다. 설치된 egui 0.36.2 TextEdit builder의 Frame 우선순위를 확인해 inner_margin을 추가했습니다. 이후 높이는 통과했지만 입력 직후 같은 frame에서 TextShape `global`을 찾는 fixture가 실패(1.19초/.12초), 진단 1회(.62초/.11초)에서 state는 이미 `global`이고 focus도 정상임을 확인했습니다. SDK는 입력 전 emptiness를 바탕으로 첫 문자 frame에 hint를 유지하고 다음 frame부터 새 galley를 칠합니다. same-frame state assertion과 정상 다음 frame paint assertion으로 fixture를 정정한 최종 검사만 재실행했습니다. 생산 SDK를 수정하거나 데이터 손실로 잘못 보고하지 않습니다. 전체 paint latency 검증은 미완료입니다.

Snippets 1/4(25%)·provider 2/4(50%)·M8 363/433(83.83%)·최종 0/8·전체 ETA 산정 보류입니다. 전체 완료 전 Git 없음·main 직접·OS/Keychain/보호 앱/사용자 데이터/의존성/lock/MSRV 불변입니다.
