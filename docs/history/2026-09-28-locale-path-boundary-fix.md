# 언어 pack 경로 경계 수정

## 대상과 원인

- `crates/taide-locale/src/service.rs`: `load_locale`와 `locale_exists`가 외부 ID를 파일명에 직접 결합해 `../`가 pack 디렉터리 밖을 읽을 수 있었습니다. `list_locales`도 내부 링크의 외부 대상을 읽었습니다.
- `src-tauri/tests/appearance_actions_runtime.rs`: 이전 동작을 기록하던 경로 이탈 조회 assertion을 보안 거부 계약으로 교체했습니다. runtime/Tauri 공개 형태는 변경하지 않았습니다.

## 수정과 동등성

저장과 조회가 같은 ID 술어를 사용합니다. 빈 ID와 경로 구분자, 점, 콜론, NUL을 거절해 ID가 파일 경로 또는 Windows 대체 데이터 스트림으로 해석되지 않게 합니다. 조회·존재·목록은 `symlink_metadata`로 일반 파일만 허용해 정적인 symlink를 건너뜁니다. 유효한 사용자 pack, 내장 우선순위, 없는 pack의 `NotFound`, 잘못된 JSON 오류는 기존 검사로 유지했습니다. 잘못된 조회 ID는 이제 `InvalidArgument`, 명시 언어 선택은 `en` 폴백입니다.

## 재현과 검증

자기 UUID fixture에 `locales/` 밖의 유효 pack과 그 pack을 가리키는 `locales/linked.json`을 만들었습니다. 두 신규 검사 모두 수정 전 exit 101이었고, 경로 이탈 조회는 실제 외부 pack을 반환했습니다. 수정 뒤 `cargo test -p taide-locale`은 20건, `cargo test -p taide --test appearance_actions_runtime locale_목록과_조회는_사용자_pack_및_경로_이탈_거부와_깨진_json_오류를_유지한다`는 1건 통과했습니다. 관련 runtime/Tauri clippy와 최종 locale all-target clippy·Rust fmt/diff·대상 MD 포맷은 모두 exit 0입니다. 총 21개 서로 다른 검사이며 재실행은 합산하지 않았습니다. 선별 commit 결과는 PROCESS에 기록합니다.

## 남은 경계

`symlink_metadata`와 실제 읽기 사이에서 로컬 동시 교체가 가능한 경쟁 조건은 남습니다. 이번 회귀는 고정된 링크와 원격 ID 기반 이탈을 확인했으며 동시 파일시스템 공격·Windows reparse point·실제 원격 호출은 검증하지 않았습니다. 따라서 M6/M7 전체 보안 완료나 모든 OS에서 원자적 파일 경계라고 주장하지 않습니다.
