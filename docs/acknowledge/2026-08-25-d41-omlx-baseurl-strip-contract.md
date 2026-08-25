# d-41 — `ai_omlx_base_url` 원격 스트립 편입 계약 (2026-08-25)

> 정본: d-38 계약 §6(잔존 경로 실사 — 검토 SEC-03)·`2026-08-25-post-batch-user-decisions.md`
> §5(사용자 확정 "스트립 편입"). 선례: `shell_override` 의 "세션 초과 지속성" 근거 스트립.

## 0. 실사 (d-38 검토 SEC-03 승계)

- OMLX 는 `Settings.ai_omlx_base_url` 호스트로 키링의 API 키를 `Authorization: Bearer` 전송
  (`ai/service.rs` omlx_provider → `providers/omlx.rs` apply_auth). 이 필드는 원격 허용
  `settings_update` 와 `sync_download` 로 변경 가능 — `strip_remote_gated_settings_patch`/
  `strip_remote_gated_settings`(dispatch.rs)는 3필드(remote_password_only_login·
  remote_allowed_hosts·shell_override)만, `strip_non_syncable`(sync/service.rs)도 이 필드를
  막지 않음. ollama/codex 는 호스트 하드코딩이라 해당 없음.

## 1. 범위 (사용자 확정)

- `ai_omlx_base_url` 을 원격 게이트 스트립 대상에 편입: `strip_remote_gated_settings_patch`·
  `strip_remote_gated_settings` 양쪽(+실사로 같은 축의 스트립 지점이 더 있으면 — 예: gist
  다운로드 경로가 dispatch 스트립을 우회하는지 — 함께 판단·기록). 데스크톱 로컬 변경은 불변.
- doc·정책 문서 정합: dispatch.rs 스트립 doc·`ipc-contract.md` 의 d-38 한정 문구("여전히 허용
  경로로 변경 가능")를 편입 완료로 갱신, d-38 계약 §6 은 시점 기록 유지.
- 테스트: 기존 스트립 테스트 관행대로 patch/전체 양쪽에서 이 필드가 제거됨을 고정.
- 범위 외: 다른 settings 필드 정책·QoS(기각 확정)·원격 세션의 OMLX 사용 자체 차단.

## 2. 실행·검토

- 구현 Rust 1(sonnet+xhigh) → 검토 1렌즈(opus+xhigh — 보안·동등성 겸무: 스트립 완전성(우회
  경로 0)·로컬 경로 불변·doc 정합) → major 시 적대적 → 수정 → 메인 2차. 소형 배치 렌즈 1
  (보안 단일 관심사·변경 반경 극소 — 축소 근거 기록).

---

## 3. 구현 완료 기록 (구현 에이전트가 기록)

> 구현: Rust 1 에이전트(sonnet, 2026-08-25). 범위 §1 전부 완료. 검증 게이트 전부 green(§3.5).

### 3.1 편입 지점 — `strip_remote_gated_settings_patch`/`strip_remote_gated_settings`

`src-tauri/src/domain/remote/dispatch.rs` 의 원격 게이트 스트립 두 함수 모두에 `ai_omlx_base_url`
을 `shell_override` 와 동일 취급으로 편입했다(선례: shell_override 의 "세션 초과 지속성" 근거 —
`providers/omlx.rs` `apply_auth` 가 키링의 OMLX API 키를 이 필드가 가리키는 호스트로
`Authorization: Bearer` 전송, 매 요청마다 새로 읽음):

- `strip_remote_gated_settings_patch`(`settings_update` 패치 대상): `patch.ai_omlx_base_url = None;`
  1줄 추가 — patch 의 3상태 규약(생략=미변경)상 `None` 강제가 곧 "이 필드는 건드리지 않음"이 된다.
- `strip_remote_gated_settings`(`app_file_write` 의 `Settings::Settings` 전체 교체 대상):
  `next.ai_omlx_base_url = current.ai_omlx_base_url.clone();` 1줄 추가 — 기존 3필드와 동일하게
  "현재값으로 되돌림"(clear 가 아니라 unchanged 전략, `shell_override` 와 동일 처리 — `Settings`
  타입에서도 `ai_omlx_base_url` 은 이미 `Option<String>` 이라 실제로는 clear 도 표현 가능하지만,
  기존 3필드 중 2개(`remote_password_only_login`/`remote_allowed_hosts`)가 `Settings` 에서
  `Option` 이 아니라 clear 불가라 "전체 unchanged" 전략으로 이미 통일돼 있었다 — 그 전략을 그대로
  따랐다).
- 두 함수의 doc 주석에 `ai_omlx_base_url` 을 대상 필드 목록에 추가하고, `shell_override` 문단과
  같은 구조로 위협 시나리오(공격자 호스트로 리다이렉트 → 향후 OMLX 트래픽이 Bearer 키째 그쪽으로
  흐름 → 세션 종료 후에도 사용자가 직접 고치기 전까지 지속)를 서술, 이 계약 파일을 근거로 링크했다.
  `strip_remote_gated_settings` 쪽은 "세 필드"→"네 필드", "`shell_override` 만 Option"→
  "`shell_override`/`ai_omlx_base_url` 이 Option" 으로 기존 문장도 정정했다.

### 3.2 같은 축 우회 경로 실사 — `sync_download` → `strip_non_syncable`(실재, 편입 완료)

계약 §0/§1 이 예시로 지목한 그대로 우회가 **실재**했다: `sync_download`(`sync/commands.rs`)는
다운로드한 gist 본문을 `apply_payload_settings` → `strip_non_syncable(&payload.settings)` →
`apply_patch` 경로로 적용하며, 이 경로는 `remote::dispatch::strip_remote_gated_settings_patch`/
`strip_remote_gated_settings` 둘 중 어느 쪽도 거치지 않는다(`dispatch.rs` 의 `"sync_download"`
match arm 이 `sync::sync_download(...)` 를 직접 호출 — 두 스트립 함수는 `"settings_update"`/
`"app_file_write"` arm 에서만 호출된다). `strip_non_syncable`(`sync/service.rs`)은 기존에
`shell_override`/`remote_access_enabled`/`remote_password_only_login`/`remote_allowed_hosts` 4필드만
막고 있어 `ai_omlx_base_url` 은 그대로 통과했다 — 이 갭은 신규 발견이 아니라
`2026-08-15-wave-b-hardening-contract.md` §4 가 이미 "gist 필터에 ai_omlx_base_url 등 | 보류 —
추천안은 게이트2+shell_override+allowed_hosts만" 으로 **당시 의도적으로 보류**해 둔 것이었다.

**같은 원칙으로 닫음**: `strip_non_syncable` 은 업로드(`settings_to_sync_patch`)·다운로드
(`apply_payload_settings`) 양방향이 공유하는 단일 함수이고(그 자체 doc 주석 근거: "두 방향이
갈라지지 않도록"), 이 필드도 기존 4필드와 동일한 "세션을 넘어 남는 지속성" 근거이므로 방향별
비대칭 필터를 새로 만들지 않고 기존 4필드와 같은 방식으로 `ai_omlx_base_url: None,` 을 구조체
갱신 리터럴에 추가했다 — `settings_to_sync_patch`(업로드)도 함께 영향을 받아, 유출·공유된 gist 가
OMLX base_url 을 더는 담지 않게 됐다(업로드 쪽 원 리터럴의 `ai_omlx_base_url:
settings.ai_omlx_base_url.clone()` 줄은 기존 4필드와 동일하게 "설정 후 strip_non_syncable 이
되돌림" 패턴 그대로 유지 — 죽은 코드가 아니라 기존에도 4필드가 쓰던 동일 패턴).

다른 원격 허용 커맨드 경로는 재확인 결과 우회가 아니다: `sync_upload` 는 인자를 받지 않고
서버측 `state.settings` 스냅샷만 `assemble_payload`→`settings_to_sync_patch`(스트립 적용됨)로
직렬화하므로 원격 호출자가 임의 값을 주입할 경로가 없다. `sync_download` 자체도 `force: bool`
외 인자가 없어 gist **본문**이 유일한 유입 지점이며, 그 본문 유입은 위에서 닫았다.
`settings_set_theme`(원격 허용)은 `themeId: String` 만 받아 `ai_omlx_base_url` 과 무관하다.
`domain::ai::commands.rs` 는 `ai_omlx_base_url` 을 읽기만 하고 쓰는 코드 경로가 없다(전수
`grep` 확인). 세 경로(`settings_update`·`app_file_write`·`sync_download`) 외의 네 번째 유입
경로는 없다.

### 3.3 doc 정합

- `docs/ipc-contract.md` "원격 dispatch 정책" 절 — d-38 블록 안의 "여전히 허용 경로로 변경
  가능하다 — … 사용자 결정 이월이다(코드 변경 없음)" 문구를, d-41 로 편입이 완료됐고
  `strip_remote_gated_settings_patch`/`strip_remote_gated_settings`(2경로) 뿐 아니라
  `sync_download`(`strip_non_syncable`, §3.2 의 우회 경로)까지 3경로 모두에서 닫혔다는 서술로
  갱신.
- 같은 절의 거부 테이블 `CredentialStoreTampering` 행 — 동일 문구를 3경로 명시 서술로 갱신.
- `docs/acknowledge/2026-08-25-d38-remote-policy-contract.md`·
  `2026-08-15-wave-b-hardening-contract.md` 는 시점 기록이라 무수정(과거 "이월"/"보류" 판단은
  그 시점의 정확한 기록으로 그대로 둔다 — 이 계약 §3.2 가 그 후속 결정과 근거를 담는다).

### 3.4 테스트

기존 스트립 테스트 관행대로 patch/전체 양쪽 + gist 양방향에 전용 테스트를 추가했다(회귀 고정):

- `dispatch.rs`: `원격_settings_update_패치는_omlx_base_url_필드가_제거된다`(patch 경로),
  `원격_app_file_write는_omlx_base_url_필드를_현재값으로_되돌린다`(전체 `Settings` 경로 — 이
  전용 함수 `strip_remote_gated_settings` 자체를 직접 호출하는 **최초의** 단위 테스트이기도
  하다; 기존에는 `app_file_write` arm 내부에서만 호출돼 전용 단위 테스트가 없었다).
- `sync/service.rs`: 기존 결합 회귀 테스트
  `손으로_만든_gist_페이로드의_shell_override와_원격_게이트_필드는_적용되지_않는다` 를
  `…_shell_override_원격_게이트_omlx_base_url_필드는_적용되지_않는다` 로 개명하며 손으로 작성한
  악성 JSON 에 `"aiOmlxBaseUrl"` 필드와 그 단언을 추가(이 파일의 기존 관행이 필드별 개별 테스트가
  아니라 결합 테스트라 그 관행을 따름), `strip_non_syncable은_omlx_base_url_필드를_제거한다`(단위),
  `settings_to_sync_patch는_원격_게이트와_허용_호스트와_omlx_base_url을_제외한다`(업로드 측 개명+
  단언 추가, 기존 함수명에 필드 미반영이던 것을 정정) 3건.

### 3.5 검증 결과

`export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup";
export PATH="$CARGO_HOME/bin:$PATH"` 로 실행.

- `cargo fmt --all` → `cargo fmt --all -- --check`: 클린(diff 0).
- `cargo test --workspace`: **1119 passed / 0 failed**(`taide` lib 1093 + `domain_boundaries` 3 +
  `session_restore` 6 + `taide_cli` 17, doc-test 0). `dispatch.rs` 신규 테스트 2건 +
  `sync/service.rs` 신규 테스트 2건(+개명 1건) 전부 green. dispatch 파리티 3종
  (`허용_테이블과_거부_테이블은_전체_커맨드를_교집합_없이_정확히_분할한다` 등)도 그대로 green —
  이번 배치는 두 테이블(`REMOTE_ALLOWED_COMMANDS`/`REMOTE_DENIED_COMMANDS`)·`match` arm 집합을
  전혀 바꾸지 않았다(스트립 함수 내부 로직만 변경).
- `cargo clippy --workspace --all-targets -- -D warnings`: **0 warnings**.
- `git status --porcelain src/shared/api/bindings.ts`: 빈 출력 — diff 0(스트립은 커맨드
  시그니처·specta 등록에 관여하지 않는 내부 로직이라 bindings 표면에 나타날 수 없다는 §1 계약이
  실측으로도 확인됨).
- `bun test`: **1481 pass / 0 fail**(2444 `expect()`, 149 파일) — d-38 완료 시점과 동일 수치,
  무회귀(이번 배치는 Rust 전용 변경이라 프론트 테스트 결과에 영향이 없다).

---

## 4. 검토 반영 (2026-08-25)

> 검토 wf_efa17b6f-884 렌즈 1(보안·동등성, opus+xhigh): major 2·minor 2·info 1 — 스트립
> 완전성(3경로 봉쇄)·로컬 경로 불변·양방향 대칭 판정 자체는 통과, 발견은 전부 문서·미러 정합.
> 수정 wf_7fee9d43-713. 원문: 태스크 출력 `wsuwuj12y.output` result[0].

- **major 2건(확정 — 적대적 생략)**: DOC-01 `ipc-contract` "부분 스트립" 항목 3필드 잔존
  (같은 절 내 자기모순)·DOC-02 `### sync` 카탈로그 4필드 잔존(업로드 미동기화 — 사용자 가시
  동작 변경의 유일 기록 지점). 메인 grep 직접 재현(:1020·:1023·:529) + 기계적 문서 정정 —
  d-34 선례로 적대적 생략.
- 수정: 1 부분 스트립 4필드 정합 / 2 sync 5필드+업로드 근거+data-model 각주 / 3
  `apply_settings_file` doc 열거 → 정본 참조 교체(드리프트 원천 차단) / 4 e2e 게이트 미러
  4필드 확장(`REMOTE_GATED_SETTINGS_KEYS`+시도값·UI 경로+spec doc+하네스 문서 — TEST-04:
  스펙 11 전수 커버 계약 복원·teardown 복원 실패 위험 문서화) / 5 연혁 문단 제자리 정정(DOC-05).
- 판정 기록: `strip_non_syncable` 업로드 방향 편입(멀티 기기 base_url 미동기화)은 기존 4필드
  선례와 등가의 의도된 동작 변경으로 검토가 수용 — 부작용 아님(DOC-02 는 그 기록 위치 문제).
