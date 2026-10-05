# M8 공용 Settings 렌더러·실제 browser Catalog

## 대상과 완료 범위

기존 후속47의 하위 작업입니다. `native/taide-native-ui/src/{settings-view.rs,theme-editor.rs,icons.rs}`에 기존 native 화면 본문과 원본 SVG를 이동했습니다. native App의 같은 경로는 re-export하고 기존 HostBridge/소유자/저장·삭제 경계를 유지합니다. 공유는 화면을 다시 설계하는 작업이 아닙니다.

`native/taide-remote-web/src/settings-catalog.rs`와 실제 BrowserWorkbench/BrowserEditor가 동일 Settings Views의 owner/mount/generation, typed theme_list/locale_list, 독립 성공·실패, 지연 응답 폐기, 사건 무효화, Closed 단일 완료를 연결합니다. BrowserEditor의 실제 show_settings가 원본 화면을 그리며 catalog 요청을 제출·소비합니다. 다른 화면 명령을 성공으로 삼키지 않고 기존 typed Output으로 caller에 반환합니다.

전체 provider는 아직 미완료입니다. ThemeEditor 원격 Load/Save/Delete, preview 전체 소비, Tooltip/toast, 폴더·settings.json·preference UI 명령, 다른 Settings 섹션, 전체 tab/surface caller를 완료했다고 주장하지 않습니다. 이 결과는 전체 M8/제품 bundle/cutover/TS 제거의 완료 근거가 아닙니다.

## 보존한 경계

- Settings와 ThemeEditor의 본문·위젯·크기·키보드·픽커·미리보기·대화상자를 이동했습니다. ThemeEditor renderer와 icons 본문은 import·공개 경계·cfg·include 경로·rustfmt의 동등 괄호를 제외한 normalized 대조가 같습니다. Settings View/helper 본문도 같습니다. native Settings 9건으로 Request/preview/HostBridge/geometry/AX 소비를 확인했습니다.
- 공유된 내부 View 상태는 비공개 retained 모듈에 두었습니다. native App dev-dependency와 시험용 browser-probe만 inspection feature로 geometry/state를 조회합니다. production remote UI는 해당 feature를 켜지 않습니다. Request는 생성자를 노출하지 않고 owner/mount/generation 읽기만 제공합니다. 실제 browser 제출은 snapshot의 프로젝트·같은 활성 Settings tab을 확인하며, 서버 인가를 대체하지 않습니다.
- ThemeEditor 4건은 shared UI로 이동하고 fixture의 native App bootstrap 의존만 기존 AppServices/TaskSupervisor/실제 domain 저장·guard·NoopEvents로 바꿨습니다. OS Platform은 예상 밖 호출을 Forbidden으로 거부합니다. 이동 전 native test 파일은 중복 dead code로 남기지 않았습니다. Keychain API는 호출하지 않습니다.
- 기존 동일 버전 resvg 0.48.1을 원본 SVG 렌더에 재사용했습니다. Wasm UI의 resvg default/text/system-fonts/raster-images는 켜지 않았고 SVG href의 외부 string/data resolver는 기존대로 None입니다. native 기능 union은 기존 App에서만 유지합니다. native UI lock 17개, remote/probe lock 각각15개 전이를 추가했고 기존 package name/version 삭제·업그레이드는0입니다. MSRV1.95·원래 toolchain을 유지합니다.
- 원격 오류는 model AppError/LocalizedError/AppErrorKind에 Deserialize만 추가해 기존 single-source serde 형태로 좁힙니다. kind/key/args/fallback을 재작성하지 않습니다. 알 수 없는 형태·binary·malformed 목록은 명시 Internal 오류이며 raw JSON을 오류 문구로 출력하지 않습니다. 직렬화 출력/IPC allowlist는 바꾸지 않았습니다.

## 실행 결과

CARGO_HOME은 `/Users/hyunseokbyun/development/rust/cargo`, cargo는 그 아래 bin, target-dir은 `/private/tmp/taide-m8-menu-build.j6Efnw`입니다. Cargo는 직렬 실행했습니다. 성공한 동일 입력 검사는 반복하지 않았습니다.

- [x] shared UI `cargo test --manifest-path native/taide-native-ui/Cargo.toml --offline --target-dir … --lib theme_editor::tests -- --test-threads=1`: 기존 Editor4 PASS, build10.04초/suite.25초입니다. 기존 AppServices fixture의 domain 동작을 실제로 사용합니다.
- [x] native App 정확한 create-host/settings-theme1 PASS(build21.93초/suite.27초). 처음 이동의 누락 import/inspection 접근14개 compile 오류는 해당 경계만 수정했습니다. 나머지 settings_view 검사8은 같은 완료 binary에 `--skip`으로 위 성공1을 제외하고 실행해 PASS(.61초)입니다. 기존 Wry17 경고·linker unwind 경고는 기존 외부 경로이며 변경해 숨기지 않았습니다.
- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir … --test settings-catalog`: 신규2 PASS(build.69초/suite.05초). fixture의 제거된 Context::run 호출과 미소비 TexturesDelta를 기존 run_ui/clear 계약으로 각각 바로잡았고 실패 검사만 재실행했습니다. 실제 Views가 만든 Request, 독립 오류, generation/remount, tombstone 소비, Closed 단일 완료/자동 replay 없음, 사건 취소, 원본 오류6형태 round-trip을 확인했습니다.
- [x] 최종 production Wasm canvas strict2.72초, 실제 probe/production 포함 Wasm strict5.40초 exit0입니다. probe 최초 build14.63초, 읽기 visual fixture 변경 영향 build1.87초·공식 wasm-bindgen0.2.129 생성 exit0입니다. author Rust exactfmt, 검사 도구 TS strict/Prettier·HTML, 관련 diff를 확인했습니다. normal Wasm graph에 taide-runtime/taide-infra/taide-native-app package/Tokio가 없고 순수 resvg/usvg edge만 추가됩니다.
- [x] 실제 Chrome-Wasm 연속 Catalog1 PASS: `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-catalog-result.json`. 최초 합성 Tab.kind를 문자열로 보내 layout DTO를 어긴 실패는 중첩 `{kind: "settings"}`로 fixture만 정정했습니다. seq1~17 고유·연속, 목록 각각4회의 서로 다른 상태 전환(최초/사건/held-remount/새 사건), mount1→2·새 mount generation1→2, 옛 stale-theme 적용0, Localized 오류 공개 뒤 새 locale en, response 누출/실패/panic/page error0입니다. frame27/pump36·정상 close Ready/socket0·quiet1.1초 추가 요청·frame·pump·연결0입니다. 동일 성능을4회 반복한 결과가 아닙니다.

## 픽셀 확인과 미검증 항목

- [x] 첫 통신 결과의 캡처는 모든 합성 foreground/background가 동일색이라 단색이었습니다. 원인은 시험 색상표였으며 product renderer를 바꾸지 않았습니다. 별도 읽기 전용 `settings-visual`1에서 대비 색상만 정정해 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-visual.png`를 직접 확인했습니다. 실제 제목/목차/원본 FileJson SVG/테마 카드/언어 오류/Interface 위젯이 보이고 panic/page error0·Drop/socket0입니다. 해당 fixture 변경의 probe strict는.77초 exit0·Rust16 exactfmt는 모듈 정렬1만 formatter 적용 후 그 파일만 확인했습니다. 기존 연속 성공은 다시 실행하지 않았습니다.
- [ ] 합성 locale는 Settings 번역을 제공하지 않아 일부 label은 원본 message-key fallback입니다. 실제 locale pack의 번역 parity 검증이 아니며 영어 완성 화면이라고 주장하지 않습니다. synthetic palette 또한 원본 theme pixel parity 근거가 아닙니다.
- [ ] 위에서 남긴 실제 ThemeEditor/provider/전체 shell와 제품 자산·패키징을 구현합니다. 동일 원격 close 소유자가 아직 theme mutation pending/failure를 갖지 않으므로 해당 추가 후 close drain까지 함께 연결해야 합니다.
- [ ] 전체 OS/IME/VoiceOver/GUI/성능/security/beta/install/rollback/cutover/Rust99/TS 제거·N1~N8은 기존 미완료 상태입니다. 사용자-last OS 입력기/VoiceOver 검증은 임의 수행하지 않습니다.

현재 renderer/Catalog 하위 경계는 완료했으나 전체 provider2/4(50%)·기존 M8 부모 게이트363/433(83.83%)·후속47 2/4·최종0/8입니다. 전체 ETA는 남은 통합·실기 증거가 없어 산정 보류이며 goal active입니다. main 직접·전체완료 전 Git mutation 없음·보호 앱/OS/사용자 데이터/제품 TS 불변입니다.

참조한 공식 API 근거는 [resvg0.48.1](https://docs.rs/resvg/0.48.1/resvg/), [Rust pub use와 privacy](https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html), [serde_json typed from_value](https://docs.rs/serde_json/1.0.151/serde_json/fn.from_value.html)입니다.
