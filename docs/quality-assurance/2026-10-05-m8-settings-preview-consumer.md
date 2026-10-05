# M8 Settings preview 소비

## 대상·구현

`native/taide-remote-web/src/browser-editor.rs`가 원본 Views.preview_where의 viewport/active owner/sequence 계약으로 표시 테마를 선택합니다. Settings/Tooltip/File/banner/Open Toast appearance에 같은 선택을 사용하고, close Pending/Failed Toast는 native와 같은 base theme를 사용합니다. finish_ui_frame에서 preview 변경과 unmount 제거를 관찰해 repaint합니다. 서버 설정이나 테마 파일을 preview 때문에 쓰지 않습니다. 전체 App/shell/terminal/surface appearance caller는 아직 미완료입니다.

Picker 원본 생산 코드는 변경하지 않았습니다. shared ThemeEditor에 inspection 전용 실제 Picker ID/rect trace만 추가했습니다. browser-probe는 원본 표시 테마·다른 viewport fallback·focus와 입력 상태를 읽습니다.

## 검증·실패 원인 구분

- [x] normal Wasm strict .73초와 실제 probe build(최신 입력 관찰 추가본 1.47초) exit0. 앞선 TS strict/Prettier 및 해당 Rust fmt PASS는 같은 소스의 결과를 재사용합니다.
- [x] 새 native Picker 밖 클릭 blur 검사 첫 PASS(9.83초/.03초), 외부 focus가 먼저 이동하는 입력을 추가한 변경 검사 PASS(1.36초/.03초). 같은 성공 입력을 반복하지 않았습니다.
- [x] 새 전체 ThemeEditor background 검색/HEX focus/이름란 blur/preview 검사 1 PASS(build .97초/실행 .14초). 원본 전체 Editor에서도 선택한 HEX가 focus를 유지하고 #244466이 적용됩니다.
- [x] 실제 Chrome/Wasm preview 연속 검사 RED→GREEN. startup theme null readiness와 원본 namespace/token 검색 입력을 고쳤습니다. HEX focus [] 진단은 enabled=true/정확한 Foreground layer/input.focused=true이나 pointer y446.0, 최신 HEX 중심462.5로 실제 hit-test=false였습니다. fixture의 Popup 초기 sizing rect 조기 클릭을 실제 containsPointer 확인 후 클릭으로 고쳤으며 생산 Picker는 수정하지 않았습니다. 추가 진단 JSON이 Zod에서 제거되던 시험 오류도 스키마로 보존했습니다. 임의 sleep/focus 주입 없음입니다.
- [x] 실패 영향 검사 1회 PASS: 실제 HEX focus→#244466 preview·다른 viewport #102030 유지·unmount #102030 복원·save/delete/settings 쓰기0·Ready/socket0·quiet1.1초 frame/pump/request 불변·page/panic/누출0. seq11·preview frame41/pump37→최종51/48입니다. 성공을 반복하지 않습니다.
- [x] 최신 normal production Wasm/canvas strict1.00초, Rust5 exactfmt·최신 fixture TS strict/Prettier exit0.

성공 자료는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-preview-result.json` 및 `settings-preview.png`입니다. 실제 원본 Editor의 #244466/변경 reset 버튼을 확인했습니다. 직후 Popup fade·합성 locale fallback/fullpixel parity 완료는 주장하지 않습니다. 실패 JSON/PNG는 진단 역사이며 최신 성공과 혼동하지 않습니다. 최신 bindings는 preview 구현과 입력 관찰 소스이며 이전 Toast/preferences/folder 성공은 당시 소스의 독립된 증거입니다. ColorPicker 밖 클릭 구현이 결함이라는 초기 설명은 순수 검사로 확인되지 않아 철회했습니다. 공식 온라인 Response 문서는 접근 실패했으며 설치된 동일 egui-input의 Response/Popup/Context 구현과 API 문서를 읽어 진단합니다.

provider2/4(50%)·전체363/433(83.83%)·최종0/8·ETA 산정 보류·M8 미완료입니다. settings.json 실제 tab/document·남은 Settings/App/전체 caller/assets/최종 gate가 남고 전체 M8 완료 전 Git 작업은 하지 않습니다.
