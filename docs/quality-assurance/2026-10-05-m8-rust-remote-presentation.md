# M8 원격 Rust 테마·언어 소비자와 시스템 입력 수명

## 결과와 제품 경계

PresentationState와 BrowserWorkbench의 새 테마·언어 연결 경계는 검증했습니다. 순수 소비자3건·실제 Chrome/Wasm 연속1건·새 matchMedia 미지원 분기1건은 각각 한 번 PASS이며, 동일한 성공 시나리오를 반복하지 않았습니다. 후속47의 public UI 준비 안에서 process/verify/save-docs를 main이 직접 적용했습니다. 실제 제품 App/canvas·모든 surfaces/consumer·폰트 공급·제품 자산/패키징·전체 handoff/failed-close/GUI/성능/beta/cutover/Rust99%는 아직 미완료입니다. probe를 제품 화면이나 fallback으로 사용하지 않습니다.

후속47은2/4(50%)·전체363/433(83.83%, 비가중 체크리스트·공수비 아님)·최종 N1~N8은0/8입니다. 이번 테마·언어 통신 경계만 완료이며 전체 완료시간은 산정 보류·goal active·전체 완료 전 commit/push 없음입니다.

## 대상과 원본 계약

- `native/taide-remote-web/src/presentation.rs`: 기존 model ResolvedTheme/ResolvedLocale와 Settings DTO를 역직렬화합니다. theme_get_current에는 필수 systemTheme, locale_get_current에는 필수 systemLanguage를 전달합니다. 조회 결과의 색상·편집기 appearance·번역은 같은 taide-native-ui::presentation을 사용합니다. 아직 받은 적 없는 theme/locale를 빈 값이나 임의 기본값으로 가장하지 않습니다.
- `native/taide-remote-web/src/browser-workbench.rs`: 실제 BrowserShell을 하나만 소유하며 반환된 event/response를 소비합니다. 원본 BrowserClient의 socket·seq·pending·reconnect 정책은 그대로 사용합니다. 외부 consumer에는 소유하지 않은 응답/이벤트를 반환하며 mutable/raw Client를 공개하지 않습니다.
- `native/taide-remote-web/{Cargo.toml,src/lib.rs}`: 기존 web-sys0.3.106의 Navigator/MediaQueryList/EventTarget feature만 활성화했습니다. 새 라이브러리나 버전 업그레이드, root/native lock/MSRV 변경은 없습니다.
- `tests/presentation.rs`, `tests/browser-probe/{src/presentation-probe.rs,src/lib.rs,presentation.html}`, `tools/m8-remote-rust-presentation-probe.ts`: 실제 Rust owner를 실행하는 격리 검증 전용 source입니다. 합성 서버 응답·fixture API 부재만 대역이며 실제 소비자와 listener를 대체하지 않습니다.

원본 `src/{entities/theme/theme.query.ts,entities/locale/locale.query.ts,shared/lib/system-appearance.ts,app/providers/ipc-sync-provider.tsx}`와 native `remote-preferences.rs`의 필수 인자를 대조했습니다. themeId/followSystemTheme 변경은 theme만, language 변경은 locale만 무효화하고 무관한 editor font 변경으로 추가 조회하지 않습니다. theme:changed는 theme를 무효화하고 recovered Connected는 둘을 갱신합니다. navigator.language는 원본과 같이 초기 읽기이며, matchMedia의 change listener는 repaint wake만 보냅니다. poll에서 실제 matches를 읽고 값이 바뀔 때만 입력 문자열을 갱신합니다.

## 소비·오류·종료 계약

- [x] 테마·언어별 in-flight 조회는 하나이며 여러 invalidation을 dirty 하나로 합칩니다. 변경 후 도착한 옛 성공/오류는 현재 값에 적용하지 않고 완료 후 새 인자로 한 번 재조회합니다. 필수 settings 초기 조회를 받은 뒤 theme/locale reads를 시작해 baseline 도착 때문에 초기 조회가 중복되지 않습니다.
- [x] malformed/binary·remote error·invoke 실패를 유형별로 공개하고 무한 자동 재시도하지 않습니다. retry_presentation은 지정한 조회만 재시도하며 refresh/recovery는 전체를 갱신합니다. 이미 성공한 데이터는 오류·재조회 중 유지하고 아직 없는 데이터는 None으로 남습니다. 실제 제품의 로딩/오류/재시도 UI는 별도 연결이 남습니다.
- [x] disconnected에서 pending/dirty를 지우고 새 연결에서 새 seq로 조회합니다. presentation이 소유한 응답은 다른 consumer에 넘기지 않습니다. 원본의 mutation 응답/채널/lifecycle 전달은 유지합니다.
- [x] ThemeListener가 Closure를 소유하고 Drop에서 removeEventListener를 호출합니다. 명시 dispose는 listener와 실제 shell socket/timer를 정리합니다. callback forget/global 강한 수명 우회는 없습니다. missing/null matchMedia는 원본의 dark 및 no-listener 경로를 보존하며 사용 가능한 API의 실행/등록 오류는 숨기지 않습니다.

## 실제 최소 검사와 실패 구분

CARGO_HOME=`/Users/hyunseokbyun/development/rust/cargo`, cargo=`/Users/hyunseokbyun/development/rust/cargo/bin/cargo`, target=`/private/tmp/taide-m8-menu-build.j6Efnw`입니다. Cargo 명령은 `--locked --offline --target-dir <target>`입니다.

- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test presentation`: 최초 테스트 작성에서 없는 Settings.font_size_code/InvokeError::Disconnected 때문에 compile101이었습니다. 제품의 실제 editor_font_size/Closed 계약으로 테스트만 정정한 뒤 실패한 신규3건이 build0.31초·suite0.00초·3 PASS입니다. 필수 인자/공유 색상·font·번역/설정·system·late coalescing/remote·binary 오류/명시 retry/새 연결 seq가 대상입니다. 기존 순수 shell/state/transport 성공은 재실행하지 않았습니다.
- [x] browser-only `cargo clippy --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib -- -D warnings`: 최초 소비자·listener 연결은2.53초 exit0이며 이후 missing matchMedia/변경 시에만 문자열 갱신하는 실제 코드 추가 뒤 최종0.33초 exit0입니다. 같은 입력의 반복 strict가 아닙니다. native App/공유 renderer의 기존 성공은 변경 영향이 없어 재사용했습니다.
- [x] 새 probe 최초 build는 존재하지 않는 ShellSnapshot.settings fixture 접근으로101이었습니다. 실제 workbench.shell().state().settings()로 수정한 뒤0.51초 exit0, 조회 중 표시를 추가한 새 probe는0.60초 exit0입니다. 마지막 unsupported matchMedia 코드가 포함된 probe는0.75초 exit0이며 이전 통과한 연속 runtime을 재실행하지 않았습니다.
- [x] 기존 검증된 wasm-bindgen CLI0.2.129를 `--target web --no-typescript --out-name probe --out-dir /private/tmp/taide-m8-wasm-tools.h2xBQB/presentation-built <wasm>`로 사용했습니다. transport/shell-built의 선행 결과를 덮어쓰지 않았습니다.
- [x] 신규 tool TS strict는 closure에서만 변경되는 settings의 narrowing 때문에 최초 TS2339였고, settingsSchema.parse(settings)에서 실제 값을 검증하도록 수정해 exit0입니다. 미지원 모드의 early return은 top-level TS1108이었고 명시 async 검증 진입 함수와 finally로 정리한 최종 strict는 exit0입니다. type assertion/검사 억제를 쓰지 않았습니다.

## 실제 Chrome/Wasm 결과

`bun tools/m8-remote-rust-presentation-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/presentation-built`를 새 합성 localhost server/격리 headless Chrome에서 연속1회 실행해 PASS입니다. sandbox localhost EPERM은 앞선 실제 근거가 있으므로 해당 명령만 require_escalated로 실행했습니다. 새 browser context는 ko-KR/light·serviceWorkers block·다운로드 금지·mock Keychain이며 사용자 OS/입력기/VoiceOver/기존 Chrome/profile/Keychain/보호 앱을 변경하지 않았습니다.

- [x] bootstrap6 reads에 필수 systemTheme=light/systemLanguage=ko-KR, 실제 shared colors=[16,32,48,255]·editorFont13·ko Rust·응답 소비 후 외부 responses0입니다.
- [x] dark 조회를 보류한 동안 light로 바꿔 실제 change listener가 wake를 보냈습니다. 옛 dark 응답은 소비하되 적용하지 않았고 새 light read 한 건으로 합쳤습니다. theme:changed 뒤 실제 적용 이력은 1-light→2-light뿐이며 1-dark를 적용하지 않았습니다.
- [x] 옛 locale read 보류→SettingsChanged(language=ja)→새 locale read·ja Rust, 실제 remote error1을 공개하고 이전 ja 데이터 유지→명시 locale retry에서 오류0입니다.
- [x] restart1012→새 연결의 read6→recovery1·theme2-light/localeja·refreshing false·wake26·외부 responses0·전체 seq1~20 고유·theme reads5/locale reads6·upgrade2입니다. seq14는 fixture-only cut이며 제품 command policy를 추가한 것이 아닙니다.
- [x] 명시 dispose 뒤 socket0·connected false이고 다시 dark로 에뮬레이션해1.1초 기다려도 wake26 유지·요청 추가0·upgrade2 유지·Drop/page error0입니다. 이는 현재 owner listener/transport 정리이며 전체 App/GPU/native handoff의 증거는 아닙니다.

결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/presentation-built/presentation-result.json`에 보존했습니다.

새 missing matchMedia 분기는 `bun ... presentation-built unsupported-media`로 **그 분기만1회** 실행해 PASS입니다. 새 격리 페이지의 API를 undefined로 만든 합성 조건이며 실제 구형 브라우저나 OS 설정을 바꾸지 않았습니다. bootstrap6·systemTheme dark/ko-KR·theme1-dark·공유 색상/font/message·error0·dispose/socket0·추가 요청/upgrade0입니다. 결과는 같은 디렉터리의 `presentation-unsupported-result.json`이며 연속 검사 결과를 덮어쓰지 않았습니다.

직접 작성한 Rust6 exact rustfmt와 tool/HTML Prettier --check는 exit0입니다. 기존 transport/BrowserShell/UUID/actual native App/server/font/color 성공은 재사용했습니다. 이번 renderer 소비자 기반으로 전체 GUI/99%/배포 완료를 주장하지 않습니다.

API는 설치된 공식 web-sys0.3.106의 Window.match_media/Navigator.language/MediaQueryList.matches/EventTarget listener 및 js-sys Reflect.get 바인딩을 읽었습니다. [Navigator.language 문서](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/language)와 [MediaQueryList change 문서](https://developer.mozilla.org/en-US/docs/Web/API/MediaQueryList/change_event)를 확인했습니다.

## 다음 제품 연결

후속 shared surface 이동: 같은 native chord 표시의 본문·치수·문구·색상/font·키보드 아이콘을 `taide-native-ui/src/status-chord.rs`로 이동했습니다. ChordStatus도 같은 정의를 이동하고 native keymap 경로는 re-export하므로 actual Window/terminal/App 호출에서 변환·복제본이 없습니다. 기존 native `status-chord.rs`는 Appearance/show를 re-export합니다. renderer 테스트는 원래 native App 위치에서 새 공개 Appearance::new를 거쳐 shared show를 호출합니다. UI의 새로운 단독 unit-test 위치는 serde_json 직접 dev edge가 없어 최초 compile101이었으므로 새 라이브러리를 추가하지 않고 기존 검증 환경을 유지했습니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib chord_status -- --nocapture`: actual viewport pending/deferral/no-match/단일 timeout route와 shared renderer의 원문구·불일치 우선·11px/font·color·9 icon dots·default 표시 없음이2 PASS(build14.34초·suite.02초·나머지342 filtered)입니다. 기존 Wry17 경고와 __eh_frame linker warning을 억제하지 않았습니다. native App/GPU/OS 실측의 증거는 아닙니다.
- [x] 직접 수정한 UI/compatibility Rust3 exact rustfmt는 exit0입니다. keymap에서는 타입 정의5줄을 re-export로만 바꾸고 파일 전체를 포맷하지 않았습니다. runtime/timer/키 매칭 본문은 변경하지 않았습니다.
- [x] surface 이동 뒤 production browser `clippy ... --target wasm32-unknown-unknown --lib -- -D warnings`는0.50초 exit0입니다. shared renderer의 새로운 cross-crate 경계를 컴파일했으며 이미 성공한 theme/locale runtime은 다시 실행하지 않았습니다.

- [ ] 실제 browser App/canvas와 원본 모든 ShellSurfaces를 같은 consumer·shared renderer에 연결합니다. no-op surface/empty theme/JS fallback으로 제품 자산 게이트를 채우지 않습니다.
- [ ] 폰트/preview-theme·catalog/provider/error UI·파일/탭/terminal/LSP/기타 read/write/channel consumer 및 native/remote query ownership을 완료합니다.
- [ ] 제품 Rust assets/manifest/패키징·전체 수명/failed-close/GUI·M8 성능/beta/cutover/Rust99%·설치/rollback은 상위 게이트에서 실제로 검증합니다.
