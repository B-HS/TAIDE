# 로케일 `{{placeholder}}` ↔ Rust `with_arg` 파리티 체크리스트

> 정본: `docs/acknowledge/2026-08-24-d34-apperror-campaign-contract.md` §5 항목 1(D34-L1-01/L2-1/L3-01).
> d-34 검토가 실측으로 발견한 결함 클래스 — `en.json`/`ko.json`/`ja.json`의 `{{placeholder}}`
> 집합과 그 키를 생성하는 `AppError::localized(...).with_arg(...)` 체인의 인자 이름 집합이
> 어긋나도, `domain::locale::service` 의 기존 파리티 테스트 2종(내장 3언어 키 **집합**·
> `required_message_keys` 포함 여부)은 값(`{{…}}` 자리표시자·인자 이름)을 보지 않아 잡아내지
> 못한다. `error.ai.providerUnavailable` 의 5xx 갈래가 실제로 이 구멍을 통과해 배포 직전까지
> 남아 있었다.

## 왜 자동화하지 않았는가

- 3언어 간 `{{placeholder}}` 집합 동일성은 문자열만 비교하면 되어 테스트로 강제 가능하다(추천 — 아래 §1).
- 그러나 카탈로그 placeholder ⊇ Rust `with_arg` 인자 이름은, **130개 `AppError::localized` 호출부**
  전부에서 체이닝된 `.with_arg("name", …)` 리터럴을 소스 레벨로 추출해야 한다. Rust 매크로/derive
  만으로는 정적으로 대조할 수 없고(값이 `format!`/변수로 조립되는 지점도 있어 단순 정규식만으로는
  오탐이 난다), 이번 검토가 쓴 방법(소스를 괄호균형 파서로 스캔)과 동등한 정확도의 스캔기를
  자체 구현해야 해서 d-34 수정 배치 범위를 넘어선다고 판단해 이월했다.

## 체크리스트

- [ ] **§1 — 3언어 자리표시자 파리티(자동화 권장 대상)**: `en.json`/`ko.json`/`ja.json` 의 각
  키에 대해 `{{…}}` 자리표시자 집합이 3언어 동일한지 `domain::locale::service` 의 `tests` 모듈에
  Rust 테스트로 추가한다. (L2-2가 지적한 미강제 축 중 이쪽은 순수 문자열 비교라 구현 난이도가 낮다)
- [ ] **§2 — 카탈로그 placeholder ⊇ Rust with_arg 인자(수동 확인)**: 새 `error.*` 로케일 키를
  추가하거나 기존 키의 `{{…}}` 를 바꿀 때마다, 그 키를 생성하는 모든 `AppError::localized(...)`
  호출부의 `.with_arg(...)` 체인이 카탈로그가 요구하는 placeholder 전부를 공급하는지 수동으로
  대조한다. `rg 'AppError::localized\(' -A 10 src-tauri/src` 로 호출부를 찾고, 각 호출부 뒤에
  이어지는 `.with_arg("이름", ...)` 를 카탈로그 값(예: `en.json`)의 `{{이름}}` 집합과 눈으로
  비교한다.
- [ ] **§3 — 신규 `error.*` 키 PR 셀프 체크**: 새 provider/도메인 에러를 추가하는 변경에서는,
  머지 전에 다음을 스스로 확인한다.
  1. `en.json`/`ko.json`/`ja.json` 세 파일 모두 같은 키·같은 placeholder 집합을 갖는가
  2. 그 키를 만드는 Rust 호출부의 `with_arg` 이름이 placeholder 전부를 덮는가
  3. `cargo test -p taide domain::locale` 가 green 인가(집합 파리티만 보장 — §2는 별도 확인 필요)

## 참고 — 이번에 실측으로 확인한 범위

d-34 검토 시점(2026-08-24/25) 기준 130개 `AppError::localized` 호출부 전수 대조 결과, 카탈로그
placeholder와 Rust 공급 인자의 불일치는 `error.ai.providerUnavailable` 1건뿐이었고(수정 완료 —
계약 §5 항목 1), 3언어 간 placeholder 집합 불일치는 0건이었다. 이 문서의 §1을 구현하면 후자
축은 자동 강제로 승격되고, §2는 당분간 수동 체크리스트로 남는다.
