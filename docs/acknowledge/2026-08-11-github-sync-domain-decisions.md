# GitHub 동기화 도메인(`domain/sync`) 신설 — 구현 시 판단 기록

> 범위: secret Gist + PAT 기반 설정/테마/로케일 동기화. 확정 결정
> (`2026-08-11-qa5-batch-decisions.md`)의 "GitHub 동기화는 secret Gist + PAT(gist 스코프),
> 페이로드 화이트리스트(settings+themes+locales, 머신 종속·시크릿 제외), 수동 업로드/다운로드
> + 시작 시 최신 배지, 충돌은 2택" 을 구현하며 코드에 남기지 않은 배경을 정리한다.

## 1. 설정 화이트리스트 = `SettingsPatch`(shell_override 제외)

새로 만드는 별도 화이트리스트 타입 대신, `settings_update` 가 이미 받는 `SettingsPatch` 를
그대로 재사용했다(`domain::sync::service::settings_to_sync_patch`). 모든 필드를 `Some(...)` 로
채우되 `shell_override` 만 `None` 으로 비운다 — 셸 경로는 머신마다 다르므로 절대 동기화하지
않는다. 이렇게 하면:

- **업로드**: 현재 `Settings` → `SettingsPatch` 변환 한 번.
- **다운로드(역적용)**: `settings_service::apply_patch(current, &payload.settings)` 를
  그대로 호출한다 — `settings_update` 커맨드와 동일한 함수. `shell_override` 는 patch 가
  `None` 이므로 로컬 값이 그대로 보존된다(별도 예외 처리 불필요, 재사용 자체가 배제 메커니즘).
- `syncGistId`/`syncLastSyncedAt` 은애초에 `SettingsPatch` 필드가 아니므로 페이로드에 담기지
  않는다(자체 참조 배제).

`version` 필드도 `SettingsPatch` 에 없으므로 자연히 제외된다 — 페이로드 최상위의
`schemaVersion`(= `SETTINGS_SCHEMA_VERSION`)이 그 역할을 대신한다.

## 2. `sync_gist_id` / `sync_last_synced_at` 을 `Settings` 에 신규 필드로 추가

두 필드 모두 `SettingsPatch` 에는 넣지 않았다 — 일반 설정 UI(`settings_update`)로는 바꿀 수
없고, `domain::sync::commands` 만 `Settings { ..current, sync_gist_id: ..., .. }` 구조체
갱신으로 직접 쓴다(`set_theme` 가 자기 전용 함수로 `theme_id` 만 바꾸는 것과 같은 패턴).
`apply_patch` 의 필드 리스트가 스프레드(`..settings`) 대신 전 필드를 수동 나열하는 기존
스타일이라, 두 필드를 그대로 통과시키는 라인을 `apply_patch` 에도 추가해야 컴파일된다(불가피한
연쇄 수정).

## 3. 충돌 판정은 문자열 비교 — 날짜/시간 크레이트를 새로 추가하지 않음

`updatedAt` 을 고정폭 `YYYY-MM-DDTHH:MM:SSZ` 로만 만들면(zero-padded), 사전식 문자열 비교가
곧 시간 선후 비교와 같다. GitHub Gist API 의 `updated_at` 도 같은 포맷이라 별도 파싱 없이 바로
비교 가능하다(`domain::sync::service::is_remote_newer`). 승인 목록(keyring) 외 신규 패키지를
추가하지 않는다는 이번 웨이브 제약과, 호출부가 이 한 곳뿐이라는 점("2회 이상" 룰) 두 가지
이유로 `chrono`/`time` 크레이트 대신 Howard Hinnant 의 `civil_from_days` 알고리즘을 손으로
구현했다(`domain::sync::service::format_unix_utc_iso8601`). 정확성은 `date -u -r <secs>` 로
뽑은 참조값들로 단위 테스트를 걸어 검증했다.

충돌 소스는 페이로드 내부의 `updatedAt` 이 아니라 **Gist 리소스 자체의 `updated_at` 메타데이터**
(서버 시각, 클럭 드리프트에서 자유롭다)로 통일했다. `sync_status` 가 원격이 더 새로운지 미리
알려줄 때도 파일 내용을 파싱할 필요 없이 같은 메타데이터만 본다.

## 4. 테마/로케일 다운로드는 "실패한 것만 건너뛴다"

`apply_theme_entries`/`apply_locale_entries` 는 엔트리 하나가 파싱 실패하거나(`save_theme`가
내장 id 충돌을 거부하는 경우 등) 있어도 나머지 엔트리는 계속 적용한다 — 플러그인 로더의
"하나가 깨져도 나머지는 계속 로드" 정책과 동일한 결이다. 실패는 `log::warn!` 로만 남기고
`sync_download` 전체를 실패시키지 않는다.

id 가 겹치는 테마/로케일은 `theme_service::save_theme`/`locale_service::save_locale` 이 이미
파일을 무조건 덮어쓰므로, 재다운로드는 곧 "마지막으로 받은 것이 이긴다".

## 5. `domain/locale/service.rs::save_locale` 신규 — `save_theme` 미러링

로케일 도메인에는 기존에 저장 경로가 없었다(조회/목록만 있었음). 테마 쪽 `save_theme` 와
완전히 동일한 가드(빈 id 거부·내장 id 덮어쓰기 거부·경로 구분자 거부)로 새로 추가했다 — 이번
웨이브에서 `locale/service.rs` 를 직접 수정하는 것이 명시적으로 허용된 범위였다.

## 6. `mask_provider_error` 재사용 (domain/ai 파일 자체는 미수정)

GitHub API 에러 메시지 마스킹에 `domain::ai::providers::mask_provider_error` 를 그대로 가져다
썼다(호출만, `domain/ai/*` 파일은 수정하지 않음). 토큰/긴 불투명 문자열 마스킹 로직을 도메인마다
새로 만들지 않기 위함("2회 이상" 공통화 근거).

## 7. IPC 표면

`sync_status`(연결·gist 존재·마지막 동기화·원격 최신 여부) / `sync_connect(pat)`(GET
`/gists` 1회로 스코프 검증 후 keyring 저장) / `sync_disconnect` / `sync_upload` /
`sync_download(force)` 5개 커맨드 + `sync:state-changed` 이벤트. PAT 원문은 어떤 IPC 응답·
이벤트·로그에도 담기지 않는다 — keyring(`SecretAccount::GithubSync`, 이미 인프라에 존재)에만
저장한다.
