# M8 native OSC133 명령 블록 기본 연결

## 대상 파일과 원본 근거

- `src/features/terminal/terminal-osc133.ts`: A/B/C/D reducer, 500개 상한, C 사용 여부 latch, 빈 prompt 폐기, 정수 exit code, 완료 시 색 선택, 2px gutter, 엄격 이전/다음 행 이동입니다.
- `src/shared/lib/keymap/keymap.ts`: terminalFocus의 Mod+ArrowUp/Down 기본 binding입니다. 사용자 override와 전체 palette는 이번 기본 binding 연결의 완료 범위가 아닙니다.
- 설치된 xterm 6.0.0의 `Buffer.ts`, `InputHandler.ts`, `CoreBrowserTerminal.ts`, `BufferDecorationRenderer.ts`, `OverviewRulerRenderer.ts`와 `ColorZoneStore.ts`를 읽었습니다. [공식 marker API](https://xtermjs.org/docs/api/terminal/interfaces/imarker/), [Terminal API](https://xtermjs.org/docs/api/terminal/classes/terminal/)도 확인했습니다. 현재 문서 사이트보다 설치된 원본 코드가 제품 동작 기준입니다.
- native terminal의 vendor Row/Storage/Grid/Term과 새 `term/native_commands.rs`, 같은 vte의 `ansi.rs`, Core/SharedTerminal, app의 terminal dispatch/host/surface와 관련 검사·합성 fixture가 대상입니다. 제품 TS/root manifest/MSRV와 보호 실기 bundle은 변경하지 않았습니다.

## 구현과 확인 결과

- [x] 단일 vte parser가 완전한 OSC raw payload를 dispatch하는 시점에 native 전용 Handler를 호출합니다. feed 끝의 cursor에서 marker를 추정하거나 두 번째 byte scanner를 추가하지 않습니다. C0·구분자를 보존한 raw 경계를 사용하므로 `1\t2`가 정수 12로 바뀌지 않습니다. reader의 명령 완료 notification side channel은 기존대로 유지합니다.
- [x] A/C/D marker는 실제 cursor 행의 공유 liveness anchor입니다. 서로 다른 block id로 같은 행의 500개 block도 구분합니다. 자연 행 이동은 Row 소유권을 따라가고 reset/drop·history cache shrink·삭제·완전 merge/overflow는 anchor를 폐기합니다. Row clone은 marker를 복제하지 않습니다. 삭제된 start block을 제거해 현재 id가 stale index로 바뀌지 않습니다.
- [x] 새 Row field는 native 전용입니다. native Storage swap은 전체 Row의 안전한 slice swap으로 변경했습니다. 원본의 고정 4-word unsafe swap은 feature-off에만 남습니다. 기존 unsafe를 늘리거나 marker field를 일부만 swap하지 않습니다. serde에서는 runtime marker를 건너뜁니다.
- [x] max 500 block, legacy C 없는 shell의 A/D 보존, C를 실제 관찰한 뒤 빈 prompt 폐기, stray/unknown/no-op·완전 BEL/ST·fragmented 입력을 연결했습니다. exit code는 JS Number의 decimal/exponent/양수 radix·공백·finite integer·큰 binary 값의 ties-to-even을 재현합니다. 최초 C latch는 RIS 뒤에도 유지합니다. 원본 tracker처럼 marker 정리와 shell 능력 관찰은 별개입니다.
- [x] D 시점의 성공/실패 RGB를 block에 보존합니다. null/잘못된 색이나 exit code는 decoration을 만들지 않습니다. theme mapping은 원본처럼 `#` 뒤 앞 6개 hex를 사용하고 alpha를 무시합니다. SharedTerminal spawn 이전에 초기 port 색을 설정하고 보이는 surface의 이후 설정은 미래 완료에만 적용합니다.
- [x] 실제 native paint의 두 pixel 왼쪽 gutter와 alternate buffer의 숨김을 연결했습니다. 표시 행만 scan하는 decoration lookup은 block 순서와 같은 행의 overwrite 순서를 유지합니다. Mod+Up/Down은 현재 view의 viewport 절대 행보다 작은 최대/큰 최소 start를 찾고 history 상한으로 clamp합니다. 이동은 Core display_offset·input epoch·PTY 입력을 변경하지 않습니다.

## 실행 증거

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`에서 직렬 실행했습니다. 같은 상태의 성공을 반복하지 않았습니다.

1. native terminal `--test commands`: 신규 parser/cap와 Number 2 PASS를 최초 suite에서 확인했습니다(전체 최초 suite 0.10초). 수명 검사는 당시 실패했습니다. 아래 수정 후 해당 수명 filter만 1 PASS(0.03초), 새 completion color/range/alt 검사만 1 PASS(0.00초)입니다. 동일 명령의 전체 재실행 3회를 통과 근거로 삼지 않습니다.
2. app `--lib command_gutter`: 신규 1 PASS(0.01초). 실제 egui Rect shape의 폭 2px·행 높이·success/failure·alternate 숨김/복원을 확인했습니다. texture delta를 먼저 회수한 뒤 assertion합니다.
3. app `--test terminal-host terminal_commands`: 신규 1 PASS(0.41초). 실제 합성 PTY의 40개 command, 실제 Views raw Mod+Up 두 번/Down·disabled frame·gutter·input epoch 불변·`continue` 뒤 exit Some(0)·dispatch/Hub join·TaskSupervisor 0을 확인했습니다. 실제 OS 키보드나 제품 창 실측은 아닙니다.
4. Row/retained/same-parser 영향 검사: native `--test core --test retained` 2 PASS(0.01초)+3 PASS(0.00초), 기존 clear의 Row clone/행 보존 영향 검사 `--test clear` 2 PASS(0.01초)입니다.
5. 정적 검사: native lib/commands/core/retained strict clippy exit 0(1.31초), app lib/terminal-host/terminal-dispatch/fixture strict clippy exit 0(3.87초)입니다. 기존 Wry 의존 경고 17개는 authored 검사 성공과 구분하며 suppression을 추가하지 않았습니다. app check도 exit 0(3.27초)입니다.

authored Rust 10개 파일의 exact rustfmt check, QA/bug 두 파일 Prettier, tracked diff check는 exit 0입니다. 새 코드·vendor·provenance·QA 19개 명시 경로의 no-index whitespace 출력은 모두 비어 있습니다(exit 1은 /dev/null과 내용 차이). 보존된 vendor 전체를 일괄 재포맷하지 않았습니다. 검사 뒤 실행 중인 Cargo/도구 세션은 없습니다.

## 실패와 정정

- 최초 marker 수명 검사의 DL 뒤 block 수는 2였고 기대값은 1이었습니다. Alacritty의 cursor가 첫 행인 delete-lines가 full-grid scroll-up을 재사용해 삭제된 행을 history에 남겼습니다. native delete-lines를 history를 만들지 않는 제한 region swap/reset으로 수정했습니다. native IL/DL은 제품 xterm처럼 column 0·pending wrap 해제도 적용하며 feature-off handler는 보존합니다.
- 다음 수명 실패는 narrow resize의 start가 `Line(1)`인데 fixture가 `Line(2)`를 기대한 것입니다. native grid는 history 1을 추가했으므로 절대 buffer 행은 2입니다. 설치된 실제 xterm Terminal에 같은 입력을 write해 marker 절대 행 `[0,1] → [0,2] → [0,1]`을 관찰했고 fixture의 비교 경계를 절대 buffer 행으로 수정했습니다. native와 xterm의 viewport/baseY 동등성을 이 수정으로 주장하지 않습니다. Bun 도구 세션은 결과 뒤 내부 timer 때문에 남아 도구 자체 세션만 Ctrl-C로 종료했습니다. 보호 앱과 무관합니다.
- E0503 mutable grid/cursor 동시 borrow는 cursor 행을 먼저 복사해 수정했습니다. 검사 코드에서 GridDimensions import 누락, Appearance 새 field 누락, PaneNode `active` 대신 잘못 쓴 field, `Session::input`/`NativeInput::CommittedText` API 오류·Number 비-snake-case 이름을 실제 정의에 맞춰 정정했습니다. 구현 완료나 실패 재현으로 세지 않는 정적 fixture 오류입니다.
- patch 검증 실패 두 건은 잘못된 signature/format match였으며 원자적 apply 실패로 변경되지 않았습니다. 현재 파일을 확인한 뒤 올바른 patch만 적용했습니다.

## 미완료 경계와 다음 작업

- [x] 후속 기본 14px overview ruler·right zone의 DPR/6~12px 높이·padding merge/border를 연결했습니다. 새 geometry/실제 paint/fit 3건과 영향 gutter/Core/actual host 3건 PASS입니다. 전체 async overlap·최종 픽셀·scrollbar 입력은 미완료이며 `2026-10-03-m8-native-terminal-ruler.md`가 최신 근거입니다.
- [ ] Core의 persistent tracker와 원본 view unmount/replay tracker의 차이를 해소합니다. 숨은 command·remount·새 theme·replay 잘림과 primary/alternate marker를 섞는 원본 command-line 목록의 정확한 정책이 남습니다. 기본 색 보존 검사가 remount 동등성을 증명하지 않습니다.
- [ ] keymap override/palette command·composition·focus/비활성·보조 창의 실제 소유권과 route를 전체 keymap에 통합합니다. 기본 Mod+arrow 검사는 전체 command dispatcher 완료가 아닙니다.
- [ ] 원본 xterm과 전체 reflow/cursor-line/중간 region 삭제/색 변경·resize viewport·RIS·shell 세대 matrix를 비교합니다. 현재 절대 marker 행 검사로 전체 VT·history/baseY 동등성을 주장하지 않습니다. marker와 selection·다른 기능의 실제 내용 수명도 남습니다.
- [ ] Row field 자체는 native에서 행마다 pointer 크기를 추가하고 anchor는 lazy Arc allocation입니다. 500 block cap은 retained anchor 수 전체가 500이라는 뜻이 아닙니다. cap에서 제거된 block의 Row anchor는 행 reset/drop까지 남을 수 있으며 grid/history 규모 안에서 retained accounting됩니다. post-operation 논리 예산과 allocator peak/RSS·Core/clone/snapshot/동시 session aggregate는 별개입니다. 전체 snapshot/navigation은 grid 행 scan과 임시 map을 사용하며 50k 행 성능·UI lock hold는 N6에서 남습니다.
- [x] marker를 추가한 실제 path fork를 별도 feature-gate에서 off/no-serde·off/serde·on/serde로 compile·기본 parser/clone/swap·Row serde 왕복해 총 5건 PASS를 확인했습니다. registry Alacritty를 사용한 spike 성공과 구분합니다. 범위·lock/compile fixture 정정은 ruler QA에 기록했습니다.
- [ ] upstream corpus·선언 MSRV·전체 feature-off/serde runtime·실제 macOS/Windows/Linux CJK IME/AX/GUI·OS 창은 미검증입니다.

N4-C와 M8 N1~N8 0/8, 213 view 전환·TS 제거·성능/보안/배포는 계속 미완료입니다. 전체 M8 완료 뒤만 commit·일반 push합니다.
