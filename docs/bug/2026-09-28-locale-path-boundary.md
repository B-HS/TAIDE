# 언어 pack 조회의 기존 경로 검증 누락

## 대상 파일

- `crates/taide-locale/src/service.rs`: `load_locale`, `locale_exists`, `resolve_language`
- `crates/taide-runtime/src/locale_actions.rs`: `locale_get`, `locale_get_current`
- `src-tauri/src/domain/locale/commands.rs`, `src-tauri/src/remote_gateway.rs`
- `src-tauri/tests/appearance_actions_runtime.rs`

## 리포트

기존 `load_locale`는 builtin 확인 후 외부 locale ID를 `locales_dir().join(format!("{locale_id}.json"))`에 직접 사용합니다. 경로 구분자/상위 경로 검증이 없으며 기존 IPC와 원격 gateway의 locale_get도 같은 정책을 사용합니다. 이번 facade 이전에서 새로 생긴 동작이 아닙니다.

## 상세와 재현

1. 자기 UUID fixture의 `locales/`에 정상 pack을 저장한 다음 그 밖의 같은 fixture 루트에 `fixture-outside.json`을 직접 생성했습니다. 모든 내용은 builtin에서 복제한 테스트 pack입니다.
2. runtime `locale_get("../fixture-outside")`와 원래 `load_locale`가 모두 그 pack을 반환했습니다. 해당 파일은 `locales_dir()` 안에 있지 않습니다. appearance target 9건이 통과했으며 이 관찰은 제품 보안 합격을 뜻하지 않습니다.
3. 사용자 파일/자격증명·앱·원격 서버·Windows는 실행하지 않았습니다. 실제 remote 도달성은 gateway의 locale_get dispatch가 검증 없는 동일 command를 호출한다는 source 근거이며 네트워크 재현 결과는 아닙니다. 유효 pack의 경로 이탈 읽기를 확인했으며 임의 파일 원문 전체가 응답된다고 주장하지 않습니다.
4. `locale_exists`도 같은 경로 조립을 사용하므로 current selector의 설정 ID 경로도 함께 검토해야 합니다. Windows drive-relative/ADS·symlink 경계는 이번 fixture로 실행하지 않았습니다.

## 처리 상태

기존 정책 보존 이전 단위와 보안 수정 단위를 분리합니다. 사용자에게 A(경로 검증 추가·별도 보안 회귀)와 B(현재 동작 유지·미완료 보안 gate 기록)를 한 번에 질문했습니다. 답변 전에는 service 정책을 변경하지 않으며 M6/M7의 보안 완료나 정상 동등성 합격으로 처리하지 않습니다. 실제 사용자 선택 이후 실패 회귀·수정·관련 최소 검증을 별도 단위로 진행해야 합니다.
