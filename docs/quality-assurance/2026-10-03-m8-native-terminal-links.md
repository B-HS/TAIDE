# M8 native terminal 링크 검출·외부 URL 기본 연결

## 범위와 대상

N4-C의 같은 terminal Core grid에서 file 후보·URL·OSC8을 검출하고 URL을 native surface→typed bounded Host→기존 승인 platform으로 연결한 최초 기록입니다. 후속 파일 승인·cache·탭 열기·좌표 이동 기본 연결과 신규 검사 결과는 `2026-10-03-m8-native-terminal-file-links.md`가 정본입니다. OSC133·N4-C 상위·M8 전체 완료가 아닙니다.

대상은 `native/taide-native-app/src/{terminal_links,terminal_surface,host,application,lib}.rs`, `tests/{terminal-links,terminal-host}.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`, `native/taide-native-app/LICENSE-XTERM-LINKS`입니다. Cargo manifest/lock·제품 TS·root/MSRV·보호 bundle은 이 단위에서 변경하지 않았습니다. 기존 dirty/untracked 작업을 유지했고 서브에이전트·OS browser·실제 clipboard·입력기·사용자 파일을 사용하지 않았습니다.

## 원본과 API 근거

- TS `terminal-link.ts`의 ASCII path와 longest coordinate suffix 6종, `terminal-file-link.ts`의 물리 행·wide/UTF-16 cell mapping·256행 FIFO·null cache·16개 승인 후보 경계를 대조했습니다. 실제 shared Rust resolver는 canonical/open project 승인만 하며 `is_file`을 추가 검사하지 않는 것을 확인했습니다. 주석의 “real file”을 새로운 계약으로 사용하지 않습니다.
- `terminal-view.tsx`의 실제 gate는 Alt 또는 macOS Meta/non-macOS Ctrl입니다. Alt를 거절하는 새 정책을 만들지 않았습니다.
- 설치된 `@xterm/addon-web-links` 0.12.0의 `WebLinksAddon.ts` strict regex와 `WebLinkProvider.ts`의 UTF-16 2048-unit 상·하 wrap 확장·공백 종료·URL parsed-base 검사를 확인했습니다. `@xterm/xterm`의 `OscLinkProvider.ts`와 `Linkifier.ts`는 OSC8 우선·hover 행에서 겹친 낮은 provider의 전체 범위 제외·down/up의 URI/range 값 동등성을 사용합니다. 고정 source와 MIT notices는 `LICENSE-XTERM-LINKS`에 연결합니다.
- 설치된 regex 1.13.1 `Regex::captures_at/find_iter`, url 2.5.8 `Url::parse/authority`, egui 0.36.2 `Response::contains_pointer`, `Ui::disable`, local `HostBridge::disconnect`와 기존 `system_open_external_url`의 실제 source/API 문서를 읽었습니다. 새 dependency나 second terminal parser를 추가하지 않았습니다.

## 구현

1. 같은 Core의 cell과 combining glyph를 byte-span→cell range로 매핑합니다. 넓은 문자는 continuation을 건너뛰고, NFD/astral 앞글자 때문에 ASCII 파일 범위가 밀리지 않습니다. Rust Unicode `\w/\d/\s`를 JS의 ASCII word/digit·ECMAScript whitespace와 동일한 명시적 class로 옮겼습니다. suffix의 숫자는 원본 Number와 같은 `f64` 값이며 실제 editor 이동의 유효성은 후속 경계입니다.
2. File 후보는 한 물리 행과 shared resolver의 첫 16개만 검출합니다. URL만 wrap 행을 이어 읽고 원본 종결 구두점·uppercase scheme·parsed-base 유효성을 사용합니다. 유효 OSC8은 같은 grid의 ID/URI와 물리 행 범위를 우선하며, 같은 hover 행에서 겹친 낮은 URL 전체를 제외합니다. invalid/non-http OSC8은 provider에서 제외됩니다.
3. Native surface는 hover hand/foreground underline과 이벤트별 실제 press/release 위치를 사용합니다. Alt/플랫폼 Mod를 release 시 평가하고 down/up의 URI/range가 같은 경우만 전송합니다. 다른 링크로 release·disabled UI·menu·숨김/닫힘/세션 교체·취소는 잘못된 activation을 만들지 않습니다. 단순 output revision 증가를 클릭 거절 근거로 사용하지 않습니다.
4. URL 요청은 opaque Weak mount owner와 project/pane/tab/session source를 캡처합니다. 기존 64개 bounded Host queue의 tracked blocking worker가 owner·active layout·열린 project·현재 Hub/Running session을 다시 검사한 뒤 기존 external URL whitelist/제어 문자/userinfo 거절과 platform을 호출합니다. 탐색기 `OpenInBrowser`는 승인 파일을 `file://`로 여는 별도 동작이므로 재사용하지 않았습니다. 실패 표시는 기존 `terminal.openLinkFailed` 키를 사용합니다.
5. Row/URI는 64KiB, URL window는 256KiB 상한입니다. 할당 전 text 증가와 window append를 검사하지만 String/Vec·URI clone·regex·grid·다중 창의 aggregate/peak/RSS 증명은 아닙니다. TS의 초대형 한 행/Unicode URL에는 이 새로운 상한의 호환성 검토가 남습니다. 후속 canvas 밖 hit 제외와 link 오류의 paint 보존은 코드·strict 검사만 확인했고 실기/회귀 완료로 세지 않습니다.

## 실제 검증과 정정

모든 Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`·`--locked --offline`·`--target-dir experiments/native-shell-spike/target`으로 직렬 실행했습니다. 보호 실기 app은 빌드/교체/재시작하지 않았습니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test terminal-links` — 기본 4 PASS, suite 0.01초·compile 2.32초. path suffix 6종/ASCII/wide/NFD/astral, 한 행 file 범위, wrapped URL/IPv6/구두점/uppercase/invalid base, OSC8/수정/reset, modifier/16후보를 검사했습니다. 관련 입력이 불변인 이 결과를 재사용합니다.
- [x] `--test terminal-links 같은_물리행` — 낮은 URL overlap이 `Some`으로 남는 RED를 재현하고 hover 행의 유효 OSC8 범위와 겹친 전체 URL을 제외했습니다. 관련 1 PASS, suite 0.01초·compile 5.03초이며 기본 4건을 반복하지 않았습니다.
- [x] `--test terminal-host terminal_links는` — actual synthetic PTY와 실제 egui hover/press/release·Alt/Mod·disabled·두 종류 URL·다른 링크 release·mock platform·승인 거절·active None·hidden Weak·continue/exit 0·Hub/host join·task 0, 최종 1 PASS, suite 0.53초·compile 4.66초입니다. 실제 OS browser/clipboard는 호출하지 않았습니다.
- [x] 같은 통합 fixture에 같은 프레임의 다른 위치 press/release를 추가한 RED에서 잘못된 URL command가 생성되는 것을 확인했습니다(suite 0.19초). 최종 위치를 모든 이벤트에 재사용하던 원인을 이벤트별 `pos`로 고쳤고 위 최종 1건만 재실행했습니다. 이전 성공 0.42초와 같은 입력의 반복 계측이 아닙니다.
- [x] `cargo clippy ... --lib --test terminal-links --test terminal-host -- -D warnings` exit 0, 2.66초. 후속 canvas/link-error paint 보존 guard의 lib strict exit 0, 1.93초입니다. Wry의 기존 17개 dependency warning은 새 authored warning으로 세지 않습니다. Authored 8개 Rust 파일 exact fmt·tracked diff exit 0, QA/bug 3개 Prettier exit 0이며 untracked authored/license/QA 12개 no-index whitespace 출력은 비어 있습니다(exit 1은 /dev/null과 내용 차이).

초기 fixture는 함수/변수 `core` shadowing으로 E0618, 기존 JS regex에서 literal `[`를 Rust에서 escape하지 않아 unclosed character class, uppercase test 함수의 snake-case warning을 보였습니다. 함수명·regex translation·test 식별자를 수정했고 URL endpoint fixture를 실제 27 ASCII glyph+5 prefix columns에 맞췄습니다. 최초 host fixture의 없는 `Ui::set_enabled`·`HostBridge::close`와 closure bool 추론 오류는 고정 source의 `disable`·`disconnect`·필수 parameter type으로 수정했습니다. 실패를 통과로 세거나 검사기를 끄지 않았습니다.

## 남은 필수 작업과 테스트 부채

- [ ] File 기본 연결은 후속 `2026-10-03-m8-native-terminal-file-links.md`에 기록했습니다. 실제 OSC7 교체·AppSurfaces loading/복원/focus frame·queue-full/remount/보조 창·IME/전체 파일 수명은 해당 QA의 남은 경계로 유지합니다.
- [ ] OSC133 같은 parser/cursor marker·500 block·빈 프롬프트·history/resize/clear/alternate·gutter/이전·다음 이동을 구현합니다. 다른 parser/after-chunk final cursor 추측으로 대체하지 않습니다.
- [ ] Canvas 밖/최대 row/error paint guard·same-frame button/move/output 혼합·multi-button·raw mouse mode·보조 창/초점/닫힘 matrix와 원본 xterm provider의 특수 빈 cell/wide-wrap 세부를 확인합니다. 기본 합성 위험과 독립적인 전체 GUI matrix는 UI 완결 뒤 검사합니다.
- [ ] 64KiB row/URI의 TS 호환성과 byte budget·row spans/regex cache/다중 창 aggregate·대형 burst CPU/GPU/RSS를 검증합니다. 구조적 상한을 실제 메모리/성능 gate로 주장하지 않습니다.
- [ ] 실제 OS external browser/URL 실패 locale 3종·theme/DPI/픽셀/AX·CJK IME·전체 VT와 213 view/full cutover/TS 제거·M8 N1~N8을 마무리합니다. 보호 bundle·사용자 데이터는 유지합니다. 전체 M8 완료 뒤만 선별 commit·일반 push합니다.
