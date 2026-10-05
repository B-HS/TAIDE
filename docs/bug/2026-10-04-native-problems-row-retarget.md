# native Problems 필터 변경 뒤 행 대상 재배정

## 대상·원본 계약

`native/taide-native-app/src/problems.rs`, `problems-tests.rs`입니다. 원본 `src/features/problems/problem-list-rows.ts`는 `problem:${path}:${index}`로 행 ID를 만듭니다. severity filter가 바뀌어 index0에 다른 진단이 들어오면 같은 node의 callback도 새 진단으로 갱신됩니다. 동일 내용 진단을 여러 서버가 발행하는 경우에도 index가 진단을 구분합니다.

## 관찰·해결

error 첫 행 Click→error 필터 off→같은 첫 행 Click을 보냈을 때 native 요청은 `[(1,1)]`이고 기대는 error와 새 warning 위치 `[(1,1),(2,4)]`였습니다. actual headless renderer의 실제 AX node를 대상으로 한 RED는 compile6.84초/suite0.04초입니다.

이전 action-order 수정은 paint 시점의 Diagnostic Arc를 캡처한 뒤 pointer identity로 검사했습니다. 사건 적용 시점에 index0에 새 warning이 들어왔는데도 이전 error Arc를 찾아 요청을 거절했습니다. 반대로 index1 행이 사라졌는데 캡처한 warning Arc가 index0에 남으면 이전 행 요청을 허용할 수 있었습니다.

Open action에는 원본 node와 같은 path/index를 보존하고, 사건 적용 시점의 실제 panel group/index로 diagnostic을 조회합니다. 없는 path/index·collapsed group·Close 뒤 동작은 계속 거절합니다. root/path/1-based 좌표·HostBridge preview 경계는 기존 그대로이며 row ID를 새로 만들어 원본 계약을 바꾸지 않았습니다.

## 실제 검증

- 신규1검사는3조합을 확인합니다. 첫 행 Click→filter off→첫 행 Click은 error/warning 두 위치입니다. filter off→이전 두 번째 행 Click은0개입니다. filter off→첫 행 Click→filter on→첫 행 Click은 warning/error 순서입니다.
- app 입력 영향7건 PASS,compile5.90초/suite0.10초입니다. 명령은 `cargo test … --lib problems::tests -- --skip problems_viewport별_scroll --skip problems_프레임_batch_헤더 --skip problems_헤더버튼 --skip problems_빈필터 --skip problems_키보드와_닫기는`입니다. Open dispatch에 영향받는 행/AX Focus/키·pointer/필터/Close만 확인했고 변경하지 않은 header/viewport 수명 성공은 재사용했습니다.
- app `cargo clippy … --lib --tests -- -D warnings` exit0,15.33초입니다. 공개 Open/actual App caller는 불변이므로 기존 bin strict15.42초는 재사용합니다. Wry dependency의 기존17경고는 별도입니다.
- Rust2파일 exact rustfmt·문서 포맷·tracked diff check와 untracked no-index whitespace 검사를 확인했습니다.

공통 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--manifest-path native/taide-native-app/Cargo.toml`, `--locked --offline --target-dir experiments/native-shell-spike/target`이며 Cargo는 직렬 실행했습니다. 보호 bundle·OS·제품 TS·manifest/lock·Git은 변경하지 않았습니다. 신규 node 생성/Tab/여러 pointer·상태바/slot 전체 순서·실제 GUI/AX와 전체 M8은 미완료입니다.
