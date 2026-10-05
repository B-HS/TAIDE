# M8 Keymap 설정 화면·native 편집기 의도

후속 정본은 `2026-10-06-m8-shared-keybinding-editor.md`입니다. 같은 생산 편집기/core·JSON의 공용화, 실제 BrowserEditor/Canvas의 global modal·Settings 출력 소비·typed 저장/실패/명시 재시도·종료 drain까지 새 연속 검사로 확인했습니다. 아래 결과는 앞선 native section 경계의 당시 기록이며 전체 browser 전역 명령 dispatch·시각/실기 완료를 뜻하지 않습니다.

## 대상 파일

`native/taide-native-ui/src/settings-controls.rs`, `settings-view.rs`, `settings-code-view.rs`, `native/taide-native-app/src/application.rs`, `native/taide-remote-web/tests/settings-resources.rs`입니다.

## 리포트

원본 `src/widgets/settings-view/settings-keymap-section.tsx`, `settings-view.tsx`, `src/features/settings/settings-section.tsx`, `src/shared/ui/button.tsx`, `card.tsx`, `src/shared/styles/global.css`를 읽고 Keymap 목차·제목·설명·편집기 열기 버튼을 shared Settings 화면에 추가했습니다. 기본 section은 6→7개입니다. native는 기존 global 키바인딩 편집기를 열며 browser 편집기 소비는 미완료입니다. 제목/버튼만으로 전체 Settings/Keymap 화면 재현 완료를 주장하지 않습니다.

## 상세

1. Keymap은 Terminal 뒤에 표시합니다. 원본 Snippets는 Editor와 Terminal 사이지만 아직 해당 takeover/편집기를 구현하지 않았으므로 빈 Snippets section을 추가하지 않았습니다.
2. CardHeader의 title/description 간격6·설명12px·outline sm 버튼32px/14px/medium/px12/radius6을 적용했습니다. app.background/app.foreground/app.border와 list.hoverBackground/list.foreground를 재사용하며 글꼴은 기존 shared medium family/fallback입니다. 이번 parent 색상 재사용에 필요한 child Appearance 두 필드만 pub(super)로 열었습니다.
3. `Output.open_keybindings`는 실제 버튼 clicked의 일회성 출력입니다. native AppSurfaces는 이를 `ShellIntent::OpenKeybindings`로 보내고 기존 `NativeApplication` 의도 처리에서 `self.keybindings.open(&context)`를 호출합니다. 새 설정 쓰기/RPC·사용자 파일·전역 프로젝트 추정·새 bridge를 만들지 않았습니다.
4. 새 버튼 로직은 이미 깊은 View.show 안의 긴 블록을 읽기 어렵게 만들기 때문에 파일 private keymap 함수로 분리했습니다. 미래 재사용을 전제로 새 공용 abstraction을 도입한 것이 아닙니다. 기존 다른 CardHeader의 렌더 경계는 원래대로 유지했습니다.

## 검증

- [x] 신규 정확한 UI 검사에서 `missing settings.keymap` RED(exit101, build0.44초/실행0.09초)를 확인했습니다.
- [x] 같은 검사 GREEN(exit0, build1.55초/실행0.11초): 실제 목차 클릭·scroll 후 설명 TextShape, 버튼32px/AX Button label, 실제 클릭의 open 의도, 다음 frame false, disabled 클릭 false, 설정 변경0을 확인했습니다. 첫 구현의 child 색상 private E0616 두 건은 pub(super)로 고쳤습니다.
- [x] native lib/bins/tests check6.27초 exit0입니다. 실제 AppSurfaces의 typed intent 연결이 컴파일되며 기존 Wry17경고 외 새 경고는 없습니다. 이후 별도 런타임에서 전체 NativeApplication UI를 실행했다고 주장하지 않습니다.
- [x] normal browser Canvas Wasm lib check14.27초 exit0·변경 Rust5 exactfmt exit0입니다. Keymap 로직의 파일 private 함수 분리는 의미 동등하므로 성공한 UI 검사를 반복하지 않았습니다. Popup Chrome/pure/native 성공도 그대로 재사용합니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources 원본_keymap_section은_목차와_설명_및_기존편집기_버튼을_표시한다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --lib --bins --tests --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --lib --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

## 미완료와 다음 경계

- [ ] browser의 실제 global 키바인딩 편집기 소비자/동일 Settings 저장·keymap 전역 입력 연결은 아직 없습니다. BrowserEditor.show_settings는 해당 flag를 반환하지만 현재 probe는 편집기를 표시하지 않습니다. 이를 성공한 browser 동작으로 쓰지 않습니다.
- [ ] 원본 전체 button focus-visible/hover/disabled/shadow·theme/DPI/픽셀·키보드·실제 native App UI 왕복은 전체 화면 재현 단계에서 확인합니다. 이번 metadata·단일 클릭 성공만으로 그 시각/실기 경계를 대체하지 않습니다.
- [ ] Snippets/별도 편집기 takeover, LSP/AI/Plugins/Sync/Remote/조건부 Performance와 기존 화면의 남은 재현·App/assets/cutover/Rust99%·N1~N8은 남습니다.

native 연결 세부3/3(100%)는 shared UI/typed native 의도/해당 최소 검사·문서의 범위입니다. Keymap 전체/Settings 부모는 미완료입니다. Editor/Terminal3/4(75%)·provider2/4(50%)·전체363/433(83.83%)·최종0/8·전체 ETA 산정 보류입니다. goal active·main 직접·전체완료 전 Git 없음·live 검사 없음입니다. 최신 probe bindings는 Keymap 이전 Popup source이며 이번 source는 normal Wasm check로만 확인했습니다. 제품TS/manifest/lock/MSRV/보호 앱/OS/사용자 데이터/Git은 이번 변경에서 불변입니다.
