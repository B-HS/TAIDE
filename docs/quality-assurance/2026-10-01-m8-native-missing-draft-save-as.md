# M8 native 삭제된 원본의 초안·Save As

## 대상과 원본

- 원본: `src/widgets/editor-pane/editor-pane.tsx`의 missing-source 읽기 전용 본문과 `handleSaveMissingSourceDraft`입니다.
- 구현: `native/taide-native-app/src/{missing_draft,host,application}.rs`, `native/taide-native-editor/src/store.rs`입니다.
- 새 의존성을 추가하거나 root MSRV·제품 실행 경로·기존 실기 bundle을 바꾸지 않았습니다. GUI 앱과 OS 저장 대화상자는 실행하지 않았습니다.

## 구현

- 실제 파일 열기가 실패했을 때만 승인된 프로젝트의 삭제된 원본 mirror를 복원합니다. 경로를 canonical로 재확인하고 모호한 중복 mirror와 거부 크기를 차단합니다. CLI 외부 파일은 project mirror를 만들지 않습니다.
- root mutation guard와 작업 lease를 GUI의 복원 승인까지 유지합니다. 버린 reply와 shutdown은 lease를 회수하며 mirror는 보존합니다.
- 초안은 편집기가 아닌 읽기 전용 본문으로 표시합니다. 원본 카탈로그의 `editor.sourceDeleted`·`editor.saveDraftAs`·`tab.saveAsTitle`을 재사용합니다. OS 대화상자 취소는 변경하지 않습니다.
- 저장은 worker에서 프로젝트 경계·현재 파일 탭·원본 부재·mirror 동일성을 재확인합니다. 미저장 destination 문서/탭과 destination mirror를 먼저 해결하도록 거절합니다. OS 선택을 CLI allowlist 확대로 취급하지 않습니다.
- 실제 atomic write·mode 보존·self-write 표시와 재열기가 성공한 뒤 원본 mirror를 지웁니다. 다른 표시 경로면 원본 탭을 유지하고 별도 파일 탭을 열며, 같은 표시 경로면 원본 파일을 재생성합니다.
- 이미 로드한 clean destination 문서는 canonical identity·공유 view를 유지한 채 새 디스크 본문으로 갱신합니다. dirty 문서는 덮어쓰지 않으며, 갱신 시 selection을 유효 UTF-8 경계로 제한하고 오래된 composition/fold/history를 지웁니다.
- 닫기 확인의 저장 버튼에도 같은 Save As를 연결했습니다. 저장/닫기 실패는 초안을 유지합니다.

## 단일 성공 증거

각 command는 공통으로 `--locked --offline --target-dir experiments/native-shell-spike/target`을 사용했습니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test missing_draft`: 3건 통과, 0.05초입니다. 실제 host의 다른 경로 저장, 같은 경로 재생성/token drop/shutdown, 경계·실패·dirty destination·destination mirror·원본 복구·mirror 변경·닫힌 layout의 보호를 확인했습니다.
- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test store clean_file_갱신`: 1건 통과, 0.00초입니다. 공유 view의 본문/selection 갱신, dirty/잘못된 identity 거절과 새 baseline의 undo를 확인했습니다.
- [x] app의 `--lib --bin taide-native-app --test missing_draft` strict clippy: exit 0, 0.59초입니다.
- [x] editor의 `--lib --test store` strict clippy: exit 0, 0.07초입니다. 단일 Range의 Vec 표기 경고만 테스트의 `std::iter::once`로 교정했으며 성공한 동작 검사를 반복하지 않았습니다.

첫 검사는 fixture가 기본 terminal을 파일 탭으로 선택해 2건 실패했습니다. 파일 kind/path로 찾도록 고친 뒤 실제 저장은 성공했지만 fixture가 mirror 저장 키를 비canonical로 만들어 cleanup assertion 2건이 실패했습니다. fixture를 실제 `file_mirror_dirty`/`file_clear_mirror` runtime 경로로 바꿔 최종 3건을 통과했습니다. 이 초기 결과를 성공이나 제품 버그 수정으로 계산하지 않습니다.

## 남은 범위

- [ ] 실제 OS 대화상자 취소·오류 및 화면 픽셀/키보드/접근성은 마지막 실기 순서에 남깁니다. GUI 코드는 컴파일됐지만 실제 실행 성공으로 주장하지 않습니다.
- [x] 후속 untitled 본문·mirror·Save As와 첫 canonical 합류는 `2026-10-01-m8-native-untitled-save-as.md`에 구현/검증 범위를 기록했습니다. 해당 첫 연결이 전체 N2/M8 완료는 아닙니다.
- [ ] 전체 conflict 배너/View Disk/Keep Mine, autosave와 모든 close 진입점은 후속 구현 범위입니다.
- [ ] destination의 별도 미저장 초안을 강제로 버리는 확인 동작, 파일 삭제 후 이미 열린 live 문서의 실시간 전환, mirror JSON의 읽기 전 용량 상한과 대형 본문의 렌더 예산은 남습니다.
- [ ] root guard는 앱 내부 변경만 직렬화합니다. 외부 프로세스의 symlink/metadata 교체에 대한 filesystem TOCTOU 및 write 이후 재열기/cleanup 실패 시 재시도 UX는 별도 검증이 필요합니다. 실패 시 이미 작성된 destination 파일 자체를 지우거나 되돌리지는 않습니다.

N2-A2c와 N2/M8 전체는 미완료입니다. 기존 성공 검사와 성능 계측은 재사용하며 M8 전체 완료 전 commit/push하지 않습니다.
