# M8 원격 파일 배너·디스크 선택

## 결과와 범위

같은 공용 conflict_banner·원본 theme/locale로 실제 FileViews surface의 conflict/read-only/loading/error 표시와 선택 사건을 연결했습니다. BrowserEditor의 choose_disk_after_dirty_flush는 이미 실제 dirty 전송을 끝낸 caller의 명시 계약이며 새 file_open 응답과 선택 당시 revision을 같은 EditorStore::choose_disk에 전달합니다. 전체 dirty/mirror/persistence 소유자를 구현했다고 세지 않습니다.

신규 pure4·native appearance 이동 영향1·새 Chrome-Wasm disk-choice1이 각각 한 번 PASS했습니다. 무효화 시험 DTO 오류는 실패 검사만 정정·재실행했고 저장 후 복원 배너 잔존은 실제 RED→수정→GREEN입니다. 이전 transport/presentation/files/indent/save 성공은 재사용했습니다. 후속47 2/4(50%)·전체363/433(83.83%, 비가중 체크리스트)·최종0/8·전체 ETA 산정 보류·goal active·전체 완료 전 Git 없음입니다. main 직접·process/verify/save-docs 적용입니다.

## 대상과 원본 근거

- `native/taide-native-ui/src/presentation.rs`, `native/taide-native-app/src/presentation-refresh.rs`, `native/taide-remote-web/src/presentation.rs`: 원래 native BannerAppearance의 statusIndicator.error/warning 두 색상 호출을 공용 factory로 이동했습니다. browser도 실제 resolved theme를 사용하며 누락·잘못된 색상은 오류로 유지합니다.
- `native/taide-remote-web/src/files.rs`: 실제 shared NativeEditor 위에 기존 배너를 그립니다. loading은 실제 editor 배경만 그리며 가짜 문서를 만들지 않습니다. 실패는 locale의 editor.openFailed를 그린 뒤 원래 failure를 반환합니다. readonly는 실제 lossy/large 메시지를 구분합니다. 실제 버튼 입력은 captured ChoiceRequested를 발생시키며 단순 action 수신을 처리 완료로 세지 않습니다.
- 같은 FileViews의 ChoiceRequest는 path/view/document/revision/choice를 보존합니다. pending 선택 중에는 UI/저장·해당 문서의 일반 조회를 제한하고 이전 일반 read 소유권을 버립니다. 새 typed file_open만 core에 적용하며 view 해제·revision 변경·fs invalidation·malformed/remote/Closed 오류는 ChoiceFinished 실패로 공개합니다. 선택을 재연결 후 자동 재실행하지 않습니다.
- `native/taide-remote-web/src/browser-editor.rs`: loaded Settings/actual locale/theme/파일별 indent를 쓰는 show_surface와 선택 이후의 실제 Workbench invoke를 연결합니다. 원래 core KeepMine은 draft/undo·line ending을 유지하며 새 디스크 baseline을 채택하고 ViewDisk는 두 view의 같은 문서를 갱신·undo를 초기화합니다. 성공한 저장·선택은 같은 문서의 복원 notice를 해제합니다.
- `tests/files.rs`, test-only FileProbe/files.html/tool disk-choice: actual pointer/renderer와 실제 BrowserEditor/WebSocket 경계를 확인합니다. probe는 실제 layout_set_dirty의 null 응답을 받은 뒤에만 choose_disk_after_dirty_flush를 호출합니다. 시험용 단일 tab caller는 전체 제품 dirty/mirror 소유자의 대체가 아닙니다.

원본 native application.rs의 flush_dirty/choose_disk·Host ChooseDisk/commit_disk_choice·SaveFinished와 TS use-editor-file-persistence를 읽었습니다. flush_dirty는 미러 저장이 아니라 탭 dirty 전송이며 원격 이름은 실제 remote-layout의 layout_set_dirty입니다. 기존 shared conflict_banner/core choose_disk와 egui run_ui/add_enabled_ui API를 실제 소스에서 확인했습니다. 라이브러리/버전/manifest/lock/MSRV·제품 TS·vendor·OS 설정/사용자 파일/보호 앱은 바꾸지 않았습니다.

## 실행한 최소 검사

공통 실행 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, cargo 절대 경로는 같은 디렉터리의 `bin/cargo`, 모든 Cargo 명령은 `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`입니다. Cargo는 직렬 실행했습니다.

- [x] remote-web `cargo test --test files 원격_disk_choice`: 새2 중 최신 조회/공유 문서/KeepMine/ViewDisk1은 첫 실행 PASS(build1.45초/suite0.00초)입니다. 무효화1은 fixture kind를 modify로 잘못 써 actual enum decode가 거절되어 실패했습니다. 실제 taide_model::file::FsChangeKind::Modified DTO로 정정한 실패1만 exact 재실행해 PASS(build0.32초/suite0.00초)입니다. 성공1은 반복하지 않았습니다. binary/remote error·late edit/invalidated read·unbind/Closed/no replay/dirty 보존도 같은 새 검사입니다.
- [x] remote-web `cargo test --test files 원격_file_surface는_실제_배너_click_잠금_loading_readonly_오류를_그린다 -- --exact`: 새1 첫 실행 PASS(build0.04초/suite0.02초)입니다. 실제 Text shape/공용 색상·배너 PointerButton down/up→ChoiceRequested·pending 입력 잠금·성공 후 배너 해제·lossy/large readonly/오류·loading 문서0을 확인했습니다. 캔버스 픽셀/GUI 검증은 아닙니다.
- [x] remote-web `cargo test --test files 원격_restore_notice는_성공한_저장뒤_같은_문서의_모든_view에서_해제된다 -- --exact`: 새1 RED([true,true], 기대[false,false], build0.35초/suite0.03초)를 저장/선택의 공용 clear_notices로 수정했습니다. 실패한 동일 검사만 GREEN(build0.60초/suite0.03초)이며 기존 native SaveFinished의 동작을 보존합니다. 시험 unused_mut/unused Result warning은 실행 결과를 검사하도록 정정했으며 억제하지 않았습니다.
- [x] native-app `cargo test --lib presentation_refresh::tests::native_presentation_refresh는_정본_appearance와_설정시스템경계를_보존한다 -- --exact`: 이동 영향1 PASS(build10.66초/suite0.06초)입니다. 실제 builtin theme/Appearances constructor/Settings·system 경계를 확인했습니다. 기존 Wry17/linker __eh_frame 경고는 유지했습니다.
- [x] 새 probe Wasm build1.52초, 저장 배너 수정 뒤 영향 build0.85초·probe Wasm clippy0.42초·remote-web native lib/files clippy0.73초(`-D warnings`) exit0입니다. probe dependency graph를 바꾸지 않았습니다. tool strict TypeScript와 Rust7 exactfmt/TS·HTML Prettier/관련 tracked diff 및 새10파일 whitespace 검사 exit0입니다. 성공한 이전 검사는 반복하지 않았습니다.

첫 apply_patch는 rustfmt가 만든 줄바꿈과 안 맞아 적용 전 거절됐으며 실제 줄을 읽고 정정했습니다. 이 도구 오류를 제품 RED로 세지 않습니다. 선행 production Wasm strict0.33초는 선택/배너 최초 코드의 결과이며 마지막 저장 배너 수정 후의 strict 결과와 혼동하지 않습니다. 마지막 코드는 실제 Wasm runtime/build 및 native lib/files strict로 확인했습니다.

## 새 실제 Chrome-Wasm 결과

`bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built disk-choice`는 첫 실행 PASS했습니다. 승인된 합성 localhost/fresh headless Chrome/mock keychain·SW block/downloads off이며 보호 앱/사용자 홈·OS·입력기·키체인은 조작하지 않았습니다.

결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/choice-file-result.json`입니다. continuous와 prepared-config는 다시 실행하지 않았고 이전 두 result 파일은 보존했습니다. 새 source로 같은 file-built의 Wasm/binding을 재생성했으므로 이전 result와 최신 artifact의 경계를 구분합니다. CLI는 기존 공식0.2.129 그대로입니다.

초기 document1/view2·disk clean, 첫 edit 후 external1을 관찰한 conflict 배너와 first disk draft를 실제 Rust Text shape로 확인했습니다. dirty=true 전송 null 응답 뒤 fresh file_open→KeepMine은 first disk/dirty=true를 보존하고 conflict·배너를 해제했습니다. external2 이후 같은 전송/조회→ViewDisk는 external2/dirty=false·conflict=false·배너 해제입니다. dirty 전송2/read5/save0·seq1~13 고유·오류/응답 누출0입니다. dispose 후 actual poll에서 connected=false/socket0·1.1초 wake14/요청/추가 연결 불변·Drop/pageerror0입니다. fixture는 현재 단일 tab을 두 pane에서 사용하는 경우이며 전체 layout consumer/dirty batching·조건부 mirror cleanup까지 완료한 실측이 아닙니다.

## 남은 제품 계약

- [ ] 실제 layout/project dirty 소유자·pending dirty flush·mirror restore/compare-before-clear/persistence·auto-save/LSP format/code actions/Git 효과를 App에 연결합니다. 이 경계의 ChoiceRequested/ChoiceFinished는 그 소비자가 사용할 실제 계약이며 no-op 소비자를 넣지 않았습니다.
- [ ] 감지된 indent/manual model 변경 유지·화면 tab-stop·모든 browser App/canvas/surfaces/폰트와 제품 Rust bundle를 완성합니다.
- [ ] 전체 handoff/failed-close/GUI/성능/beta/cutover/Rust99%·최종 N1~N8을 검증합니다. CJK/VoiceOver 시스템 실기는 사용자-last로 유지합니다.

이 세부 경계만으로 후속47/전체 M8을 체크하지 않습니다. live command/browser/server handle은 검사 종료로 모두 회수됐습니다.
