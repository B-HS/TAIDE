# M8 native editor 첫 화면·입력 구현

## 대상과 계약

`native/taide-native-editor/src/editing.rs`·`tests/editing.rs`, `native/taide-native-ui/src/editor_surface.rs`·`tests/editor_surface.rs`를 구현했습니다. 기존 TypeScript `features/editor/code-editor.tsx`와 `widgets/editor-pane/editor-pane.tsx`의 단일 공유 본문·분할 화면별 선택/스크롤·preedit/본문 구분을 기준으로 사용합니다. 전체 옵션·기능을 아직 재현하지 않았으며 이 문서는 완료표가 아닌 실제 구현 증거입니다.

기존 egui 0.36.2의 고정 source에서 Event/IMEOutput·focus ownership/EventFilter·Galley cursor hit-test API를 확인했습니다. unicode-segmentation 1.13.3은 이미 egui 그래프에 있는 동일 버전을 editor의 직접 edge로 사용합니다. [GraphemeCursor 공식 API](https://docs.rs/unicode-segmentation/1.13.3/unicode_segmentation/struct.GraphemeCursor.html)의 PreContext/PrevChunk/NextChunk 계약을 Rope chunk에 적용합니다. 제품 root manifest/lock·MSRV와 기존 앱/bundle은 이번 변경으로 바꾸지 않습니다.

## 구현된 경계

- NativeEditor는 caller가 소유하는 EditorStore/ViewId를 직접 사용합니다. 지속적인 String 본문이나 별도 undo/IME 모델을 만들지 않습니다. visible logical line의 transient String/Galley만 생성하고 전체 문서 String은 frame마다 만들지 않습니다.
- 줄 번호·현재 줄·커서·여러 선택·pointer hit-test/drag와 화면별 scroll을 그립니다. 변경/이동 시 커서가 있는 줄을 세로로 드러내고 문서가 줄어들면 scroll 범위를 복구합니다. 글꼴·line height·색·indent는 caller 입력이며 theme/settings 자동 매핑은 host 연결 항목입니다.
- text/paste·선택 copy/cut·기본 탐색/선택 확장·grapheme 삭제·CRLF/UTF-8 경계·여러 선택 replacement를 기존 transaction/undo에 연결합니다. save shortcut은 요청만 반환하며 실제 쓰기를 성공 처리하지 않습니다. copy 본문은 사용자 copy 이벤트 시에만 materialize합니다.
- grapheme 탐색은 Rope chunk를 직접 공급하므로 결합 문자·국기·CRLF 탐색을 위해 전체 본문을 복사하지 않습니다. 중복/겹친 선택은 합치되 인접 범위는 별개의 replacement로 유지합니다. primary selection identity를 새 caret으로 연결합니다.
- focus를 가진 view만 해당 입력을 소비하고 미지원 키는 남깁니다. caller의 명시적 focus 요청과 클릭/drag만 focus를 획득합니다. 자체 문서 상태로 preedit을 유지하며 commit 때만 본문을 바꿉니다. 문서가 외부 transaction으로 바뀐 뒤의 stale preedit/commit은 거절합니다.
- 기본 IME 후보 위치를 현재 caret의 global rect로 출력합니다. read-only 문서의 편집 입력은 본문/undo를 바꾸지 않고 typed error로 반환합니다. 오류 뒤에도 기존 문서를 렌더합니다.

## 실제 검증

모든 Cargo 명령은 `--target-dir experiments/terminal-core-spike/target --offline`을 사용했습니다. lock 생성이 끝난 뒤에는 `--locked`도 사용합니다.

- [x] editor core `--test editing`: 첫 시도는 새 test fixture의 open_untitled language_id 누락으로 compile exit 101이었습니다. 실제 API에 맞춰 수정한 단일 실행에서 2 passed/0 failed, 0.00초입니다. Rope chunk 경계의 결합 문자·국기·CRLF/빈 마지막 줄, 선택 replacement·undo·여러 caret과 원자적 boundary 거절을 확인했습니다.
- [x] native UI `--test editor_surface`: 첫 compile에서 Galley cursor의 CharIndex를 usize로 다루던 타입 오류를 확인해 공개 `.0` 필드를 사용하고 unused import를 제거했습니다. 이어 2 passed/0 failed, 0.04초입니다. 실제 egui frame이 shapes를 생성하고 text→preedit→commit→undo→save intent→외부 edit→stale commit 거절을 수행했습니다. 50,000행에서 보이는 줄만 요청하고 동일 문서의 auxiliary view scroll은 독립적임을 확인했습니다.
- [x] 추가된 UI `--test editor_surface 커서_이동`: 1 passed/0 failed, 0.03초입니다. 문서 끝 이동 후 화면이 해당 줄로 이동하고 전체 선택을 짧은 본문으로 바꾸면 scroll 0/한 줄로 복구됩니다. 앞서 성공한 다른 UI 검사는 반복하지 않았습니다.
- [x] 인접 선택을 별개로 유지하고 primary를 정확히 선택하는 변경 뒤 영향 core `--test editing 다중_선택`만 재검사: 1 passed/0 failed, 0.00초입니다. unchanged grapheme 검사는 재사용합니다.
- [x] core `cargo clippy --lib --test editing -- -D warnings`: 최종 exit 0, 0.22초입니다. UI `cargo clippy --lib --test editor_surface -- -D warnings`: 초기 identical branch 경고를 구현에서 합친 뒤 exit 0, 0.40초입니다. 검사기 허용/끄기와 성공 결과 반복은 사용하지 않았습니다.

frame 검사는 창/GPU 없이 egui의 실제 layout·interaction·paint command를 실행한 것입니다. OS 표시 픽셀·OS clipboard·실제 IME/VoiceOver 성공 증거가 아닙니다. 기존 shell/controller/store/admission의 unchanged 성공 증거는 재사용하며 사용자 파일/앱·입력기/VoiceOver·실기 bundle을 조작하지 않습니다.

## 남은 구현과 검증

후속 `native/taide-native-app`의 최초 file-tab host와 실제 저장 연결이 추가됐습니다. 범위·검증은 `2026-09-30-m8-native-application-host.md`를 따르며 아래 전체 host/view 동등성 항목을 완료로 바꾸지 않습니다.

- [ ] 실제 native executable/AppServices·shell tab/editor owner 연결, file/untitled/app-file/diff host·dirty state·save/hot-exit·document close 수명
- [ ] COSMIC 수준 shaping·font fallback/ligature·bidi·word wrap/minimap/sticky scroll·syntax·LSP/AI/Git/snippet/Emmet·전체 editor 옵션/키맵과 typing undo grouping
- [ ] 전체 grapheme affinity/단어·수직 목표 column·horizontal caret reveal·page 이동·indent/outdent·drag autoscroll·빈 선택 copy/cut line·escape/focus 복원
- [ ] preedit active range·선택 replacement의 정확한 가시 projection·IME 이벤트 순서/취소·AX text runs/선택 actions·context menu·실제 CJK 입력/VoiceOver
- [ ] 긴 단일 행의 transient Galley/String 메모리·shaping latency와 전체 quota, 실제 native 픽셀/성능/배포·213 TS view 누락 0

현재 viewport는 logical line별로 줄 전체를 layout합니다. 긴 단일 행은 전체 문서 사본을 매 frame 만드는 방식은 아니지만 해당 한 줄의 String/Galley 비용이 크므로 대형 단일 행 제품 gate를 충족했다고 주장하지 않습니다. epaint의 기본 글꼴/글리프 layout을 최종 shaping 동등성으로 대체하지 않습니다. N2/N3와 M8 전체는 미완료입니다.
