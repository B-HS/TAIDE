# M8 native untitled 본문·Save As

## 대상과 원본

- 원본은 `src/widgets/editor-pane/untitled-pane.tsx`, `src/features/tab/{tab-bar-add-menu,tab-bar-menu-items}.tsx`/`.ts`, `src/widgets/editor-area/pane-tab-bar.tsx`입니다. 기존 `layout_open_untitled`와 `convert_untitled_to_file`의 번호 재사용·같은 pane 중복 합류를 재사용합니다.
- 구현은 `native/taide-native-app/src/{untitled,application,host}.rs`, `native/taide-native-editor/src/store.rs`, `native/taide-native-ui/src/{commands,shell}.rs`입니다.
- 원본에는 없는 welcome 새 파일 버튼을 추가하지 않습니다. 실제 원본의 탭 바 추가 메뉴(새 파일→터미널 순서, 24px trigger/14px plus)와 기존 메시지 카탈로그를 사용합니다. `COMMAND+N`의 새 untitled 생성도 연결했습니다. 전체 사용자 keymap·탭 바 빈 공간 double-click/context menu는 아직 남습니다.
- 새 의존성·root MSRV/제품 manifest 변경 없이 구현했습니다. 기존 실기 bundle·OS 입력기·VoiceOver·키체인을 변경하거나 GUI 앱을 실행하지 않았습니다.

## 구현

1. 새 untitled 생성과 실제 plaintext editor를 연결합니다. root의 살아 있는 untitled 탭을 검증한 worker가 mirror를 조회하고 mutation guard와 operation lease를 GUI admission까지 소유합니다. 신규 빈 문서는 clean이고, 복원한 빈 mirror도 원본처럼 미저장으로 표시합니다. 기존 live 문서는 이전 mirror로 덮어쓰지 않습니다.
2. 편집기의 저장 요청과 닫기 확인의 저장 버튼은 OS Save As로 연결합니다. 기본 파일명은 원본의 `Untitled-<index>`이며 대화상자 취소는 문서·탭·mirror를 유지합니다. 대상 경로를 프로젝트 경계로 재확인하고 dirty 대상 문서/탭과 대상 mirror를 먼저 해결하도록 거절합니다.
3. 저장 worker는 immutable snapshot을 atomic write하고 원래 mirror를 유지합니다. staged layout의 변환과 canonical document/view 합류는 guard를 소유한 reply를 GUI가 승인할 때 함께 적용합니다. 승인 전 reply 폐기나 shutdown은 원래 untitled layout/document/mirror를 보존합니다. 이미 작성한 destination 파일 자체를 삭제하거나 되돌리지는 않습니다.
4. 동일 canonical clean 파일이 있으면 문서가 두 개가 되지 않도록 합류합니다. 원본 untitled의 body/undo와 저장 snapshot의 baseline을 유지하고, 기존 대상 문서의 모든 view를 같은 canonical 문서로 옮깁니다. 같은 pane 기존 파일 탭에 합류할 때 중복 view를 정리합니다. 저장 이후 새로 들어온 편집은 dirty로 유지합니다.
5. 실제 전환 후 old untitled mirror cleanup을 별도 worker로 수행합니다. 비교한 mirror와 현재 mirror가 달라졌거나 같은 ID의 untitled가 아직 살아 있으면 지우지 않습니다. 닫기 폐기/clean 회수와 정상 종료의 untitled mirror 쓰기도 연결했습니다. cleanup 실패/queue 포화 시 mirror가 남으며 데이터 삭제로 처리하지 않습니다.

## 변경 위험별 성공 증거

공통 인자는 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 같은 성공 검사를 반복하지 않았습니다.

- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test untitled`: 2건 통과, 0.00초입니다. 빈 mirror의 dirty/용량/live body, dirty 대상 거절, clean canonical 합류, view 중복 정리·공유 보존, 늦은 편집과 새 baseline의 undo를 확인했습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test untitled`: 3건 통과, 0.07초입니다. 실제 host 새 탭/복원/편집/저장/guard 전달/cleanup, root의 같은 pane 합류와 late edit/new mirror, 경계·write 실패·dirty 대상/mirror·reply drop·shutdown을 확인했습니다.
- [x] 최종 app `--lib --bin taide-native-app --test untitled --test missing_draft` strict clippy: exit 0, 0.57초입니다. 추가 메뉴와 CLI testcase 이름이 변경된 최종 상태이며 이전 동일 성공을 반복한 것이 아닙니다.
- [x] editor `--lib --test untitled` strict clippy: exit 0, 0.17초입니다.
- [x] UI `--lib` strict clippy: exit 0, 0.37초입니다.
- [x] 변경 Rust 파일 rustfmt와 tracked diff whitespace check: exit 0입니다.

관련 CLI 외부 missing-source 분기는 신규 `missing_draft는_cli_승인_외부파일을_project_mirror로_변환하지_않는다` 1건이 0.02초 통과했습니다. 최초 실행의 대문자 CLI 함수명 경고를 소문자로 바꿨고 본문은 그대로이므로 동작 성공을 재사용했습니다. 이 검사를 untitled 저장 성공으로 중복 계산하지 않습니다.

## 남은 구현/실기 범위

- [ ] 실제 추가 메뉴 클릭·COMMAND+N 포커스와 Save As 대화상자의 취소/오류, 종료 후 실제 앱 재시작의 시각 동등성은 실행하지 않았습니다. 컴파일/단위 검사를 GUI 실기 통과로 주장하지 않습니다.
- [ ] 전체 conflict banner/View Disk/Keep Mine·autosave/hot-exit debounce/epoch와 모든 close/project/window 진입점은 남습니다.
- [ ] 전체 LSP/syntax/providers, 대형 본문 shaping/메모리, terminal/기타 surface와 최종 TS 제거·서명/배포는 남습니다.
- [ ] destination에 별도 dirty 초안이 있는 강제 덮어쓰기 확인 정책과 외부 filesystem TOCTOU, mirror JSON의 파싱 전 크기 제한, cleanup queue 포화의 재시도 UX는 남습니다.

이 결과는 N2-A2c의 첫 untitled 연결 범위이며 N2-A2c/N2/M8 전체 완료가 아닙니다. 사용자 실기 IME·VoiceOver와 fallback은 코드 구현 후 마지막 순서를 유지합니다.
