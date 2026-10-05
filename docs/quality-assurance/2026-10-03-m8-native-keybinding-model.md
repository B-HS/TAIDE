# M8 native keybinding catalog·capture·Settings host 기본 경계

## 대상 파일

- `native/taide-native-app/src/keymap.rs`, `keybinding-catalog.rs`, `keybinding-capture.rs`, `keybinding-commands.json`
- `native/taide-native-app/src/host.rs`, `presentation.rs`
- `native/taide-native-app/tests/fixtures/keybinding-catalog.json`, `tools/keybinding-catalog/export.ts`, `tsconfig.json`

## 리포트

기존 키맵 편집 화면의 데이터와 입력·저장 기반을 구현했습니다. 화면 구현이나 `open-keybindings-editor` 제품 action 연결 완료는 아닙니다. 상위 N1~N8은 0/8이며 기존30 shell action+terminal 자체2 이동의 기본 연결 수는 변경하지 않습니다.

원본 bootstrap의 8개 등록 호출 순서와 정적 metadata 212개를 보존합니다. 공통41 keymap 중 별도 명령에 연결되지 않은 행까지 합쳐 macOS는218행, 비macOS는 CLI3개 제외215행입니다. disabled command도 원본처럼 행에 남습니다. `monaco.*` metadata와 기본 라벨은 표시·충돌 근거이며 Monaco action 실행이 구현됐다는 뜻이 아닙니다. native 런타임은 JSON snapshot만 읽으며 TS/Bun/Monaco를 실행하거나 참조하지 않습니다.

도구는 원본 TS 모듈을 실행하지 않고 설치된 TypeScript5.9.3의 AST·type guard로 whitelist metadata·상수·spread를 읽습니다. bootstrap 순서나 지원하지 않는 metadata/platform 조건은 명시적 실패입니다. [공식 Compiler API 문서](https://github.com/microsoft/TypeScript/wiki/Using-the-Compiler-API)의 SourceFile/AST 방식과 설치된 선언을 확인했습니다. 검증 fixture는 실제 원본 `buildKeybindingRows`, `parseKeymapOverrides`, conflict index와 captured-key filter의 결과입니다. 개발·검증 도구이며 제품 TS view를 추가하지 않았습니다.

## 구현 상세

1. 같은 Stage/Entry 파서와 rebind scope 규칙을 공유합니다. 첫 유효 override·legacy 별칭·malformed chord 제거·명시적 unbind·원본 등록 순서·keymap-only category·Monaco default label을 보존합니다. conflict index는 immutable 행 slice를 빌려 다른 시점의 행 배열을 잘못 넣는 위험을 없앴습니다. 원본 first-in-array·modifier platform collapse·서로 다른 when/chord second stage·빈 binding 거절을 유지합니다.
2. 원본 reset/merge처럼 해당 ID의 모든 override를 제거한 뒤 마지막에 새 binding을 붙입니다. 알 수 없는 command·미래 추가 필드의 유효 JSON은 유지합니다. empty list는 `[]`로 직렬화합니다. 행별 실제 key 검색은 원본처럼 미override Monaco default label까지 검색하지 않습니다. standalone command chord를 첫 stage에서 실행하지 않습니다.
3. capture는 첫 stage modifier 필수·둘째 bare key 허용·pending bare Enter 단일 확정·modified Enter chord·modifier-only 무시·Escape 취소·blur 뒤 검색키 보존·검색 toggle 초기화·Monaco F1~F12/bindable 제한을 보존합니다. physical fallback·named key 표기·플랫폼 mods 순서·shortcut label도 연결했습니다. capture repeat/IME의 실제 egui 전달과 글로벌 chord 초기화·modal 소유권은 아직 미연결입니다.
4. typed `HostCommand::SetKeymapOverrides(Overrides)`가 기존 runtime SettingsPatch 저장과 SettingsChanged 이벤트를 재사용합니다. keymap 필드만 수정하므로 editor/terminal/integration 설정은 그대로입니다. 전용 no-op integration callback은 enable flags를 수정하지 않는 이 경계에만 적용되며 일반 Settings callback 구현을 대체하지 않습니다. 실패는 기존 HostReply::Failed로 반환하고 live Settings·이벤트는 유지합니다.

## 검증 결과

- [x] `bun tools/keybinding-catalog/export.ts --check`:212개 metadata와 원본8개 등록 순서 PASS. `--check-fixture`:실제 원본의 전체6개 catalog 시나리오 PASS. malformed chord/legacy·중복 first override·shared prefix/when·Monaco default unbind·standalone chord·nonmac ctrl/mod·space·유효 미래 필드를 포함합니다.
- [x] app `cargo test ... --lib keybinding_catalog -- --nocapture`:1 PASS, compile4.24초/suite0.05초. 전체 행의 필드/순서·직렬화된 override·첫 conflict·unassigned/filter 목록을 원본 fixture와 대조하고 모든41개 Core entry의 현재 binding/when과 일치시켰습니다. reset/merge·unknown field·invalid boundary·standalone dispatch·default label 실패 경계도 같은 검사에서 확인했습니다.
- [x] app `--lib keybinding_capture`:1 PASS, compile5.26초/suite0.00초. 별도의 새 capture 위험만 검사했으며 전체 catalog 성공을 다시 실행하지 않았습니다.
- [x] app `--lib keybinding_host`:1 PASS, compile2.66초/suite0.05초. 실제 합성 Settings 파일·전체 Settings equality·assign/unbind/reset/empty·SettingsChanged·쓰기 실패 뒤 live/event 불변·host disconnect/TaskSupervisor0입니다. 실제 clipboard writer/reader는 panic port이며 사용자 데이터/보호 앱/OS 설정을 사용하지 않았습니다.
- [x] 최종 app lib strict Clippy exit0(1.91초)·authored5개 Rust exact rustfmt/추적diff exit0, 생성 도구 최종 strict TypeScript exit0·Prettier exit0. 초기 TS Map/flatMap null tuple 추론 오류는 타입 경계로 수정했습니다. Wry의 기존17개 vendor warnings와 구분합니다. 동일 상태 성공 검사는 재사용했습니다.

Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, app manifest·`--locked --offline --target-dir experiments/native-shell-spike/target`이며 Cargo는 직렬입니다. 신규 dependency·manifest/lock·root MSRV 변경은 없습니다.

## 미완료·실행 조건

- [ ] 실제 dialog layout·localized label·assigned-first localeCompare·NFC/8-token fuzzy rank·검색/필터 UI·빈 목록·count·active context500ms frozen snapshot·scroll/AX/키보드 focus를 연결합니다. Rust std만으로 원본 Unicode normalization/locale collation을 가정하지 않습니다. 설치 ICU normalizer2.2.0과 objc2 NSString 소스 위치만 조사했으며 의존성 추가나 대체 정렬은 하지 않았습니다.
- [ ] 원본 default editor deferral을 보존하는 open action·글로벌 capture/chord 초기화·같은 frame event owner·popup/modal/Zen 보호·close/reset·blur/IME/반복·pending single 확인 클릭을 실제 egui/App에 연결합니다.
- [ ] 충돌 warning 후에도 저장하는 원본 정책·다른 행 unbind·Settings async 갱신/실패/빠른 연속 변경·다중 창/aux·실제 픽셀·OS 입력을 검증합니다. 현재는 model과 독립 실제 host만 확인했습니다.
- [ ] 모든 명령 실행·전체 Monaco/editor/keymap/팔레트 상태·원본213view·N1~N8/성능/보안/배포/cutover/제품 TS 제거를 완료합니다. remount A/B는 계속 응답 대기이며 임의 정책 변경이나 parser replay는 추가하지 않았습니다. 전체 M8 완료 뒤만 commit/push합니다.

유효하지 않은 `actionId`/key 타입 또는 비배열 mods는 native 경계에서 거절합니다. 원본 shape-only 검사의 후속 UI crash를 재현하지 않으며, malformed chord만 있는 유효 첫 stage는 원본처럼 유지합니다. 잘못된 JSON 전체는 기본 설정으로 해석합니다. 이 자료는 화면·입력 실기·전체 제품 완성 증거가 아닙니다.
