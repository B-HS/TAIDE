# M8 native terminal 폰트·커서 설정

## 대상·상태

`native/taide-native-app/src/{application,terminal_settings,terminal_fonts,system_fonts,terminal_host,terminal_surface,preview_svg}.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/src/{lib,session}.rs`, `tests/cursor.rs`, native Alacritty fork입니다. 기존 TS `terminal-view.tsx`와 `font-stack.ts`, 설치된 xterm InputHandler/WebGL CursorBlinkStateManager를 기준으로 초기·실시간 설정 경계를 연결했습니다. 메인이 workflow·서브에이전트 없이 수행했습니다. N4/M8 전체와 N1~N8 0/8은 유지하며 전체 완료 뒤만 commit/push합니다.

## 커서·크기

- Hub의 실제 Core 생성 직후, PTY spawn 전에 settings의 Bar/Block/Underline·blink 기본값을 적용합니다. SharedTerminal의 publication/state lock 안에서 실제 설정 변경만 수락하며 같은 값은 다시 덮어쓰지 않습니다. 따라서 프로그램의 DEC12 변경을 매 UI tick의 같은 사용자 설정값으로 취소하지 않습니다. 앱은 숨은 session을 포함한 같은 Hub에서 실제 변경을 반영하고 repaint합니다. parser/history/title·revision을 초기화하거나 새 Core를 만들지 않습니다.
- native feature의 기본값 setter는 Alacritty Config의 cursor 기본값과 damage만 갱신합니다. DECSCUSR override는 그대로 우선합니다. native DEC12는 기존 xterm처럼 기본 blink option을 변경하며 DECSCUSR override를 덮어쓰지 않습니다. native DECRQM 12는 xterm InputHandler의 options blink 값으로 응답하고 query 자체가 override를 생성하지 않습니다. feature가 꺼진 upstream handler 분기는 유지합니다. CSI 0 q/RIS와 hidden cursor를 actual Core로 검사했습니다. 모든 키/VT/VI mode parity 완료는 아닙니다.
- renderer는 같은 Core의 effective cursor style과 기존 hidden shape를 읽습니다. focused cursor의 600ms 주기로 다음 repaint를 예약하고, focus/style/위치·수락된 입력 변경에서 visible 주기를 다시 시작합니다. unfocused cursor는 기존 hollow block을 유지합니다. scrollback·hidden cursor에는 blink repaint를 예약하지 않습니다. 화면 밖/숨은 view의 focus 수명은 별도 gate입니다.
- 초기 font size와 실시간 변경은 같은 Appearance에 반영하며 terminal 전용 family를 유지합니다. 측정된 glyph/row metrics를 기존 fit/resize 경계가 사용합니다. settings 서비스의 기존 font size sanitize 정책은 바꾸지 않았습니다. multi-window controlling view·unfocused resize 동등성은 아직 미완료입니다.

## 폰트 소유·입력

- 기존 SVG worker가 쓰던 system fontdb OnceLock을 `system_fonts`로 옮겨 두 실제 소비자가 한 process cache를 공유합니다. 새 스캔을 추가하거나 UI frame에서 파일을 읽지 않습니다. 초기 scan/prepare는 기존 supervised presentation worker, 변경은 단일 pending request와 supervised blocking result worker가 수행합니다. 변경 중 더 새 설정은 coalesce하며 오래된 결과를 적용하지 않습니다. 취소 중인 worker의 slot은 결과/closed reply까지 유지합니다. 종료 이후 새 입장은 하지 않습니다.
- family는 **복제 전에** 512바이트·control 문자 경계를 확인해 Default/Named/Invalid typed request로 좁힙니다. requested family→platform ui-monospace 후보→SFMono-Regular→Menlo→Apple SD Gothic Neo→generic monospace→egui 기본 monospace 순서입니다. family 대소문자·PostScript 이름을 확인하고 face ID를 중복 적재하지 않습니다. platform ui-monospace 후보는 browser OS font resolver와 완전히 같은 결과라고 주장하지 않습니다.
- 새 준비의 font bytes는 파일당 64MiB·합계 128MiB 이하입니다. 파일 metadata/regular type·실제 read 길이·추가 byte를 검사하고, Unix는 NONBLOCK/NOFOLLOW로 열어 FIFO와 최종 symlink를 거절합니다. Binary도 복제 전 길이와 복제 뒤 capacity를 확인합니다. `FontData.index`에 실제 TTC face index를 보존합니다. 거절/없음에는 redacted 경고와 fallback이 있습니다. 이 cap 때문에 큰 TTC/사용자 font가 거절될 수 있으며 제품 full parity gate에서 확인해야 합니다.
- egui의 same parser인 `skrifa::FontRef::from_index`로 GUI 등록 전 검증합니다. 기존 renderer graph에 있던 `skrifa 0.44.0` 직접 edge와 app lock entry만 추가했습니다. 기존 default features는 epaint의 std/autohint_shaping과 같습니다. 새 package/version·root dependency/lock·MSRV는 이 변경에서 추가/갱신하지 않았습니다. license는 기존 MIT OR Apache-2.0입니다. system font 파일을 제품 asset으로 복사/배포하지 않습니다.
- 전용 FontFamily만 등록하고 기본 UI/editor family chain은 보존합니다. 한 prepared payload의 byte cap을 fontdb metadata·이전/현재 준비·egui font cache/atlas/GPU·전체 RSS의 합계 상한으로 주장하지 않습니다. glyph별 paint·bold/italic face·shaping/CJK fallback 전체 fidelity도 미완료입니다. [egui set_fonts](https://docs.rs/egui/0.36.2/egui/struct.Context.html#method.set_fonts), [fontdb query/face source](https://docs.rs/fontdb/0.23.0/fontdb/struct.Database.html#method.query)와 설치된 skrifa/read-fonts/epaint source를 확인했습니다.

## 실제 검증

Cargo는 직렬, 기존 `CARGO_HOME`, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. direct edge 최초 resolve만 offline unlocked check를 실행했으며 이후 locked를 유지했습니다. 합성 in-memory font/고유 임시 fixture·실제 자기 PTY와 headless egui만 사용했습니다. 사용자 font 전수 scan/보호 앱·bundle·OS 설정/clipboard·입력기/VoiceOver를 실행·변경하지 않았습니다.

1. [x] `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --test cursor -- --nocapture`: 최종 1 PASS(compile 1.14초/suite 0.02초). 실제 SharedTerminal/Core의 live default·DECSCUSR/DEC12/query 우선순위·동일 설정 재적용 거절·grid/history/mode/revision 유지·hidden/RIS·실제 quota retire 뒤 거절입니다. 최초 private retire 접근 compile 실패와 private grid cache clone 비교 panic(1001/0)은 테스트 가정 오류였으며 실제 quota 폐기·공개 logical cells 비교로 교정했습니다. query option 응답을 xterm source에 맞춘 실제 변경 이후 해당 검사만 다시 실행했습니다.
2. [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib cursor_blink -- --nocapture`: 1 PASS(compile 3.90초/suite 0.00초). 600ms의 서로 다른 시각·focus/style/위치·비유한/역행 시각, 크기 변경과 family 보존을 검사했습니다. OS pixels·전체 실제 window focus 시험은 아닙니다.
3. [x] `cargo test … --lib terminal_fonts -- --nocapture`: 최종 1 PASS(compile 3.96초/suite 0.03초). 합성 regular/TTC index 1·charmap·실제 egui font parse/mesh, family/PostScript alias, 기본 UI chain 유지, File/Binary/byte budget/invalid index/data·fallback, typed invalid family, cancelled reply 적용 거절·닫힌 supervisor 새 입장 거절·tracked 0입니다. 완료 reply는 test port로 주입했으며 실제 system scan·font worker 전체 shutdown/race·사용자 font 픽셀을 실행한 증거는 아닙니다. 복제 전 typed family 경계 변경 뒤 영향 검사만 1회 재실행했습니다.
4. [x] `cargo test … --test terminal-host native_cursor_settings -- --nocapture`: 신규 1 PASS(compile 10.62초/suite 0.36초). 첫 ready의 기본 Bar/blink, 살아 있는 실제 PTY의 settings 변경·동일 변경 dedup·grid/history/cursor/revision 유지와 close/join/tracked 0입니다. 초기 설정의 spawn 전 순서는 실제 create_session source와 함께 확인했으며 OS 최초 pixel 시각은 측정하지 않았습니다. 기존 startup/headless/resize/queue/writer/query 성공 runtime은 반복하지 않았습니다.
5. [x] native `clippy … --lib --test cursor -- -D warnings`: exit 0(0.90초). app `clippy … --lib --test terminal-host -- -D warnings`: 최종 exit 0(2.11초), typed 요청 변경 전 결과는 exit 0(4.88초)입니다. exact authored rustfmt·`git diff --check` exit 0이며 vendor 전체 재포맷은 하지 않았습니다. Wry의 기존 dependency warnings 17개는 유지합니다.

## 실패 정정·남은 gate

app 최초 check는 fontdb의 실제 feature graph에 없는 SharedFile variant 사용으로 실패했고 실제 File/Binary만 처리했습니다. 폰트 test는 설치된 epaint `has_glyph`의 replacement face identity 비교가 실제 문자 map과 달라 false가 된 것을 확인했으며 charmap과 실제 font parse/mesh를 검사합니다. 또 headless output의 unapplied texture delta Drop panic을 정상 API `textures_delta.clear()`로 처리했습니다. GUI renderer의 texture 처리나 검사기를 비활성화하지 않았습니다. 첫 typed 요청 patch는 rustfmt 이후 문맥 불일치로 거절돼 실제 source를 읽고 재작성했습니다. 별개 실패 원인을 같은 가정의 반복 성공/성능 표본으로 세지 않습니다.

Font worker 실제 cancel/close 재개·동시 settings 변경 전체 수명, aggregate DB/cache/atlas/RSS·OS fonts/동적 설치·CJK/ligature/Arabic/emoji·bold/italic·wide cursor/decorations·모든 cursor VT query, selection/copy/wrap/trim/search/link/context·hidden focus/multi-window·mouse/keypad/Kitty/IME/AX·project Hub·agent/remote/IDE/CLI, 213 TS view의 full parity·제품 cutover/배포가 남습니다. 보호 실기 bundle과 기존 제품/TS는 유지하며 M8 전체 완료로 계산하지 않습니다.
