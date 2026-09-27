# 언어 pack 경로 경계 QA

## 확인한 결과

- [x] 자기 UUID pack의 `../outside` 및 외부 파일 symlink 조회 검사 각각 수정 전 exit 101을 확인했습니다. 사용자 파일은 읽지 않았습니다.
- [x] `cargo test -p taide-locale`: 20건 통과(exit 0). 잘못된 ID의 조회·존재·선택 폴백, 목록의 잘못된 ID 제외, 정적 symlink 거부, 정상 저장/내장·병합/오류를 확인했습니다.
- [x] `cargo test -p taide --test appearance_actions_runtime locale_목록과_조회는_사용자_pack_및_경로_이탈_거부와_깨진_json_오류를_유지한다`: 1건 통과(exit 0). runtime facade의 이탈 ID 거부와 정상/손상 pack 계약을 확인했습니다.
- [x] `cargo clippy -p taide-locale -p taide --lib --test appearance_actions_runtime -- -D warnings`와 최종 `cargo clippy -p taide-locale --all-targets -- -D warnings`: 각각 exit 0입니다. invalid pack 목록 assertion도 최종 test-target 검사에 포함됐습니다.
- [x] `cargo fmt --all -- --check`, `git diff --check`, 신규/수정된 acknowledge·bug·history·QA 4개 MD의 Prettier `--check`: 각각 exit 0입니다. 기존 대형 PROCESS 전체 재포맷은 수행하지 않았습니다.

## 남은 검사와 한계

- [x] 최종 locale test-target clippy·Rust fmt/diff·대상 MD 포맷을 확인하고 검증 단위만 선별 commit합니다.
- [x] 후속 no-follow 읽기 단위: 자체 symlink fixture에서 Unix `O_NOFOLLOW | O_NONBLOCK` 열기가 `ELOOP`를 반환하고 조회·목록에서 외부 pack이 보이지 않는 것을 확인했습니다. `cargo test -p taide-locale` 20건, `cargo test -p taide --test appearance_actions_runtime locale_목록과_조회는_사용자_pack_및_경로_이탈_거부와_깨진_json_오류를_유지한다` 1건이 exit 0입니다.
- [x] 후속 `cargo clippy -p taide-locale --all-targets -- -D warnings`, `cargo clippy -p taide --lib --test appearance_actions_runtime -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`, 변경한 bug/history/QA 3개 MD의 Prettier `--check`: 각각 exit 0입니다. Cargo.lock은 기존 libc 버전의 locale 직접 의존 한 줄만 추가됐습니다.
- [ ] Unix 최종 파일명 링크 교체는 no-follow 열기로 제한했습니다. 부모 디렉터리 교체, Windows reparse point/ADS 실기, 실제 원격 gateway/GUI, 전체 M6/M7/M8 gate는 별도 검증입니다. 모든 OS에서 원자적 containment를 주장하지 않습니다.
