# native 미러 정리와 원격 쓰기의 비교·삭제 경계

## 대상·원인

`native/taide-native-app/src/file_sync.rs::cleanup_mirror`는 전역 mutation guard 안에서 미러 목록과 expected를 비교한 다음 일반 clear를 호출했습니다. `crates/taide-runtime/src/file_actions.rs::file_mirror_dirty`는 의도적으로 같은 전역 guard 없이 blocking worker에서 씁니다. 따라서 전역 guard만으로 조회→삭제 사이 새 미러 쓰기를 막을 수 없었습니다.

## 해결·검증

파일 미러 서비스의 write/clear/prune/조건부 compare-clear를 같은 전용 Mutex로 연결했습니다. native cleanup은 최종 compare-clear로 다시 확인합니다. 원격의 기존 file_clear_mirror에는 expected optional 모드를 연결하되 없는 경우 기존 null 계약을 유지합니다. 새 원격 expected 검사 null/기대 false RED→수정 뒤 GREEN·새 실제 파일 동시 쓰기1·변경된 actual native host1 각각 PASS이며 상세 명령/시간/원자성 범위는 `docs/quality-assurance/2026-10-05-m8-remote-mirror-conditional-clear.md`입니다.

RED는 새로운 expected 원격 계약의 부재를 재현한 것이며 기존 TS가 이미 그 인자를 보내던 회귀라고 주장하지 않습니다. native 경합의 특정 과거 사용자 데이터 손실을 관찰한 것도 아닙니다. browser mirror owner/전체 제품 게이트는 미완료입니다.
