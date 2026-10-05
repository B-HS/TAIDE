# M8 실제 Settings 폴더 버튼의 원격 거절 보존

## 대상·리포트

`native/taide-remote-web/src/settings-folders.rs`와 BrowserWorkbench/BrowserEditor에 실제 Output.folders 소비·typed kind·matched seq·단위 응답·Closed 단일 오류·no replay를 연결했습니다. 원본 TS SettingsView의 폴더 실패는 일반 IPC 오류 Toast 제목이며 Settings 저장 실패 제목으로 바꾸지 않습니다. 같은 Toast provider를 재사용하고 Theme 오류도 같은 Ipc 분류로 이름만 명확히 했습니다.

기존 `crates/taide-remote/src/command-policy.rs`의 system_open_app_data_path는 UnreachableDesktopWindow 거절입니다. 이 정책/allowlist/서버는 수정하지 않았습니다. 버튼은 같은 명령을 보내고 localized Forbidden을 사용자에게 표시합니다. 원격에서 폴더를 실제로 열거나 OS 권한을 늘리지 않습니다. 클라이언트 표시로 서버 인가를 대체하지 않습니다.

## 검증

- [x] 새 portable1 첫 PASS(build1.09초/.00초): Themes/Locales/Plugins/Snippets args·unknown/중복 seq·기존 Forbidden 보존·Closed1회·malformed binary·즉시 invocation 오류.
- [x] 새 실제 Chrome/Wasm `settings-folders` 연속1 첫 PASS: 원본 themes 폴더 버튼 실제 클릭→정확한 typed command1→기존 localized 거절/raw Toast→unmount→Ready/socket0·quiet1.1초 불변. seq10·frame17/pump18→최종frame32/pump34·page/panic/consumer 누출0입니다. 이전 성공 경계는 반복하지 않았습니다.
- [x] 최신 probe build2.84초·공식 bindings·TS strict/Prettier·Rust9파일 exactfmt exit0. 최신 native lib/bins/tests strict6.25초 exit0이며 기존 Wry17 외 새 경고 없습니다. normal Wasm graph에 taide-runtime/infra/native App/terminal/Tokio0입니다.

실측은 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-folders-result.json`/`settings-folders.png`입니다. 실제 원본 Settings 화면 위 거절 문구 Toast/아이콘/닫기를 확인했습니다. 합성 locale fallback이며 전체 번역/pixel parity/제품 close UI 완료를 주장하지 않습니다. 이 fixture의 거절 응답은 기존 서버 정책을 동일 재현한 것이며 실제 OS 호출 검사가 아닙니다.

이 경계 완료·provider2/4(50%)·전체363/433(83.83%)·최종0/8·ETA 보류·goal active·main 직접·Git/live handle 없음입니다. settings.json 실제 tab/document caller·전역 preview·나머지 Settings 섹션·전체 App/assets/bundle/최종gate는 남습니다. OS 설정/보호 앱/사용자 데이터/제품TS·기존 버전/MSRV를 유지합니다.
