# M8 native 테마 draft와 편집 서비스 요청

## 대상 파일

- `native/taide-native-app/src/theme-draft.rs`, `theme-draft-tests.rs`
- `native/taide-native-app/src/theme-edit.rs`, `theme-edit-tests.rs`, `lib.rs`
- 원본 계약: `src/widgets/theme-editor/theme-editor.tsx`, `src/shared/lib/theme-draft.ts`, `src/entities/theme/theme-selection.ts`, `theme.query.ts`
- 공통 구현: `crates/taide-runtime/src/theme_actions.rs`, `settings_actions.rs`, `task_supervisor.rs`, `state.rs`, `crates/taide-theme/src/service.rs`

## 리포트

테마 편집기의 상태·저장 결과와 실제 supervised 서비스 요청을 Rust로 구현했습니다. Settings/ThemeEditor 화면이나 HostBridge 명령·응답에 아직 연결하지 않았으므로 UI/takeover/전체 M8 완료가 아닙니다. 이전 Settings 기본 화면6 PASS는 해당 화면/host를 수정하지 않아 재사용합니다.

## 구현 계약

1. create는 원본 resolved 테마를 복제하고 최초 loaded 상태를 따로 보존합니다. 아무것도 바꾸지 않은 복사본은 dirty가 아닙니다. name/current colors/syntax/terminal만 loaded와 비교하고 token count는 loaded가 아니라 base와 비교합니다. metadata는 UI 편집 항목에 포함하지 않습니다.
2. builtin/bundled 복제는 해당 builtin id를 base로, custom 편집/복제는 동일 type `taide-dark`/`taide-light`를 base로 사용합니다. 저장은 base와 달라진 token만 쓰며 palette는 비웁니다. raw TextMate 규칙이 base와 같으면 상속시키고 다르면 원본 규칙을 보존합니다. author/license/source도 보존합니다. preview DTO는 현재 값과 규칙을 유지하고 syntaxOverrides는 실제 base diff key만, warnings는 빈 목록입니다.
3. trim된 name·원본 허용색 `#RGB`/`#RRGGBB`/`#RRGGBBAA`와 대소문자 무관 transparent를 검사합니다. 입력 중 잘못된 색을 draft에 둘 수 있지만 build/save는 거절합니다. 모르는 token/reset은 원본 draft를 변경하지 않습니다. 식별자는 공통 safe-component를 적용합니다. slug는 ASCII 소문자/숫자와 단일 hyphen이며 빈 slug는 custom-theme입니다. 기존 id가 있으면 기존 UUID 생성기의4자리 hex suffix를 사용하고 목록에 없는지 재확인합니다. 원본 Math.random의4자리 base36과 난수 source/문자 분포는 같다고 주장하지 않으며, 새로운 RNG 의존성을 넣지 않았습니다. 저장 직전에도 create id 중복을 거절해 다른 테마를 덮어쓰지 않습니다.
4. Session은 Settings owner project/pane/tab과 고유 Arc ticket을 소유합니다. 요청은 private typed 데이터만 전달하고 load/save/delete admission에서 shutdown/활성 owner/ticket을 검사합니다. Session drop은 ticket을 폐기합니다. 새 session은 같은 owner/source라도 이전 응답을 소유하지 않습니다. 다른 source/mode draft를 저장 요청에 넣는 것도 거절합니다. 실제 응답의 pending/duplicate/late 소비는 향후 UI wiring에서 구현합니다.
5. load/save/delete의 theme I/O는 기존 TaskSupervisor blocking 경계를 사용합니다. save는 기존 owned mutation guard를 worker로 넘기고 성공 이벤트까지 worker 안에서 완료합니다. 호출 waiter가 사라져도 실제 worker가 lock을 잃지 않도록 했습니다. delete의 fallback→삭제 operation은 기존 nonabortable task lease로 유지하고 mutation lock을 잡습니다. 이 변경의 검사에서 호출 취소 중 실제 fsync/rename을 멈춘 것은 아니며 이미 commit을 시작한 쓰기를 취소/롤백한다고 주장하지 않습니다.
6. 활성 custom 테마 삭제는 실제 source type을 읽고 동일 type builtin으로 settings를 먼저 저장/공표합니다. 저장된 followSystemTheme는 그대로 둡니다. 명시 테마 상태에서만 fallback ThemeChanged를 추가합니다. settings 저장이 실패하면 테마 삭제를 실행하지 않습니다. 삭제 성공 후 모든 native consumer의 재해석을 위해 canonical settings.themeId의 ThemeChanged를 발행합니다. 원본 TS의 로컬 Query 무효화와 구현 통로가 다르며 native App/목록의 실제 갱신은 다음 gate입니다. 이 설정 변경은 themeId만 다루므로 기존 shared apply_and_broadcast와 no-op integration reconcile을 재사용합니다.

모델 맵은 기존 DTO의 BTreeMap을 재사용했습니다. 순서/iter/get/get_mut API는 공식 [BTreeMap 문서](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html), ASCII 판별은 공식 [char 문서](https://doc.rust-lang.org/std/primitive.char.html#method.is_ascii_alphanumeric)를 확인했습니다. 실제 고정 DTO·TaskSupervisor·mutation API는 설치된 프로젝트 source가 근거입니다. Rust의 fn/enum·소유권 mutation은 기존 native 구현의 언어 예외를 유지하며 TS 컨벤션을 근거로 잘못된 Rust 문법이나 임의 library를 만들지 않았습니다.

## 실제 검증 결과

Cargo는 한 번에 하나만 실행했습니다. 공통 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] `cargo test … --lib native_theme_draft -- --test-threads=1`: 2 PASS, suite0.43초/컴파일5.79초. 합성 import의 raw 규칙/출처 no-op round trip, 최소 diff/reset/dirty, partial syntax patch, unknown token 불변과 모든 builtin/bundled 복제의 실제 저장/상속 해석을 확인했습니다. 색상/잘못된 id/slug/충돌 suffix/builtin 편집/shutdown 거절도 포함합니다. 이후 source/mode provenance 필드와 비교 메서드만 추가했고 동일 행동 성공은 반복하지 않았습니다.
- [x] `cargo test … --lib native_theme_edit -- --test-threads=1`: 3 PASS, suite0.11초/컴파일5.71초. 실제 supervised load/save/custom edit·중복 create 거절·새 session과 폐기된 ticket·다른/hidden owner·mutation lock 대기 뒤 폐기 거절, active light 삭제의 follow false/true/비활성 분기·실제 settings 파일/이벤트 순서, settings 저장 실패의 원본 file/settings/no-event/no-temp 잔류를 확인했습니다.
- [x] `cargo clippy … --lib --bins --tests -- -D warnings`: exit0,12.79초. inherited Wry17 warnings는 기존 dependency에서 발생하며 authored 경고 억제/새 warning은 없습니다.
- [x] authored5파일 `rustfmt --check --edition 2024 --config skip_children=true` exit0, tracked PROCESS `git diff --check` exit0. 전체 workspace/vendor formatter를 실행하지 않았습니다.
- [x] 최신5문서 Prettier exit0, tracked PROCESS/HANDOFF diff check exit0입니다. 이번 slice authored8개·재개 프롬프트/신규 QA2개 등11파일의 no-index whitespace check는 모두 출력 없음/exit1(내용 차이만)로 오류가 없습니다.

첫 편집 서비스 fixture compile은 존재하지 않는 `AppPaths.settings_path`를2곳 사용해 E0599/exit101로 실패했습니다. 실제 `paths.rs`의 `settings_file`로 정정한 뒤 관련3건만 실행해 통과했습니다. 제품 코드를 바꾸거나 검사기를 억제하지 않았습니다. 합성 UUID temp 디렉터리는 각 검사 뒤 제거하며 사용자 데이터/보호 bundle/OS 입력/clipboard·IME·VoiceOver는 사용하지 않았습니다.

## 남은 게이트

후속 [ThemeEditor 검증](2026-10-03-m8-native-theme-editor.md)에서 실제 Settings/HostBridge/App 기본 renderer·요청별 identity/목록 generation·local/window-scoped preview 연결과 관련11건을 확인했습니다. 아래 초기 서비스 단위의 미연결 TODO는 역사 상태입니다. 기본 연결과 실제 NativeApplication 창·aux/픽셀/AX/포화/동시 worker의 전체 게이트를 구분합니다.

- [ ] 실제 HostBridge command/reply·pending/late/duplicate·App SettingsViews wiring, create/duplicate/edit/custom 목록과 save/delete/discard dialog. 현재 서비스 API만으로 사용자가 화면에서 편집할 수 있다고 표시하지 않습니다.
- [ ] 원본 ColorPicker HSV/hex blur·잘못된 입력 표시·keyboard/drag pointerup1회/취소/강제 닫힘, token namespace 순서/filter/reset·syntax style, local ThemeLivePreview와 앱 전체 preview frame coalescing·해제/동시창/원본 AX/pixel.
- [ ] catalog invalidation/외부 파일 변경·동시 remote theme 변경·custom 중복 생성 경합, 실제 write cancellation/close commit boundary, 멀티창/OS/메모리·CPU/GPU. 저장된 builtin1단 상속은 공통 runtime과 같으며 임의 custom 상속 chain을 새 기능으로 추가하지 않습니다.
- [ ] N1~N8 0/8·전체213view/41action/Monaco21/palette·cutover/TS제거/Rust99%·성능/보안/서명/배포/rollback. 별개 keybinding Tab RED와 PTY remount A/B는 해결되지 않았습니다. 전체 M8 완료 전 commit/push하지 않습니다.

branch `to_rust_native`, HEAD `2824005573ea1e1e6a70136499016b14aa06a2bf`를 재확인했습니다. 이 slice는 dependency/manifest/root/native lock/MSRV·제품 TS·vendor·보호 앱 bundle·OS 설정/Git 상태를 변경하지 않았으며 목표active입니다.
