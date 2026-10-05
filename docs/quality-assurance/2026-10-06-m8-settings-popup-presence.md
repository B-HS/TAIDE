# M8 Settings Popup Tab·Presence·motion

Snippets global 자동 focus의 후속 closing/outside-focus 오류는 `../bug/2026-10-06-settings-popup-external-focus.md` 및 `2026-10-06-m8-snippet-editor-and-producer.md`에 기록했습니다. slow250ms Presence deadline 이후에도 외부 focus를 trigger로 되돌리지 않으며 기존 owned-focus 성공은 재사용했습니다.

## 대상 파일

- `native/taide-native-ui/src/settings-code-view.rs`, `settings-view.rs`
- `native/taide-native-ui/src/tooltips.rs`, `tooltip-motion.rs`
- `native/taide-native-app/vendor/egui-input/src/containers/popup.rs`
- `native/taide-remote-web/tests/settings-resources.rs`, `tests/browser-probe/src/settings-probe.rs`
- `tools/m8-remote-rust-file-probe.ts`

## 리포트

기존 Editor·Terminal 화면 재현 체크 안에서 검색/옵션 Popup의 Tab 순환, 닫힘 Presence, 공용 CSS motion을 연결했습니다. 부모 화면 전체 완료와 구분하며 기존 완료 수를 올리지 않습니다.

## 원본 근거와 구현

원본은 `src/features/settings/font-picker.tsx`, `option-picker.tsx`, `src/shared/ui/popover.tsx`, `command.tsx`, `src/shared/styles/global.css`입니다. 설치된 공식 구현 `@radix-ui/react-popover`, `react-focus-scope`, `react-presence`의 `dist/index.mjs`와 `tw-animate-css/dist/tw-animate.css`를 확인했습니다.

1. non-modal FocusScope는 loop=true/trapFocus=false입니다. 검색 Popup의 유일한 tabbable은 입력이며, 옵션 Popup은 tabbable이 없어 Content Dialog 자체를 포커스합니다. cmdk 옵션은 tabindex 없는 div이므로 native 옵션도 `Sense::CLICK`으로 포커스 후보에서 제외했습니다. Dialog/input이 Tab 이벤트를 소유합니다.
2. 닫힌 Content는 150ms exit animation이 끝나기 전까지 남습니다. close state와 presence를 분리하고 닫힘 중 controls·입력·옵션을 유지합니다. 종료 후 명시 Escape/선택은 트리거로 복귀하며 outside interaction은 강제 복귀하지 않습니다. 실제 재열기는 남아 있는 state를 유지하도록 구현했으나 빠른 재열기의 독립 실측은 아래 미완료로 남깁니다.
3. 원본 ease/opacity/scale 0.95→1, 닫힘 1→0.95를 기존 Tooltip motion에서 재사용합니다. Popup translation은 0이며 sideOffset=4, 선택된 배치 pivot, `0 2px 8px app.shadow`를 적용합니다. 레이어 색상 전체를 같은 opacity로 조정하고 Area 기본 fade-in을 끕니다. 정상 종료/Drop에서 transform/dismissal 등록을 회수합니다.
4. 기존 SDK에 추가한 `Popup::fade_in(bool)`은 기본 true이므로 기존 caller 동작은 유지합니다. SDK의 missing_docs 계약 때문에 공개 API에 영어 doc 속성 한 줄만 추가했으며 내부 설명 주석·억제 지시·새 의존성은 없습니다.

## 실제 검증

- [x] 신규 portable 연속 검사 `원본_popup의_tab_loop와_닫힘_presence는_검색_및_옵션_포커스와_transform을_회수한다`: 최초 Tab이 옵션으로 이동하는 RED 뒤 옵션 focusability/입력 필터/Presence를 수정했습니다. 2.32초 build/0.12초 실행 PASS입니다. 중간 `crate::tooltip_motion` 경로 오인 compile 실패는 실제 `tooltips::motion` 공개 경계로 수정했습니다. 이번 inspection 추가 상태에서도 아래 suite의 같은 검사가 PASS했습니다.
- [x] 신규 pure `tooltips::motion::tests::popup은_같은_css_전이에서_위치이동없이_불투명도와_크기만_변경한다`: 3.05초 build/0.00초 실행 PASS입니다. 기존 Tooltip과 같은 네 시점의 opacity/scale/active 및 Popup translation=0을 확인합니다.
- [x] 변경 파일 suite 7건: 1.70초 build/0.16초 실행, 6 PASS/1 FAIL입니다. disabled→enabled가 자동 focus를 되찾는다는 기존 fixture 가정 때문에 End가 소유자에게 전달되지 않았습니다. 재활성화 후 실제 검색창 클릭으로 focus를 얻도록 fixture만 수정했고 실패한 정확한 검사 1건만 0.44초 build/0.12초 실행 PASS했습니다. 나머지 6건은 재실행하지 않았습니다. 두 종료 기대값은 원본 150ms 종료 뒤로 옮겼으며 IME 검사는 PASS했습니다.
- [x] 새 Chrome/Wasm `settings-popup-presence` 연속 검사 1 GREEN입니다. 최초 옵션 Dialog inspection의 early pass rect가 NaN/null이어서 Zod가 거절했습니다. inspection만 최종 frame/transform rect를 읽도록 수정해 실패 검사만 재실행했습니다. 검색창·옵션 Dialog의 Tab/Shift+Tab, 닫힘 중 focus/노드 유지, 종료 후 trigger focus, 열린 채 unmount, Ready/socket0, quiet1.1초 요청 불변, settings_update0, seq11, failures/page/panic0을 확인했습니다. exact 150ms 경계는 portable 시계 검사가 증명하며 browser poll을 픽셀 시각 계측으로 주장하지 않습니다.

Wasm 최초 build5.10초, 최종 inspection 수정 build1.86초 exit0·bindgen exit0입니다. native lib/bins/tests check7.98초 exit0이며 기존 Wry 경고17건 외 새 경고는 없습니다. 이 native check 뒤 생산 동작 변경은 없고 inspection 좌표/fixture만 바뀌었습니다. TS strict/Prettier도 exit0입니다. 같은 성공 Chrome/pure 검사는 반복하지 않습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources 새_picker는_실제_ax_펼침_선택_관계와_disabled_입력_및_escape_회수를_보존한다 -- --exact --nocapture
/Users/hyunseokbyun/development/js/bun/bin/bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-popup-presence
```

원자료는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-popup-presence-result.json`과 `settings-popup-presence-failure.json`입니다. 최신 bindings는 Popup Presence/motion/Tab 및 최종 inspection source입니다. unmount snapshot의 popups/resources/mount는 최신이지만 targets/focused는 probe의 이전 표시 관측이 남을 수 있으므로 그 두 필드를 unmount focus 회수 근거로 쓰지 않습니다.

## 미완료 경계

- [ ] 전체 Popup modifier/cmdk navigation·빠른 재열기·ordered mixed input은 별도 실제 source 대조가 남습니다. 이번 검사로 Ctrl-N/P/J/K 또는 Meta 방향키를 검증했다고 쓰지 않습니다.
- [ ] viewport edge collision/scale clip, 모든 TextEdit AX 자식 bounds, 다중 viewport/top-modal 회수는 전체 화면 검증 시 확인합니다. 현재 단일 viewport happy path 성공을 전체 배치/AX 완료로 확대하지 않습니다.
- [ ] 원본 전체 theme/DPI/픽셀/접근성·실제 CJK/VoiceOver는 남습니다. OS 설정은 변경하지 않았고 실기는 사용자-last 순서를 유지합니다.
- [ ] Snippets/Keymap/LSP/AI/Plugins/Sync/Remote/Performance 실제 UI·전체 App/assets/cutover/Rust99%·최종 N1~N8은 미완료입니다.

이번 Tab·Presence 경계 4/4(100%), Editor/Terminal 세부3/4(75%), provider2/4(50%), 전체363/433(83.83%), 최종0/8입니다. 전체 ETA는 공수가 확정되지 않아 산정 보류합니다. goal active·main 직접·전체완료 전 commit/push 없음·검사 handle 없음이며 제품TS/manifest/lock/MSRV/보호 앱/사용자 데이터/OS/Git은 이번 변경에서 불변입니다.
