# 언어 pack 경로 경계 수정

## 대상과 원인

- `crates/taide-locale/src/service.rs`: `load_locale`와 `locale_exists`가 외부 ID를 파일명에 직접 결합해 `../`가 pack 디렉터리 밖을 읽을 수 있었습니다. `list_locales`도 내부 링크의 외부 대상을 읽었습니다.
- `src-tauri/tests/appearance_actions_runtime.rs`: 이전 동작을 기록하던 경로 이탈 조회 assertion을 보안 거부 계약으로 교체했습니다. runtime/Tauri 공개 형태는 변경하지 않았습니다.

## 수정과 동등성

저장과 조회가 같은 ID 술어를 사용합니다. 빈 ID와 경로 구분자, 점, 콜론, NUL을 거절해 ID가 파일 경로 또는 Windows 대체 데이터 스트림으로 해석되지 않게 합니다. 조회·존재·목록은 `symlink_metadata`로 일반 파일만 허용해 정적인 symlink를 건너뜁니다. 유효한 사용자 pack, 내장 우선순위, 없는 pack의 `NotFound`, 잘못된 JSON 오류는 기존 검사로 유지했습니다. 잘못된 조회 ID는 이제 `InvalidArgument`, 명시 언어 선택은 `en` 폴백입니다.

## 재현과 검증

자기 UUID fixture에 `locales/` 밖의 유효 pack과 그 pack을 가리키는 `locales/linked.json`을 만들었습니다. 두 신규 검사 모두 수정 전 exit 101이었고, 경로 이탈 조회는 실제 외부 pack을 반환했습니다. 수정 뒤 `cargo test -p taide-locale`은 20건, `cargo test -p taide --test appearance_actions_runtime locale_목록과_조회는_사용자_pack_및_경로_이탈_거부와_깨진_json_오류를_유지한다`는 1건 통과했습니다. 관련 runtime/Tauri clippy와 최종 locale all-target clippy·Rust fmt/diff·대상 MD 포맷은 모두 exit 0입니다. 총 21개 서로 다른 검사이며 재실행은 합산하지 않았습니다. 선별 commit 결과는 PROCESS에 기록합니다.

## 남은 경계

첫 단위에서는 `symlink_metadata`와 실제 읽기 사이의 로컬 동시 교체 경쟁이 남았습니다. 후속 보강에서 `load_locale`와 `list_locales`가 같은 `read_locale_file`을 사용하고 Unix의 최종 파일 열기에 `O_NOFOLLOW | O_NONBLOCK`을 적용했습니다. 열기 뒤에도 실제 파일 핸들의 metadata가 일반 파일인지 확인한 다음 읽습니다. 자체 symlink fixture에서 `O_NOFOLLOW` 열기의 `ELOOP`와 조회·목록 거부를 확인했습니다. [Apple open(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/open.2.html)의 최종 symlink 거부 계약에 맞는 방식입니다. 기존 `libc 0.2.189`를 locale crate에 직접 연결했으며 Cargo.lock은 의존 edge 한 줄만 추가됐습니다.

이 보강 뒤 locale 20건과 Tauri 경유 1건, locale all-target/Tauri 관련 clippy·Rust fmt가 통과했습니다. 새 helper는 파일 읽기와 JSON 파싱의 기존 `Io`/`Internal` 매핑을 유지합니다. 최종 diff/문서 검사는 QA에 기록합니다. `O_NOFOLLOW`는 마지막 경로 성분만 보호합니다. 신뢰된 `AppPaths`의 부모 디렉터리 교체, Windows reparse point의 동시 교체·ADS 실기, 실제 원격 호출은 별도 미검증이며 M6/M7 전체 보안 완료로 해석하지 않습니다.
