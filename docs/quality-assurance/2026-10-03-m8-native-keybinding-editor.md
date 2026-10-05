# M8 native keybindings 검색·modal 기본 연결

이 문서는 최초 기본 연결의 역사 스냅샷입니다. 이후 icon/input과 layout의 현재 상태는 `2026-10-03-m8-native-keybinding-icons-and-input.md`, `2026-10-03-m8-native-keybinding-layout.md`가 정본입니다. 아래 텍스트 버튼·popup/Tab·그림자 미완료는 최초 시점을 설명하며 후속 성공을 덮어쓰지 않습니다.

## 대상 파일

- `native/taide-native-app/src/keybinding-editor.rs`, `keybinding-search.rs`, `keybinding-capture.rs`, `keybinding-catalog.rs`
- `native/taide-native-app/src/application.rs`, `keymap.rs`, `terminal_surface.rs`, `shell_keymap.rs`, `lib.rs`
- `native/taide-native-ui/src/commands.rs`, app `Cargo.toml`·`Cargo.lock`
- `tools/keybinding-catalog/search-fixture.ts`, `tsconfig.json`, app `tests/fixtures/keybinding-search.json`

## 리포트

원본 검색 함수·전체 3개 언어 행 정렬 대조와 native modal의 기본 입력·저장 연결을 구현했습니다. 전체 keybindings 화면의 시각·접근성·상태 동등성 완료가 아닙니다. 상위 N1~N8은 0/8입니다. shell action의 기본 코드 연결은 기존30개에 열기1개를 추가한31개이며 terminal 자체 이동2개와 합쳐33개입니다. 기본 Editor의 Cmd+K Cmd+S deferral은 원본대로 유지되므로 이 숫자를 모든 scope의 실행·전체41개 완료율로 계산하지 않습니다.

## 구현 상세

1. 원본 NFC normalization, codepoint별 lowercase, UTF-16 index/길이·연속 보너스·JS whitespace·최대8 token·중복 index 합집합·score/path/길이/collation stable sort를 구현했습니다. assigned-first 정렬과 전체 en/ko/ja label은 실제 TS 함수를 실행해 얻은 fixture와 대조했습니다. fixture의 기본 Intl locale은 en-US이며 언어별 번역 locale과 collation locale을 혼동하지 않습니다.
2. 실제 egui modal에 query·key search·conflict/unassigned count/filter·localized rows·scope/충돌 표시·capture·confirm/reset/unbind·빈 결과·scroll을 연결했습니다. settings가 갱신될 때만 현재 override/rows를 교체하며 저장은 기존 typed Settings host로 즉시 요청합니다. 성공 전 낙관적 적용은 하지 않습니다. 후보 충돌은 경고하되 저장을 막지 않습니다.
3. 캡처 중 Key/Text/Copy/Cut/Paste를 underlying view·일반 버튼 입력에서 분리하고 실제 capture button focus가 유지될 때만 적용합니다. 확인 버튼 pointer-down은 capture focus를 유지합니다. Escape는 capture만 먼저 취소하고 다음 Escape는 dialog를 닫습니다. 실제 0.36 Preedit의 빈 text/Commit과 현재 raw IME frame을 보호합니다. 이 검사는 OS 입력기 실기가 아닙니다.
4. App에서 `OpenKeybindings`를 처리하고 캡처 시작 시 해당 viewport의 pending/editor deferral/decision cache만 초기화합니다. 일반 query 중에는 원본처럼 글로벌 APP 단축키를 허용하며 modal 동안 underlying surfaces와 child WebView는 비활성화합니다. delete/dirty-close modal은 keybindings보다 나중에 그려 상위 confirmation을 유지합니다. 첫 lazy mount는 inspector가 비어 있고 닫힌 이후500ms 간격으로 editorTextFocus/terminalFocus를 관찰하며 open 동안 마지막 값을 동결합니다.
5. 보호 bundle·OS 설정·사용자 파일·clipboard는 사용하지 않았습니다. 실제 App 생성/OS 창·auxiliary viewport 전체 수명은 실행하지 않았습니다. reset/unbind/confirm 버튼은 현재 텍스트 버튼이므로 원본 아이콘·정확한 grid/tooltip/토스트 동등성 완료로 계산하지 않습니다.

## 의존성·공식 근거

Rust std와 기존 패키지에 원본 NFC·localeCompare를 대체할 portable API가 없어 ICU compiled data를 사용합니다. public registry가 확인한 최신 `icu_collator=2.3.1`, `icu_normalizer=2.3.0`, `icu_locale_core=2.3.0`을 app manifest/lock에만 추가했습니다. 세 package의 registry manifest는 MSRV1.88·Unicode-3.0이며 native1.95/root1.89는 변경하지 않았습니다. 최신 fetch 후 locked/offline library check exit0(12.55초)입니다. 초기2.2 compile은 호환 조사이며 최종 구현 검증을 중복 계산하지 않습니다.

설치된 공식 egui0.36.2 Modal/Frame/Context/TextEdit/Button/IME source와 ICU2.3 public source를 읽었습니다. source는 `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` 아래이며 새로운 API를 기억만으로 쓰지 않습니다. 제품 TS view는 추가하지 않았고 TS 도구는 원본 oracle 생성용입니다. 최종 bundle/SBOM·license notices·전체 memory/성능은 N6/N7 gate에 남습니다.

## 실제 검증

- [x] 원본 검색 fixture 생성 후 app `cargo test ... --lib keybinding_search`:1 PASS, compile19.70초/suite0.07초. raw Unicode·NFD·astral·lowercase·JS whitespace·8-token·전체3언어218행 label/order/검색을 대조했습니다. 테스트명 snake_case warning만 제거했고 성공 검사는 반복하지 않았습니다.
- [x] `bun tools/keybinding-catalog/search-fixture.ts --check`:actual original functions/3 catalogs PASS. 이 변경의 strict TS exit0(0.37초), 해당 tool/tsconfig Prettier exit0입니다.
- [x] app `--lib keybinding_editor -- --nocapture`:1 PASS, compile1.49초/suite0.56초. 실제 raw Text 검색·그려진 버튼 pointer click·capture focus·modifier 경고·single 확인/1회 save·reset·key search·Escape/IME/current enabled guard·close/filter reset·focus restore·lazy/frozen500ms context·no-project Cmd+K Cmd+S와 viewport chord 초기화·ko/ja title shape입니다. 실제 OS 픽셀·전체 App host 수명 검사로 확대 해석하지 않습니다.
- [x] app `--lib keybinding_theme`:1 PASS, compile2.89초/suite0.06초. runtime이 제공한 모든 builtin theme의 실제 resolved colors로 Appearance와 App의 statusIndicator error/warning keys를 확인했습니다. 임의 fixture 색상 추가는 하지 않았습니다.
- [x] app `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(14.02초). authored10개 Rust exact rustfmt exit0·추적diff whitespace exit0입니다. Wry의 inherited17 warnings는 authored strict와 구분합니다. cargo는 직렬, `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, app manifest·`--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 기존 catalog/capture/Settings host 성공은 재사용하고 다시 실행하지 않았습니다.

첫 compile의 ConflictIndex platform 인자·egui Context style/Preedit variant 오류를 설치 source로 수정했습니다. UI 검사의 status.warning 실패를 처음 raw fixture 상속 문제로 판단했으나 runtime resolved theme에서도 실패했습니다. 정본은 CSS가 매핑하는 statusIndicator.warning/error였으며 새 화면과 App 초기화4곳을 수정했습니다. 중간에 transitive crate를 직접 쓴 테스트 compile 오류도 기존 runtime facade로 제거했습니다. 실제 theme 결과에 맞춰 코드 키를 고쳤으며 fixture를 통과시키기 위한 색상 주입은 하지 않았습니다.

## 미완료와 다음 조건

- [ ] 원본 Lucide 아이콘·grid column/폭·small viewport·header 정렬·pills·tooltip·opacity/animation/shadow·scroll·실제 픽셀/AX를 재현합니다. 현재 텍스트 shape와 기본 bounds는 픽셀 동등성 증거가 아닙니다.
- [ ] 실제 충돌 warning/쓰기 실패를 원본 toast UI로 표시하고 빠른 연속 저장·late Settings event·다른 창 갱신/auxiliary 소유권을 확인합니다. 현재 warning은 기존 App status로 전달됩니다. 기존 host의 실제 저장·실패 검사는 model QA에 있고 새 UI test는 typed save output을 확인합니다.
- [ ] popup의 Escape 우선권, 같은 frame pointer→key 순서/blur·capture 변경·multi-pass exactly-once·focus trap/Tab wrap·actual WindowBlur/IME Unicode/OS·underlying App/widget 동작을 이어서 확인합니다. current App 입력 보호의 compile과 단일 raw modal 검사를 전체 수명 동등성으로 세지 않습니다.
- [ ] 나머지8개 공통 action·전체 Monaco21/팔레트/상태·AppFile/Save/raw Unicode·213 view·N1~N8/cutover/TS제거·성능/보안/배포를 완료합니다. remount A/B는 응답 대기이고 임의 변경하지 않았습니다. 전체 M8 완료 뒤만 commit/push합니다.
