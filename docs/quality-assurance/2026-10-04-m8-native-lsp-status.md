# M8 실제 편집기 LSP 상태 표시

## 대상과 원본 기준

대상은 `native/taide-native-app/src/lsp.rs`, 신규 `lsp-status.rs`, `application.rs`, `presentation-refresh.rs`, 기존 `tests/lsp-recovery.rs`입니다. 원본은 `src/widgets/window-chrome/status-bar-content.tsx`와 `src/features/window/status-bar.tsx`, 공유 locale의 `window.lspStatus`입니다.

실제 LspBridge의 서버 session registry를 watch로 게시하고 NativeApplication의 포커스된 shell 프로젝트에 맞춰 AppSurfaces.status_bar에서 읽습니다. 문서별 Synced reply를 별도 상태 map에 쌓지 않습니다. 같은 서버에 여러 문서가 있어도 registry entry 하나만 집계하며 다른 프로젝트를 섞지 않습니다. Running은 실행 수, Degraded는 crashed 표시로 대응하고 준비 중인 서버도 전체 수에 포함합니다. session이 없거나 publisher가 닫혔으면 표시하지 않습니다.

원본의 성공/오류 색상 키 `statusIndicator.success/error`, 11px 글꼴, 12px 원형 check/X 아이콘, 아이콘 간격4px를 사용합니다. 초기 생성·정상 테마 갱신·실시간 테마 preview 모두 동일 Appearance를 갱신합니다. 문구는 실제 locale를 받아 기존 메시지 확장 경로를 사용합니다. 새 recovery 문구·spinner·버튼은 추가하지 않았습니다. 기존 일반 상태 문자열은 별도로 유지합니다. 실제 egui Label을 사용하지만 OS 접근성 실기는 아직 수행하지 않았습니다.

상태 전달에서는 channel identity·generation·phase를 유지하되 pending 등 독립적인 요청 수 변화 때문에 phase 통지를 버리던 전체 SessionSnapshot 비교를 제거했습니다. 실제 현재 snapshot을 게시하고 복구 Synced 전달 전에 상태와 repaint를 갱신합니다. 초기 generation의 기존 reply 순서는 유지합니다. 이것은 코드 대조로 확인한 과도한 비교 정정이며 해당 경합의 별도 RED를 실행했다고 주장하지 않습니다.

## 최소 검증과 실제 결과

명령 prefix는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo`, 공통 flags는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] `test --lib lsp::status::tests`에서 집계/프로젝트 분리/closed publisher 1 PASS를 확인했습니다. compile11.84초·suite0.01초(handle83586). 같은 실행의 renderer는 egui0.36.2 TexturesDelta를 headless fixture에서 처리하지 않은 오류로 실패했으며 제품 렌더 실패로 분류하지 않습니다.
- [x] 공식 TexturesDelta 계약대로 headless 출력의 texture delta를 명시적으로 clear한 뒤 실패 renderer만 `test --lib lsp::status::tests::상태_렌더`로 실행했습니다. 1 PASS·compile3.92초/suite0.04초(handle9634). ko/ja/en 문구·성공/오류 색상·11px 글꼴·원형/경로 개수·아이콘 gap4px·빈 상태에서 widget 없음입니다. GPU texture upload나 실제 화면 픽셀 비교가 아닙니다. 앞선 집계 성공은 재사용했습니다.
- [x] 변경된 publisher의 기존 실제 child recovery 검사 `test --test lsp-recovery` 1 PASS·compile12.43초/suite0.82초(handle31128). UUID 합성 root의 두 Rust 문서·같은 server entry, 다른 프로젝트 없음, 실제 child kill/Degraded 0/1·기존 backoff 중 최신 편집·generation1/Running 1/1·최신 문서 formatting·마지막 문서 retain 제거 후 상태 없음·worker/process join·task0·weak owner 회수를 확인했습니다. 사용자 앱/process에는 접근하지 않았습니다.
- [x] `clippy --lib --bin taide-native-app --tests -- -D warnings` exit0·16.29초(handle64713). 기존 Wry dependency17 warnings는 authored strict 결과와 구분합니다. authored5 exact rustfmt 완료, tracked whitespace exit0·신규 Rust5 no-index whitespace 출력 없음(exit1은 신규 diff)입니다.

Cargo는 직렬/locked/offline/기존 target만 사용했습니다. 이전 recovery policy와 assets/startup 검증 성공은 재사용했습니다. 이 slice에서 새 의존성·manifest/lock·root/Tauri·MSRV·제품 TS·Git 변경은 없습니다. 보호 bundle·사용자 home·OS/IME/VoiceOver·키 파일에도 접근하지 않았습니다.

## API와 아이콘 근거

설치된 공식 egui0.36.2의 Label·Ui.horizontal·allocate_exact_size·Painter.circle_stroke·WidgetInfo·Context.run_ui와 epaint TexturesDelta 문서를 읽었습니다. 종료된 watch receiver는 기존 Tokio watch 계약으로 판단합니다. 공식 Lucide [circle-check SVG](https://github.com/lucide-icons/lucide/blob/main/icons/circle-check.svg)와 [circle-x SVG](https://github.com/lucide-icons/lucide/blob/main/icons/circle-x.svg)의 공개 원형/경로를 확인했습니다. 기존 LICENSE-LUCIDE의 ISC/MIT 고지를 유지합니다. 로컬 pinned lucide-react 원문과 요청한 CDN 버전 경로는 사용할 수 없었으므로 현재 공식 SVG를 기준으로 했으며 pinned 원본 binary의 픽셀 동등성을 주장하지 않습니다.

## 미완료

- [ ] 상태바의 Problems·IDE·시스템 사용량·cursor·chord·font stepper 등 다른 부분과 전체 status bar/AppSurfaces 비교는 별도 미완료입니다.
- [ ] 실제 GUI/다중 창·좁은 창 배치·OS 접근성·원본 아이콘 anti-alias/pixel 비교·대규모 서버 집계/재페인트 비용은 최종 화면·성능 gate에서 확인합니다.
- [ ] 원본 same-generation handshake3회/2초·dispose/reacquire·multi-root/shares_sessions·진단/provider/log/progress 전체 parity는 남습니다. 전체 snapshot 경합 RED와 actor ABA 경합도 미실행이며 session 수명 통합 gate에서 확인합니다.
- [ ] actual App assets/effects/ports owner·Settings/AppFile/IDE 화면·Rust remote UI·keybinding RED/PTY remount·N1~N8 0/8·최종 TS 제거/배포는 미완료입니다. M8 전체 완료 전에 commit/push하지 않습니다.
