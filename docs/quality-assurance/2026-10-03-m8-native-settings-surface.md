# M8 native Settings 기본 화면 연결

## 대상 파일

- `native/taide-native-app/src/settings-view.rs`, `settings-view-tests.rs`
- `native/taide-native-app/src/application.rs`, `host.rs`, `presentation-refresh.rs`, `lib.rs`
- 선행 저장 경계: [settings-controls QA](2026-10-03-m8-native-settings-controls.md)

## 리포트

Settings 탭의 제목·미연결 status fallback을 기본 실제 화면으로 교체했습니다. 현재 연결 범위는 4개 card의 테마/언어 선택과 기존20 switch·9알림 위치·2숫자 입력입니다. 원본4section 전체, 13개 section, takeover, 전체 Settings/M8 완료가 아닙니다.

Settings 열기 rail intent는 현재 창의 선택 프로젝트/pane을 명시해 공통 `layout_open_tab`에 전달합니다. main과 auxiliary target을 구분하며 `Settings`, preview=false, 원본 번역 제목을 사용합니다. 중복 탭은 기존 layout dedup 규칙을 그대로 사용합니다. 이 마지막 미연결 ShellIntent가 구현돼 unreachable fallback을 제거했습니다.

## 실제 구현 경계

원본 근거는 `settings-view.tsx`, `settings-{appearance,language,interface,notification}-section.tsx`, `ThemePicker`, `LanguagePicker`, `SettingsToc`, `SwitchField`, `NumericField`, `ToastPositionPicker`와 Card/Button/Switch 스타일입니다. 고정 API는 installed egui0.36.2 source 및 공식 [ScrollArea 문서](https://docs.rs/egui/0.36.2/egui/containers/scroll_area/struct.ScrollArea.html)·[Ui 문서](https://docs.rs/egui/0.36.2/egui/struct.Ui.html)를 확인했습니다. ComboBox 문서 조회는 실패했고 private 함수는 사용하지 않습니다.

1. 테마·언어 목록은64-capacity host에서 supervised blocking worker로 읽습니다. project/pane/tab/mount request를 캡처하고 실제 활성 Settings 탭이 아닐 때 worker admission/앱 응답을 거절합니다. 두 목록의 결과는 독립적입니다. 같은 mount의 중복 제출·완료, 숨김/닫힘 뒤 응답, 새 mount의 오래된 응답을 구분합니다. 실패는 attempted로 유지해 프레임마다 재요청하지 않습니다. 향후 theme editor 저장·삭제의 목록 invalidation/수동 retry 정책은 미완료입니다.
2. page padding16/32, 목차192/행 간격2, body gap32, card 간격24·padding20·본문 간격12와32px 목차 이동을 사용합니다. 목차 active는 클릭만 바꾸며 자동 intersection 갱신은 만들지 않았습니다. viewport 절반의 tail space와 기본 smooth scroll을 유지합니다. 현재 목차는 구현한 기본4개만 존재하며 나머지 빈 card/작동하지 않는 목차를 완료 증거로 만들지 않습니다.
3. 테마는 TAIDE 기본2개/나머지 bundled/custom을 원본 순서로 구분하며 window viewport640 기준2/3열·선택 표시·theme id 저장을 연결합니다. 언어는 system 첫 항목·알 수 없는 현재 id의 system 표시·192높이 popup·ArrowDown 열기/재열기·Up/Down·Enter 선택/닫기와 trigger focus를 연결합니다. 저장된 설정과 locale/theme hot refresh는 별도 로컬 설정 복사본으로 대체하지 않습니다.
4. switch와 position에는 번역 label·선택 상태의 widget info를 연결합니다. 숫자는 원본 저장값 변경에 keyed TextEdit와 draft sync를 적용하고 Enter 단독으로 blur하지 않으며 실제 blur에서만 검증/commit/snapback합니다. 양쪽 control을 행 오른쪽으로 배치합니다. UI 검증 AppError는 원본을 유지한 채 앱의 `settings_failed` toast로 전달합니다. themes/locales 폴더는 닫힌 AppDataPathKind와 기존 `system_open_app_data_path`를 연결했으나 실제 OS 폴더 열기는 실행하지 않았습니다.
5. 새 appearance는 공통 hot-refresh aggregate에 포함돼 다른 appearance와 함께 검사 후 적용됩니다. 기존 shared/runtime DTO·settings storage/event·locale/theme service를 재사용했고 dependency/manifest/lock/MSRV·제품 TS·vendor·보호 앱 bundle·OS 설정은 변경하지 않았습니다.

## 검증 결과

모든 Cargo 명령은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target` 환경에서 직렬 실행했습니다. 아래6개는 서로 다른 검사이며 같은 코드·입력의 성공은 반복하지 않았습니다.

- [x] `cargo test … --lib native_settings_ -- --nocapture` 최초 신규 목록/화면2건 PASS, suite0.10초/컴파일6.31초. 목록 실제 HostBridge·TaskSupervisor·합성 디스크 builtin 조회, 잘못된 프로젝트, duplicate/hidden/remount/late 응답, 독립 실패와 shutdown 회수를 확인했습니다.
- [x] `cargo test … --lib native_settings_open_host -- --nocapture` 1 PASS, suite0.02초/컴파일0.21초. 실제 main pane/동일 pane 중복/aux pane Settings 생성과 invalid pane의 layout 불변을 확인했습니다. 실제 OS 보조 창을 실행한 검사가 아닙니다.
- [x] `cargo test … --lib native_settings_ -- --nocapture --skip native_settings_catalog --skip native_settings_view는 --skip native_settings_open_host` 좌표·행 폭 정정 뒤 numeric/position1 PASS, language1 FAIL, suite0.07초/컴파일4.13초. numeric의999→8 commit/저장값 snapback과 bottom-right 단일 Change가 성공했으며 해당 성공은 재사용합니다.
- [x] `cargo test … --lib native_settings_language_keyboard -- --nocapture` 입력 처리 정정 뒤1 PASS, suite0.07초/컴파일3.27초. 실제 mouse toggle·ArrowDown 재열기·다음 항목·Enter의 language id/닫힘을 확인했습니다.
- [x] `cargo test … --lib native_settings_palette -- --nocapture` 1 PASS, suite0.06초/컴파일3.28초. 모든 builtin/bundled 테마의 새 Settings appearance 토큰을 해석했습니다.
- [x] `cargo test … --lib native_settings_view는 -- --nocapture` 행 폭·진단 glyph helper 변경의 영향1 PASS, suite0.10초/컴파일1.84초. 원본 테마 클릭/목차 이동/ShowSystemUsage 단일 변경, 열린 mount의 한국어·light theme 교체와 active section 유지입니다. 초기 같은 검사 PASS 뒤 실제 해당 경계를 수정했으므로 이 영향 검사만 재실행했습니다.
- [x] `cargo clippy … --lib --bins --tests -- -D warnings`: 최종 exit0,12.48초. inherited Wry17 warning은 기존 dependency 출력이며 authored 경고/억제는 없습니다.
- [x] authored6파일 `rustfmt --check --edition 2024 --config skip_children=true` exit0. 최신 PROCESS/HANDOFF/재개 프롬프트/QA/bug5개 문서의 Prettier exit0이며 tracked PROCESS/HANDOFF `git diff --check` exit0입니다. untracked authored6개·재개 프롬프트/QA/bug3개 `git diff --no-index --check /dev/null <file>`는 전부 출력 없음/exit1로 내용 차이만 있으며 whitespace 오류가 없습니다.

선행 Settings 저장 모델/host·keybinding/font 저장 및 toast의 성공은 관련 경로가 바뀌지 않아 재실행하지 않았습니다. 전체 suite는 실행하지 않았으며 별개 keybinding Tab RED를 성공으로 덮지 않았습니다.

## 실패와 원인·정정

- 최초 compile 실패: `SettingsFailed`가 tuple AppError인데 result 필드 variant로 작성한 곳을 기존 `result.err().map` 계약으로 수정했습니다. 다음 library check는 exit0/2.23초였습니다.
- fixture compile 실패: 존재하지 않는 NoopEventSink import를 제거하고 합성 EventSink를 정의했습니다. ComboBox의 private popup-id 함수를 사용할 수 없어 공개 Popup API와 원본 combobox 동작을 직접 구현했습니다. 검사 억제·vendor 변경·private API 추측 우회는 없습니다.
- 첫 strict는 fixture frame helper8개 인자만 실패했습니다. event/time을 실제 raw input 묶음으로 합쳐7개 인자로 수정했습니다. 마지막 ShellIntent 구현 후 생긴 unreachable fallback과 fixture 대문자 함수 이름/Debug-only dead-field 경고도 원인을 제거했습니다.
- language 재열기는 기준/수정2회 같은 지점에서 FAIL(suite각0.07초)하여 추측 수정을 중단하고 진단 선택 질문을 제시했습니다. 실패한 pointer focus/필터 수정을 제거한 뒤 test-only 위젯 ID·rect·focus·click·키 소비를 계측했습니다. stable coordinate가 준비되기 전 클릭하는 fixture 문제와, egui raw clickable trigger가 browser 버튼처럼 자동 focus를 주지 않는 제품 경계를 구분했습니다. 공개 Memory source에서 이전 프레임의 arrow filter와 기본 Enter 소비를 확인한 뒤 pointer focus·vertical arrow filter·열린 popup Enter의 선행 소비를 함께 연결했습니다. 최종 관련 단일 검사가 통과했습니다. 이 계측은 합성 headless 입력뿐이며 별도 OS 권한을 사용하지 않았습니다.
- position 최초 두 FAIL은 이전 drawing의 클릭 좌표를 재사용한 fixture였습니다. 다음 drawing을 사용하자 position이 통과했고 numeric blur만 FAIL했습니다. 독립 numeric 실패 후 계측에서는 숫자 text rect가[[439.1,593.4]–[459.1,607.4]]인데 실제 입력 rect는[[363.1,243.4]–[443.1,261.4]], focus=false였습니다. ScrollArea가 content rect 구성 뒤 offset을 갱신하는 source와 우측 정렬 galley의 음수 local origin을 확인했습니다. 시간 점프 뒤 안정화 프레임을 그린 뒤 입력하고 helper는 `galley.rect.translate(text.pos)`로 실제 glyph를 계산합니다. 이는 반복 계측이 아니라 검사 준비이며 animation을 끄지 않았습니다. 숫자/switch label의 사용 폭만 예약하던 제품 배치도 min width를 명시해 오른쪽 정렬을 복원했습니다. 자세한 내용은 [입력 geometry 기록](../bug/2026-10-03-native-settings-control-hit-geometry.md)입니다.

## 미완료 게이트

- [ ] Settings header의 settings.json AppFile 실제 editor/save, theme create/duplicate/edit/custom 목록/takeover, agent CLI/hooks/project 목록·IDE start/stop, native notification permission/test/system settings. 현재 실제 연결하지 않은 CTA/integration을 작동한다고 표시하지 않습니다.
- [ ] 남은9개 section 및 조건부 performance, snippet/theme editor takeover, 목록 invalidation/retry와 모든 mutation 부수효과.
- [ ] 실제 App mount/close/hidden·멀티창 focus/keyboard/AX/IME/clipboard·모든 theme/locale pixel/반응형/transition·scroll timing·popup Tab/포커스 복원·집계 메모리/캐시 회수/성능. 현재2/3열·scroll·thumb/check/icon은 headless 기본 계약이지 원본 pixel 전체 일치 증거가 아닙니다.
- [ ] N1~N8 전체213view/41action/Monaco21/palette·cutover/TS제거/Rust99%·성능/보안/패키징/서명/notary/설치/rollback. M8 0/8 상태이며 전체 완료 전 commit/push하지 않습니다.
