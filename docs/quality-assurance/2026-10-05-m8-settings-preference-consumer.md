# M8 실제 Settings UI 변경과 실패 피드백

## 대상·계약

BrowserEditor.show_settings가 실제 Output.changes를 한 번 소비하여 기존 typed PreferenceWrites와 같은 socket으로 보냅니다. Theme 선택은 기존 settings_set_theme, 나머지는 원본 단일 필드 SettingsPatch입니다. owner active 확인·원본 숫자 commit·cache/generation·no replay·pending close 대기 계약은 그대로 사용합니다. 즉시 invoke 실패와 matched 응답 실패는 원본 settings.saveFailed 제목/오류 설명의 Toast로 표시합니다.

실패 피드백·진단용 Finished·종료용 일회성 실패를 분리합니다. 실제 runtime에서 과거 결과가 재시도 뒤 Close를 재실패시키는 버그를 먼저 재현하고 원인을 수정했습니다. Toast 표시가 종료 실패를 drain해 숨기거나, 진단 이력을 삭제하여 해결하지 않습니다. switch geometry 추가는 inspection feature에서만 컴파일합니다.

## 최소 검증

- [x] 신규 portable `설정_실패_feedback은_종료_결과를_보존하고_늦은_응답을_중복_표시하지_않는다`1 PASS. 첫 build2.49초는 필터 오기로0건이므로 통과 수에 넣지 않습니다. 정확한 새 검사1 PASS(.06초/.00초), close 채널 수정으로 영향받은 검사만1회 재실행 PASS(1.15초/.00초)입니다.
- [x] 실제 Chrome/Wasm `settings-preferences`: 원본 followSystemTheme 클릭→held/Closing→실패/원본 Toast/진단 result 보존→cancel→명시 현재값 retry→ack 후 Ready/socket0·quiet1.1초. 첫 실행의 과거 오류 재실패 RED를 수정한 뒤 실패 검사1회 GREEN입니다. seq12/settings_update2(거절1/성공1)·Failed frame14/pump17→최종frame20/pump23·페이지 오류/panic/consumer 누출0입니다. 의도 오류1은 result/Toast/Close에 그대로 표시합니다.
- [x] production Wasm canvas strict.54초·최신 probe build2.42초/공식 bindings exit0. TS strict의 synthetic closure nullable narrowing1은 nextSettings를 non-null scope에서 캡처하도록 고친 뒤 영향 검사1회 PASS입니다. Rust/Prettier를 적용했습니다. 이전 Pref/Catalog/Tooltip/Toast 성공 검사는 반복하지 않았습니다.

명령은 기존 Cargo_HOME·locked/offline·전용 `/private/tmp/taide-m8-menu-build.j6Efnw`와 `bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-preferences`입니다. 최신 JSON/PNG는 같은 file-built의 `settings-preferences-result.json`/`settings-preferences.png`이며 최신 bindings는 이 source입니다. 화면의 실제 원본 오류 Toast를 확인하되 전체 제품 close overlay/번역/pixel parity 완료를 주장하지 않습니다.

## 남은 범위

공용 Settings 중 이미 이동한 모든 Change variant는 같은 소비 경로를 쓰지만 이번 실제 입력은 followSystemTheme1개입니다. 이전 typed variant/숫자 검사 성공은 재사용하고20개 스위치를 동일 방식으로 반복 계측하지 않습니다. 폴더/settings.json·전역 preview·남은 섹션/전체 surface caller·제품 bundle/최종 gate는 미완료입니다. 이 소비 경계 완료·전체 provider2/4(50%)·M8 363/433(83.83%)·최종0/8·ETA 보류·goal active·main 직접·Git/live handle 없음입니다. 원격 폴더 열기는 기존 command-policy의 UnreachableDesktopWindow 거절을 유지해야 하며 권한을 확장하지 않습니다.
