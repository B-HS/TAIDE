# M8 파일별 들여쓰기와 공용 저장 준비

## 결과와 전체 경계

원본 파일별 들여쓰기의 명시 축·스타일별 크기 우선순위를 shared core/native formatter/native renderer/browser caller에 연결했습니다. 기존 네이티브 저장 준비 본문은 shared core로 그대로 이동하고 네이티브 경로는 re-export, BrowserEditor는 같은 cleanup→새 SaveSnapshot을 사용합니다. 위험별 검사5건(새 core1/shared Tab1/remote 준비1/native 이동 영향1/새 Chrome-Wasm1)이 각각 한 번 PASS입니다. 이전 성공 시나리오는 반복하지 않았습니다.

후속47 2/4(50%)·전체363/433(83.83%, 비가중 체크리스트·공수비 아님)·최종0/8·전체 ETA 산정 보류·goal active·전체 완료 전 Git 없음입니다. main 직접·검증/결과 저장 스킬을 적용했습니다. 들여쓰기 자동 감지·파일별 수동 변경 지속성·화면의 탭 폭, 전체 format/code actions/LSP·auto-save/mirror·배너·실제 App/canvas와 제품 bundle·handoff/failed-close/GUI/성능/beta/cutover/Rust99%는 미완료입니다. 명시적 override와 저장 준비만으로 전체 Monaco/파일 UI를 완료 처리하지 않습니다.

## 대상과 원본 근거

- `native/taide-native-editor/src/{indent.rs,save-preparation.rs,lib.rs}`: IndentOptions의 두 축과 resolve는 원본 editorconfig.ts 규칙을 공유합니다. tab 스타일은 tab_width→indent_size, space/미지정 스타일은 indent_size→tab_width이며 미지정 축은 caller의 현재 값을 유지합니다. .editorconfig 탐색/해석은 기존 서버의 resolved DTO이고 브라우저가 디스크를 읽지 않습니다. prepare는 기존 native App의 함수 본문과 동일하며 revision/key/read-only/File/dirty guard·cleanup·최종 snapshot을 유지합니다.
- `native/taide-native-ui/src/editor_surface.rs`: 기존 EditorAppearance를 복사한 with_indent만 추가해 원래 editor의 색상/font/focus/keymap와 다른 파일의 기본 indent를 바꾸지 않습니다. tab은 실제 탭 문자, spaces는 resolve된 칸 수만큼 입력됩니다. 화면 tab-stop/자동 감지는 아직 이 메서드로 구현했다고 세지 않습니다.
- `native/taide-native-app/src/{application.rs,save.rs}`: 실제 show_document의 keymap route는 보존하고 파일별 metadata/settings를 resolve해 shared editor를 호출합니다. 실제 LSP FormattingOptions도 같은 resolver를 사용해 기존 tab style 우선순위 차이를 제거했습니다. save::prepare/PreparedSave는 core의 re-export이며 기존 caller/test는 유지합니다.
- `native/taide-remote-web/src/{files.rs,browser-editor.rs}`: 파일 metadata로 actual shared editor의 들여쓰기를 정하고 loaded Settings의 cleanup flags·원본 per-file false override를 같은 core prepare에 전달합니다. 변경된 cleanup은 Edited event를 내며 하나의 in-flight save를 유지합니다. clean이면 저장을 보내지 않습니다. save_prepared는 실제 formatter/code actions가 끝난 후의 호출 계약이며 이를 조용히 생략한 완성 저장 UI가 아닙니다.
- 새 `tests/indent.rs`, shared editor_surface 신규1, files 신규1와 probe의 prepared-config 모드는 해당 실제 경계만 검사합니다. probe의 egui Text shape는 실제 Rust renderer 산출물이지 픽셀/canvas GUI 성공이 아닙니다.

원본 `src/shared/lib/{editorconfig.ts,editorconfig.test.ts}`, `src/features/editor/code-editor.tsx`, native save/App, core save_cleanup을 읽었습니다. [공식 EditorConfig](https://editorconfig.org/)의 미지정 속성·indent/tab-width·trim/final-newline 계약을 확인했습니다. docs.rs FormattingOptions 조회는 접근 오류였고 실제 고정 lsp-types0.97.0의 registry `src/formatting.rs`에서 필드/Serialize camelCase를 확인했습니다. 기존 서버 config 열수1~64, Settings tabSize1~8 정상화도 확인했으며 새 의존성/버전/manifest/lock/MSRV/시스템 설정은 변경하지 않았습니다.

## 단일 검증 증거

공통 cargo/CARGO_HOME/target은 file-views QA와 같습니다. Cargo 명령은 `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`이며 직렬 실행했습니다.

- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test indent`: 새1 PASS, build0.40초/suite0.00초입니다. 두 스타일의 두 property 경쟁·각 fallback·스타일만 지정/크기만 지정/빈 config·현재 축 보존을 확인했습니다.
- [x] `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface 파일별_들여쓰기는_실제_Tab_입력에_적용되고_다른_editor의_기본값을_바꾸지_않는다 -- --exact`: 새1 PASS, build4.62초/suite0.03초입니다. 실제 Event::Key Tab으로 두 칸 공백/탭 문자·다른 editor 기본값/font 불변을 확인했습니다. 시험 이름의 Tab non_snake_case warning은 tab으로 이름만 정정했고 실제 성공 검사는 반복하지 않았습니다.
- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test files 원격_저장_준비는_공유_cleanup과_editorconfig_false_및_clean_noop을_보존한다 -- --exact`: 새1 PASS, build1.21초/suite0.00초입니다. plaintext CRLF·공유 cleanup·명시 false/전역 true·실제 wire content·clean/in-flight no-op·Edited/SaveFinished/baseline을 확인했습니다. 선행 files3 성공은 재사용했습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib save::tests::save_keymap은_재지정_실제_editor와_등록문서_host_저장을_연결한다 -- --exact`: 이동 영향1 PASS, build19.39초/suite0.08초입니다. 실제 registered editor/keymap→같은 prepare→host 파일 저장/live·disk baseline/read-only 거절을 확인했습니다. prepare의 기존/이동 함수 전체를 sed 범위로 diff하여 exit0·본문 동일을 확인했습니다.
- [x] 생산용 Wasm lib strict0.80초, native App lib/bins/tests strict8.34초, 새 probe build1.75초 exit0입니다. 기존 Wry17/large __eh_frame warning은 끄지 않았습니다. exact Rust12파일 fmt·tool strict TypeScript·tool/HTML Prettier가 exit0입니다. patch 전사/import 위치/검사 범위 한 줄 오류는 도구 작성 과정이며 제품 RED라고 부르지 않았습니다.

새 native 들여쓰기/format 차이는 기존 소스와 원본의 직접 대조로 확인했습니다. 변경 전 실제 실패 테스트를 실행한 것은 아니므로 RED→GREEN 실측으로 주장하지 않습니다. 최소 새 골든/actual input/실제 browser 결과를 근거로 수정 검증을 기록합니다.

## 새 Chrome/Wasm prepared-config 실행

`bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built prepared-config`가 처음 실행에서 PASS했습니다. 승인된 synthetic localhost/fresh headless Chrome/mock keychain/SW block/downloads off이며 보호 앱·사용자 홈·OS·입력기·키체인·제품 TS는 건드리지 않았습니다. 실제 제품 core/UI/BrowserEditor가 구현 주체이고 서버 파일/settings 응답과 browser input Event만 합성입니다.

결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/prepared-file-result.json`입니다. 선행 file-result.json은 덮어쓰지 않았고 continuous 재연결 모드는 실행하지 않았습니다. 최신 시험용 Wasm/binding은 같은 file-built에 생성됐으므로 선행 결과와 source 버전 경계를 구분합니다.

초기 doc1/view2·`disk  \r\n` clean에서는 Save가 wire 호출0입니다. Settings는 4칸/cleanup true이고 파일은 2칸/cleanup false입니다. 실제 BrowserEditor.show_file→NativeEditor Tab 입력은 `  disk  \r\n`, 첫 save는 false override에 따라 이 내용을 그대로 기록하고 dirty=false입니다. 실제 file_open metadata refresh 뒤 cleanup true를 적용한 다음 Tab/save는 `    disk\r\n`이며 CRLF를 보존합니다. 성공 save2/read4·seq1~12 고유·오류/응답 누출0·conflict=false입니다. dispose 이후 poll을 기다려 connected=false/socket0·1.1초 wake13/요청/추가연결 불변·Drop/page error0을 확인했습니다. 이는 선행 결과의 오래된 disposed DOM 표본을 제품 장애로 재해석하거나 같은 연속 검사를 반복한 것이 아닙니다.

## 남은 원본 계약

- [ ] model 자동 들여쓰기 감지·수동 변경·화면 탭 폭과 native/browser 동일 경계를 완성합니다. 현재 resolver는 주어진 baseline의 미지정 축을 보존하지만 caller baseline은 아직 Settings이며 detected model을 제공했다고 가장하지 않습니다.
- [ ] 원격 format/code actions/LSP participant·auto-save timer·hot-exit mirror·dirty/Git 효과와 conflict/read-only/loading/error 배너를 실제 제품 caller에 연결합니다. prepare 내부 참여자 실패 정책은 기존 본문 그대로이며 전체 participant/오류 UI 완료와 구분합니다.
- [ ] actual browser App/canvas/surfaces/폰트·제품 자산/패키징·최종 M8 게이트를 완료합니다. 보호 spike 앱과 CJK/VoiceOver 사용자-last 계약을 유지합니다.

현재 live Cargo/browser/server handle은 없고 목표는 active입니다.
