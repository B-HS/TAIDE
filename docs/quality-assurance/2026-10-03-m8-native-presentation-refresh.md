# M8 native hot theme·locale 기본

## 대상 파일

- `native/taide-native-app/src/{presentation-refresh.rs,application.rs,host.rs,keybinding-editor.rs,terminal_surface.rs,lib.rs}`
- `crates/taide-runtime/src/{theme_actions.rs,locale_actions.rs}`
- 원본 `src/app/providers/{theme-provider.tsx,locale-provider.tsx,ipc-sync-provider.tsx}`, `src/widgets/app-toaster/app-toaster.tsx`, `src/features/terminal/terminal-view.tsx`

## 리포트

기존 native App은 theme·locale을 초기화 때만 읽었습니다. Settings/Theme 이벤트와 follow-system-theme 변경 경계에서 기존 bounded host로 현재 창을 재조회하고, stale/취소/종료 응답을 적용하지 않도록 연결했습니다. theme의 전체 appearance는 검증한 뒤 적용하며 locale 결과는 theme 실패와 독립입니다. 열린 keybindings의 query/capture/focus와 문서·terminal·toast queue는 재생성하지 않습니다. App 배선은 컴파일 근거와 하위 결정적 검사이며 full App/모든Settings UI/실기/전체 M8 완료가 아닙니다.

## 상세

1. 원본은 themeChanged 및 themeId/followSystemTheme/language를 바꾸는 Settings 이벤트에 theme·locale query를 무효화합니다. provider는 현재 화면 변수/메시지를 적용하고 AppToaster는 theme type을 읽습니다. terminal-view도 기존 terminal의 options.theme를 바꿉니다. native는 현재 직접 배선된 appearance가 editor 설정에도 의존하므로 Settings 전체를 snapshot으로 비교하고 이벤트를 coarse invalidation합니다. 원본의 모든 mutation·preview/cache 정책이나 성능 동등성을 완료로 계산하지 않습니다.
2. event sink는 Settings/Theme 이벤트의 atomic revision만 기록합니다. 요청은 창마다 한 개만 pending이며 여러 중간 이벤트의 payload를 쌓지 않습니다. Settings snapshot·사용되는 system theme/language·event revision·요청 sequence를 비교합니다. 다른 이벤트는 revision을 올리지 않습니다. follow=false/고정 language에서는 사용하지 않는 system 값은 key에서 제외합니다. 시스템 언어는 navigator.language에 대응하는 창 시작 값이며 실제 OS 언어 변경 observer는 없습니다.
3. 기존 runtime resolver에 snapshot Settings/명시 language 진입점을 추가하고 current 진입점도 같은 구현에 위임합니다. UI에서 변경된 live Settings를 IO 중 다시 읽어 theme/locale이 섞이는 경로를 피합니다. 잠금을 가진 채 파일을 읽지 않습니다. 기존 identifier 검증·builtin/system fallback·user pack resolution을 재구현하지 않습니다. load는 기존 TaskSupervisor blocking task와64-capacity host를 사용합니다. [Tokio blocking 작업 계약](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)과 실제 TaskSupervisor 코드를 확인했으며 새로운 runtime/의존성/별도 무상한 채널은 없습니다.
4. finish는 pending identity를 먼저 확인하므로 duplicate/취소 전 reply가 새 pending을 제거하지 않습니다. stale reply를 버린 뒤 같은 frame의 poll 다음에 최신 요청을 제출합니다. 실패도 attempt로 기록해 매 frame 무한 재시도하지 않으며 후속 event/change로 다시 시도합니다. 명시적 retry 버튼과 두 독립 load-error banner는 아직 구현하지 않았습니다. 원본처럼 theme 오류가 정상 locale 결과를 막지는 않습니다.
5. shell/editor/banner/terminal/keybindings/pdf/presentation/spreadsheet/toast type을 모두 준비한 뒤 App 필드를 교체합니다. 누락·잘못된 색상은 기존 appearance를 유지합니다. keybindings는 appearance만 바꾸고 기존 query/capture/focus/icons를 유지합니다. locale는 현재 메시지를 바꾸며 이미 발급된 toast 문자열을 재번역하거나 수명·identity를 초기화하지 않습니다. raw literal toast/모든 hot label의 실제 render 비교는 별도입니다.
6. 기존 terminal color fallback callback이 attach-time Appearance를 캡처하고 있었습니다. App 소유 shared palette를 기존 actor callback이 읽도록 연결하고 lazy 첫 show/새 theme 적용에서 갱신합니다. queued/hidden actor도 해당 callback을 유지합니다. 각 창은 별도 palette이며 callback은 Arc 소유 수명으로 종료 drain 중에도 마지막 palette를 읽을 수 있습니다. Core의 captured OSC override 우선순위·Writer/dispatcher/control order·geometry callback은 변경하지 않았습니다. 실제 PTY feed/OSC wire·hidden command-marker·동시 theme/event 순서는 full gate입니다.
7. public API와 installed egui source를 확인했습니다. shadow Frame::paint는 Rect를 Shape::Vec 안에 반환하므로 실제 renderer 검사도 중첩 shape를 순회합니다. [egui Context 계약](https://docs.rs/egui/0.36.2/egui/struct.Context.html)에서 경고하는 중첩 Context lock은 추가하지 않았습니다. 이번 변경은 OS theme 설정/native window chrome을 쓰지 않고 제품 TS·vendor·보호 bundle·root lock/MSRV를 변경하지 않았습니다.

## 실제 검사

동일 app manifest·locked/offline·공유 target의 serial Cargo, synthetic AppState/embedded builtin·headless egui만 사용했습니다. clipboard port는 호출 시 실패하는 합성 함수이며 실제 clipboard·OS 설정·사용자 data·PTY를 사용하지 않았습니다.

- [x] 신규 `--lib native_presentation_refresh`:첫 batch host1 PASS/fixture2 FAIL(suite0.21초, compile8.32초). Settings 기본 language=system을 고정 언어로 가정한 오류와 최상위 Rect만 찾은 renderer 오류였습니다. 제품 bug RED로 계산하지 않습니다.
- [x] 고정 language fixture 수정 뒤 정본 appearance/system 경계1 PASS(suite0.06초, compile1.95초). 전체 builtin을 새로운 aggregate constructor로 확인하고 누락 색상 오류·현재 Settings 불변·follow-system light/dark·ko/ja를 검사했습니다. 이미 성공한 기존 builtin Appearance 검사를 다시 실행한 것은 아니며 새 aggregate 위험을 덮습니다.
- [x] 열린 renderer는 time 경계 수정에도1 FAIL(suite0.19초), shape 수집 진단1 FAIL(suite0.22초, compile3.28초)이었습니다. 설치된 Frame::paint와 기존 geometry 검사의 Shape::Vec 순회 근거로 fixture를 수정한 뒤1 PASS(suite0.21초, compile3.65초). 실제 translated title/light modal 배경·열린 query/capture·previous focus를 덮습니다. fade가 주원인이라는 초기 commentary는 정정했습니다. 동일 가정으로 blind 제품 수정을 하지 않았습니다.
- [x] strict의 large-enum-variant2 FAIL:HostCommand544bytes/HostReply912bytes로 bounded queue 전체를 키우는 차이를 검출했습니다. Request를 Box로 전달해 원인을 수정했으며 suppression을 쓰지 않았습니다. 수정 상태의 lib/bin/test strict exit0(12.51초), 영향 host1 PASS(suite0.01초, compile4.77초)입니다. 이 strict는 다음 독립 결과/공유 palette 변경 이전 근거이지 최종 결과가 아닙니다.
- [x] theme/locale 독립 결과 변경 뒤 관련 resolver/host2 PASS(suite0.06초, compile4.70초). captured light/ko를 요청한 뒤 live dark/ja로 변경해도 request snapshot을 읽고, UI admission은 stale을 거부합니다. 새 pending에 옛 reply가 영향을 주지 않음·same-id ThemeChanged·취소/replacement·invalid theme에도 정상 ja locale·동일 실패의 자동 반복 없음·host disconnect를 덮습니다. 변경 없는 열린 renderer 성공은 재사용합니다.
- [x] shared palette fixture의 WindowSize::default E0599를 설치된 public fields로 수정했습니다. 이어 headless FullOutput의 미처리 TexturesDelta Drop1 FAIL(suite0.01초, compile4.46초)을 기존 renderer 계약의 clear로 정리한 뒤 callback1 PASS(suite0.01초, compile1.86초)입니다. 두 기존 callback의 named/ANSI 최신 fallback·invalid index·창별 분리·Views Drop 뒤 소유 수명을 검사했습니다. 실제 actor/PTY wire 검사가 아니라 실제 EffectPorts callback 검사입니다.
- [x] 마지막 제품 변경 뒤 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(11.21초)입니다. 이후 제품 코드는 변경하지 않았고 위 fixture의 TexturesDelta 정리만 바뀌어 좁은 test/compile 결과를 사용했습니다. queue/host/App/terminal 제품의 strict 성공을 반복하지 않았습니다. authored8파일 exact rustfmt --check·tracked whitespace exit0, untracked authored/manifest/lock12파일 no-index whitespace 출력은 비어 있습니다(exit1은 /dev/null과 내용 차이). inherited Wry17 warnings는 분리합니다.

## 남은 gate

- [ ] 실제 App에서 theme/locale/Settings mutation·두 오류 배너/retry·같은 frame/late/종료/재개/다중 창·aux/WebView/hidden/minimized와 모든 label을 검증합니다. 하위 host·renderer 검사만으로 full App 적용을 완료로 계산하지 않습니다.
- [ ] theme draft preview/save/import/delete·syntax/tokenColors/Shiki/Monaco·전역 native widget styling/window chrome·OS focus/wake/system appearance·locale pack 변경 observer·전체 theme/font/OS 픽셀·성능을 재현합니다.
- [ ] terminal live OSC fallback/override/writer wire·hidden command-marker·queued theme 순서와 실제 source xterm 비교를 검증합니다. Core/dispatcher의 기존 query snapshot 성공은 이번 무변경 범위에서 재사용하며 모든 hot path 성공으로 확장하지 않습니다.
- [ ] toast selection/capture/touch·parent position·raw/translated hot label·same-frame/late/aux·전체 focus-visible·nonmac reduced-motion과 실제 OS/AX gate는 미완료입니다. 별개 keybindings Tab RED와 remount A/B 응답 대기를 해소한 근거가 아닙니다.
- [ ] 전체213view/41action/Monaco21/palette·N1~N8은0/8이며 cutover/TS제거·성능/보안/배포/rollback과 완료 감사가 남습니다. 목표active·전체 M8 완료 뒤만 commit/push합니다.

process/verify/save-docs skill은 현재 체크리스트·실패 영향만 재검사·성공 재사용·정확한 분류 기록에 적용했습니다.
