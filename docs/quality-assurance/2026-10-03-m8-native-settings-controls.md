# M8 native Settings control·저장 경계

## 대상 파일

- `native/taide-native-app/src/{settings-controls.rs,host.rs,application.rs,lib.rs}`
- `native/taide-native-app/tests/settings-controls.rs`
- 원본 `src/widgets/settings-view/settings-{appearance,language,interface,notification}-section.tsx`, `src/features/settings/{switch-field,numeric-field,toast-position-picker}.tsx`, `src/shared/lib/numeric-field-commit.ts`, `src/entities/settings/settings.query.ts`
- 재사용 `crates/taide-runtime/src/settings_actions.rs`, `crates/taide-settings/src/service.rs`, `crates/taide-infra/src/root_guard.rs`

## 리포트

Settings 원본4section의 순수 policy/presentation 설정을 typed control로 연결했습니다.20개의 bool·9개의 고정 toast position·2개의 numeric과 theme/language 변경은 기존 runtime save/sanitize/event를 사용합니다. 기존 font/keymap host도 같은 저장 함수를 사용합니다. 아직 native Settings tab renderer·theme/language 목록 worker는 연결하지 않았으며, control/host 라이브러리를 화면 동등성이나 전체 M8 완료로 세지 않습니다.

## 상세

1. Switch는 실제 Settings/SettingsPatch에서 field를 읽고 해당 field 하나만 Some으로 만드는 closed enum입니다. 원본 message label/description과4section 분류를 보존합니다. agentHooksEnabled/ideIntegrationEnabled처럼 OS/integration 시작·종료가 필요한 field는 Change에 없습니다. 기존 no-op reconcile을 그 설정들에 적용하지 않으며 실제 native port를 연결한 뒤 활성화합니다. notification policy flag 저장은 실제 OS 알림 전달 검사가 아닙니다.
2. Position은 원본 row-major9개 값/label의 private static 값으로 임의 문자열을 만들지 않습니다. Numeric은 resizer0~~8/search debounce50~~2000 범위입니다. 상한과 field sanitize는 원본 service/TS 상수와 대조했습니다. raw NaN/동일 값은 mutation 없음, 범위 밖은 clamp, 내부 소수는 기존 u32 DTO의 거절과 같은 실패로 반환하며 조용히 truncate하지 않습니다. 이미 clamp된 유한 정수의 u32 변환만 수행합니다.
3. NumericDraft는 typing을 보존하되 저장값이 바뀌면 원본 key=value remount처럼 text를 초기화합니다. blur/commit은 현재 저장값으로 text를 먼저 되돌리고 새 Change를 내보내므로 실패한 값이 저장된 것처럼 남지 않습니다. [HTML number input 계약](<https://html.spec.whatwg.org/multipage/input.html#number-state-(type=number)>)과 [유효 floating-point 문법](https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#valid-floating-point-number)에 맞춰 optional minus·mantissa/fraction·signed exponent·ASCII digits·유한값을 검증합니다. leading plus/trailing dot·공백/Infinity/잘못된 exponent는 유효 input value로 인정하지 않습니다. 실제 브라우저의 localized numeric UI/step rounding·OS event 순서는 별도 gate입니다.
4. Host UpdateSettings는 원본 settings_update를 사용하고 Theme만 settings_set_theme로 보내 followSystemTheme=false와 SettingsChanged→ThemeChanged를 유지합니다. 저장 실패는 기존 SettingsFailed(error) 원본을 전달하고 App submit의 queue 실패도 settings toast 경로로 분류합니다. Settings/Theme 이벤트의 현재 hot appearance loader는 재사용합니다. App Settings surface가 아직 Change를 내보내지는 않습니다.
5. theme_exists는 기존 서비스에서 identifier를 안전 구성요소로 검사하지 않고 path를 합칩니다. native 새 host는 theme/language ID에 기존 root_guard::ensure_safe_component를 적용한 뒤 runtime action에 전달합니다. 원본 catalog의 정상 ID는 유지하고 traversal·empty/dot component는 InvalidArgument로 거절합니다. 기존 root public service 전체를 수정한 것은 아니며 전체 입력/보안 감사는 별도입니다. Secret/remote/integration 설정 전체를 generic JSON patch로 노출하지 않습니다.
6. 현재 queue는 기존64-capacity/serial host이며 source service의 owned mutation/save/event 경로를 사용합니다. 새 dependency·패키지 버전·Cargo lock/MSRV·제품 TS·vendor·보호 bundle·OS 설정을 변경하지 않았습니다. 새 UI control나 stream을 위해 무상한 channel을 추가하지 않습니다.

## 실제 검사

locked/offline native app manifest·공유 target의 serial Cargo와 자기 UUID의 임시 합성 Settings 파일만 사용했습니다. clipboard port는 호출 시 panic하도록 설정했으며 OS clipboard/알림/사용자 데이터는 사용하지 않았습니다.

- [x] 첫 `--test settings-controls`는 모듈 미구현 E0432 RED(exit101)와 테스트의 미설치 taide_settings 직접 참조 E0433 오류입니다. 새 의존성을 추가하지 않고 DTO 단일-field 검사를 현재 serde_json으로 변경했습니다. 실제 service 검사는 별도 HostBridge 왕복이 담당합니다.
- [x] Host fixture bootstrap 인자 순서 E0308을 실제 signature로 수정했습니다. 다음 batch 모델1 PASS/host1 FAIL(suite0.12초/compile1.18초)에서 root_guard가 원본 Localized AppError를 반환하는데 단순 InvalidArgument variant만 요구한 fixture 오류가 드러났습니다. 모델 PASS는 재사용했습니다.
- [x] 실제 AppError::kind를 비교하도록 fixture만 수정한 host1 PASS(suite0.14초/compile1.13초).20개 switch의 field/SettingsChanged·theme follow 해제·SettingsChanged→ThemeChanged·language/position·numeric backend sanitize·실제 디스크 equality·agent/IDE enable 보존·traversal/없는 theme/디스크 write 실패의 원본/state/event 불변·disconnect/shutdown을 덮습니다.
- [x] HTML number grammar를 추가한 뒤 영향 모델1 PASS(suite0.01초/compile4.28초). 단일-field DTO·9positions·NaN/동일/범위 clamp·fraction 거절·draft typing/sync/blur snapback·invalid string12개/유효 exponent5개를 덮습니다. host 저장 경로는 변경하지 않아 위 성공을 재사용합니다.
- [x] 공통 저장 함수의 기존 `--lib keybinding_host`1 PASS(suite0.07초/compile6.59초), `--lib font_keymap`1 PASS(suite0.04초/compile0.21초). 서로 다른 입력의 실제 디스크/실패·editor 영향 경계이며 같은 성공 명령을 반복한 것이 아닙니다.
- [x] lib/bin/test strict 최초 batch는 fixture의 MutexGuard explicit drop에도 await-holding-lock(exit101)이었습니다. lexical scope로 guard 수명을 고정한 뒤 최종 동일 관련 strict exit0(3.07초)입니다. 검사 억제나 async Mutex 도입은 없습니다. 마지막 제품 변경 이후 결과이며 순수 fixture lock scope 변경 전의 실제 host/model 성공은 재사용합니다.
- [x] authored5파일 exact rustfmt --check와 tracked whitespace exit0, untracked5파일 no-index whitespace 출력 없음(exit1은 내용 차이)입니다. 이후 lexical-scope fixture 변경의 exactfmt/whitespace도 출력 없음입니다. inherited Wry17 warning은 authored 검사와 구분합니다.

## 남은 gate

- [ ] 실제 native Settings tab admission/project owner와 renderer·목차/scroll·theme/language catalog worker·loading/error/late/닫힘/aux·blur 이벤트/IME/AX/키보드·모든 builtin locale/theme 픽셀을 연결합니다. 이번 code/host는 화면 구현 완료가 아닙니다.
- [ ] custom theme duplicate/edit/import/delete/takeover·locale folder·agent hooks/CLI·IDE integration·native test notification/system settings action을 실제 port에 연결합니다. OS 행동은 테스트에서 수행하지 않습니다.
- [ ] 나머지 Settings9개 기본 section·조건부 performance·snippet/theme takeover, 모든 Settings mutation·shared App/aux/원본213view의 기능·상태·시각/접근성 동등성이 남습니다.
- [ ] 별개 keybindings Tab RED·PTY remount 결정과 N1~N8 0/8·cutover/TS제거·성능/보안/배포/rollback을 유지합니다. 전체 M8 완료 후에만 commit/push합니다.

process/verify/save-docs 스킬을 기존 Settings 체크리스트 세분화·실패 영향만 재검사·정확한 미완료/분류 기록에 사용했습니다.
