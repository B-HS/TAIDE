# 취소된 native 저장이 이전 경로를 재생성한 오류

## 대상·관찰

- 대상: `native/taide-native-app/src/file_sync.rs`의 `save_snapshot`, `tests/disk.rs`의 취소된 generation 검사입니다.
- 실제 합성 파일의 snapshot을 얻고 파일을 이름 변경한 뒤 DraftEpoch를 취소했습니다. 취소된 snapshot을 기존 경로로 저장하자 성공 응답을 반환했고 기존 경로를 다시 생성했습니다.
- 신규 검사 최초 실행은 `result.is_err()`에서 실패했습니다. 기존 mirror worker의 generation 검사를 실제 파일 저장에도 적용했다고 가정할 수 없는 상태였습니다.

## 원인·수정

- 저장 worker는 성공 후 epoch를 invalidate했지만 저장 전에 `is_current()`를 검사하지 않았습니다. 기존 저장 서비스의 missing target 생성 기능까지 도달해 이름 변경 전 경로를 만들었습니다.
- owned mutation guard를 획득한 뒤와 file policy 재조회 뒤 원자적 저장 직전에 generation을 확인합니다. 취소된 요청은 Forbidden으로 종료하며 disk write와 mirror 정리를 실행하지 않습니다.
- native resource admission의 affected document generation 취소와 연결합니다. 이미 원자적 쓰기에 들어간 OS 작업의 임의 취소·rollback이나 모든 외부 FS 경쟁을 해결했다고 주장하지 않습니다.

## 검증

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test disk 취소된_저장_generation --locked --offline --target-dir experiments/native-shell-spike/target` 실패 재현 후 수정한 대상 1회 통과, 0.01초입니다. 이전 경로 부재·이름 변경된 파일의 내용 불변·tracked worker 0을 확인했습니다.
- [x] 최종 app lib/bin 및 disk·lsp·workspace-worker strict clippy exit 0, 0.91초입니다. 같은 성공 기능 검사는 반복하지 않았습니다.
