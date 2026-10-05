# M8 native UI 글꼴 — 후속46 부분 구현

## 대상 파일·리포트

`native/taide-native-app/src/ui-fonts.rs`, `terminal_fonts.rs`, `problems.rs`, `lib.rs`, `Cargo.toml`, `Cargo.lock`이 대상입니다. 실제 bootstrap/terminal Loader가 설치하는 `FontDefinitions`에 UI regular/medium을 함께 조립했습니다. 터미널 글꼴 변경의 전체 `set_fonts` 교체가 UI 글꼴을 잃지 않게 했으며 기존 파일 검증·메모리 상한을 재사용합니다.

원본은 `src/shared/styles/global.css`의 UI stack과 Problems의 medium/tabular 계약입니다. [WebKit 시스템 글꼴 문서](https://webkit.org/blog/3709/using-the-system-font-in-web-content/)가 private period-prefixed family 이름을 보장하지 않으므로 `.SF NS` 하드코딩을 기각했습니다. [Apple 글꼴 문서](https://developer.apple.com/fonts/)와 설치된 epaint0.36.2/objc2 AppKit·CoreText0.3.2/CF0.3.2/fontdb0.24.0/skrifa0.44.0 소스를 확인했습니다. SF 기본 숫자는 proportional이며 medium 연결만으로 tabular parity를 주장하지 않습니다.

## 구현·검증 체크리스트

- [x] macOS `NSFont::systemFontOfSize_weight`가 선택한 regular/medium을 typed CTFont 경유로 읽습니다. URL을 CFURL로 검증해 경로로 변환하고 실제 head table54바이트와 fontdb의 file/collection face를 대조합니다. private family 이름이나 collection index를 가정하지 않습니다.
- [x] 실제 OS variation dictionary의 각 key/value를 CFNumber로 검증하고 tag·finite value·실제 axis 범위를 확인합니다. medium에 literal500을 강제하지 않습니다. 읽기 전용 실제 API 관찰값은 wght510/opsz17이었으며 synthetic bold를 쓰지 않았습니다.
- [x] 원본 fallback stack Segoe UI/Roboto/Helvetica Neue/Arial/Noto Sans KR와 fontdb SansSerif·egui fallback을 regular/medium별로 구성합니다. 실제 static500 face를 조회하며 variable fallback에는 wght 좌표를 전달합니다. 터미널 PostScript 지정은 기존의 정확한 face를 보존합니다.
- [x] 기존 안전한 `face_data`의 NOFOLLOW/NONBLOCK/CLOEXEC·파일 종류/크기/성장·font/index 검증을 유지합니다. face64MiB/terminal+UI 공유128MiB 상한을 차례로 차감하며 미리 여러 font allocation을 일괄 보관하지 않습니다. 경고에 경로나 이름은 출력하지 않습니다.
- [x] Problems title·파일 그룹 이름에 medium family를 연결했습니다. 실제 headless Problems title의 family/12px·nonempty glyph mesh를 확인했습니다. 그룹 이름의 별도 renderer assertion이나 실제 GUI 픽셀 검증은 수행하지 않았습니다.
- [x] 합성 weight/대소문자/PostScript 선택과 공유 budget0 거절·기본 fallback, 기존 terminal binary/file/TTC/invalid data/index·loader cancel/late reply/task0 검사를 확인했습니다.
- [ ] `tnum` OpenType 숫자 기능입니다. pinned epaint shaping이 `ShapeOptions::new()`를 쓰고 TextFormat에 feature 선택 필드가 없어 아직 구현하지 않았습니다. mono 대체나 임의 숫자 폭 조정으로 완료 처리하지 않습니다.
- [ ] CJK/script cascade·전체 optical size/line metrics·Windows/Linux 실행·전체 UI/실제 GUI parity입니다. CSS500과 AppKit Medium의 모든 glyph/pixel 동등성도 별도 확인이 필요합니다.

## 의존성·실행 결과

기존 objc2-app-kit0.3.2의 NSFont/NSFontDescriptor/objc2-core-text feature와 objc2-core-foundation0.3.2의 CFURL feature를 켰습니다. 추가 direct dependency는 같은 objc2 계열의 objc2-core-text0.3.2 하나이며 typed CTFont API가 필요했습니다. native app lock은 이 package 하나를 추가했고 기존 package 버전/MSRV·root/Tauri lock은 이 작업에서 바꾸지 않았습니다.

공통 명령 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--manifest-path native/taide-native-app/Cargo.toml`, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. Cargo는 직렬 실행했습니다.

| 명령·범위                                                            | 실제 결과                                                                                                                                                                                                                                         |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 읽기 전용 Swift NSFont/CTFont metadata                               | 기본 module cache sandbox 거절 뒤 `/private/tmp`의 명시적 cache로 exit0입니다. 시스템 설정·앱을 조작하지 않았습니다.                                                                                                                              |
| `cargo check --lib --offline`                                        | 새 lock package 해석 후 CF API 이름/opaque Dictionary wrapper compile 실패입니다. 실제 pinned API에 맞춰 정정한 locked check는 exit0,4.42초입니다.                                                                                                |
| `cargo test … --lib ui_fonts::tests` 초기                            | 2 PASS,compile31.43초/suite2.16초입니다. 이후 caller/PS 증거를 확장한 결과는 다음 행이 정본입니다.                                                                                                                                                |
| `cargo test … --lib fonts::tests` 최종 UI 증거                       | 실제 Problems caller·PS 보존 포함 UI2 PASS,compile6.38초/suite2.19초입니다. 함께 실행된 terminal1은 합성 DB의 새 UI 경고를 무경고로 기대해 FAIL입니다. UI 성공은 재실행하지 않았습니다.                                                           |
| `cargo test … --lib terminal_fonts::tests` fixture 정정 후           | 1 PASS,compile5.93초/suite2.16초입니다. 앞선 invalid-name count3/기대1과 loader is_empty 실패는 같은 UI 경고 분리 누락이며 제품 오류가 아닙니다. 정확한 unavailable-system-UI 경고 문자열만 제외하며 다른 거절/상한 경고는 숨기지 않습니다.       |
| `cargo clippy … --lib --bin taide-native-app --tests -- -D warnings` | 새 UI 선택의 collapsible_if2곳을 let chain으로 정정한 뒤 exit0,17.62초입니다. 이후 terminal 테스트 helper의 slice 입력 변경은 해당 테스트 compile/PASS로 검증하며 생산 코드 strict 성공을 재사용합니다. Wry dependency의 기존17경고는 별도입니다. |

전체 M8 N1~N8 0/8·목표 active입니다. 보호 bundle·사용자 앱·OS 설정·IME/VoiceOver·clipboard/Keychain·제품 TS·Git은 변경하지 않았으며 전체 완료 전 commit/push는 하지 않았습니다. live Cargo handle은 없습니다.
