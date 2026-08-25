# d-34 — AppError 캠페인(T2-E/J taxonomy) 계약 (2026-08-24)

> 정본: `2026-08-21-batch-consolidation-decision.md` §2 d-34("T2-E/J 369지점 taxonomy —
> 워크플로 1회 다단(설계→Rust→TS 다단) 순차·검토 1회전") / 감사 C9(:226-229)·T2-E(:350)·
> T2-J(:757)·R1#9·R2#10·F6#12·R8#15. 운영: /goal — 커밋·푸시 없음.
> **bindings 표면 순증**(오류 코드 채널 추가)은 큐 정본(사용자 확정 2026-08-21)이 d-34 의
> 본질로 승인한 범위로 판단해 진행하되, 델타를 §3 에 정밀 기록한다(additive 한정 — 기존
> 5변종·필드 형태 불변).

## 0. 착수 전 메인 실사 (2026-08-24)

- `error.rs` 38줄: `AppError` 5변종(Io·NotFound·InvalidArgument·Forbidden·Internal),
  `#[serde(tag="code", content="message")]` — TS 표면 `{code: 5종 union, message: string}`.
- 생성 지점 실측 **356**(49파일 — 감사 시점 369 에서 소폭 변동). 한국어 포함 지점 **98**
  (감사 101 — 사용자 노출 확정분). 나머지 ~258 은 영어 기술 문구.
- TS 소비 경계: `shared/api/unwrap-result.ts` 의 `IpcError extends Error`(code+message) 단일
  수렴 — 소비처는 대부분 `toast.error(error.message)` 직송(전수는 설계 단계 실사).
- T2-J 분기 불가 3축: `map_git_err`/`run_git`(R4#15)·provider HTTP(R6#21)·설치 취소(R7#16)
  가 전부 `Internal` 평탄화.

## 1. 범위·원칙 (추천안 자체 채택)

1. **설계 고정 원칙**: ① 기존 5변종·message 경로는 폴백으로 **보존**(미이관 지점 동작 불변)
   ② 코드화 채널은 **additive**(예: `Localized { kind, key, args, fallback }` 신규 변종 —
   정확한 형태는 설계 단계가 확정하되 기존 변종의 직렬화 형태 불변·`IpcError.code` 소비처
   실사 후 kind 보존 여부 결정) ③ 로케일 `error.*` 네임스페이스 신설 — 3언어(en·ko·ja)
   실번역·MESSAGE_NAMESPACES 등 4곳 동기 ④ 프론트 표시는 key 존재 시 `t(key, args)`, 부재
   시 `message` 폴백(단일 공용 헬퍼).
2. **이번 배치 이관 범위**: ⓐ T2-J 분기 3축(git·provider HTTP·설치 취소 — 프론트 분기
   가능한 코드 부여) ⓑ **한국어 생성 지점 98건 전수**(로케일 우회 해소 — infra 12건·remote
   WS 포함 여부 실사) — 각 지점을 코드화 변종+로케일 키로 이관. ⓒ 영어 ~258건은 **이관하지
   않음**(폴백 경로로 동작 불변) — taxonomy 소비 가능 상태로 이월 기록.
3. **검증 게이트**: bindings 재생성 diff 가 의도된 순증(신규 변종 표면)만인지 명시 대조 /
   로케일 3파일 키 파리티 / verify 전체 / 기존 오류 경로 테스트 무회귀.
4. 범위 외: 오류 상태코드/HTTP 매핑 재설계·기존 5변종 리네임·영어 지점 이관·프론트
   에러 UI 재설계.

## 2. 실행

- Phase D(설계, opus+xhigh 1): 변종 형태·코드 체계(도메인 접두 `GIT_*` 등)·로케일 키 규약·
  `IpcError`/toast 헬퍼 계약·98건+3축의 지점→코드→키 매핑 표 초안. 산출은 §3-D 에 기록.
- Phase R(Rust, sonnet+xhigh **순차 2**): R1 인프라(error.rs 확장+헬퍼+T2-J 3축) → R2 한국어
  98건 도메인 스윕(설계 매핑 표대로). Rust 한 시점 한 에이전트.
- Phase T(TS, sonnet+xhigh 1): unwrap-result 확장·공용 표시 헬퍼·로케일 3파일 키 추가(실번역)·
  소비처 전수 갱신.
- 검토 1회전(3렌즈: 동등성·폴백 계약 / 로케일·번역 품질 / 경계·표면 델타) + major 적대적 →
  수정 → 메인 2차(verify+vite+bindings 델타 검수). 커밋 없음.

---

## 3. Phase D 설계 확정 (2026-08-25 — wf_e8bd2aa7-33d, 전문 `scratchpad/d34-design.md`)

### 3.1 §0 실사 정정 (설계 실측 — 단일행 grep 이 다중행 format! 21건을 놓친 것이 원인)

생성 지점 356→**341**(비-test, test 포함 376)·한국어 98→**119**·infra 12→**28**(6파일)·영어 ~258→**222**. 한국어 변종 분포: InvalidArgument 66·Internal 41·NotFound 10·Forbidden 2. 로케일 3파일 각 **792키** 완전 파리티.

### 3.2 채택 설계 (확정안 A)

- `AppErrorKind`(5종 enum) + `LocalizedError { kind, key, args: BTreeMap<String,String>, fallback }` + `AppError::Localized(LocalizedError)` **newtype variant 추가** — 기존 5변종 직렬화 형태 불변. 생성 API = `AppError::localized(kind, key, fallback).with_arg(name, value)` 빌더(매크로 금지) + `AppError::kind()`.
- 키 규약 `error.<domain>.<slug>`(17개 domain — infra 는 관심사명: root_guard→path 등), 보간 `{{camelCase}}`, 동일 문구 키 통합(실측 중복 13종 32지점). `MESSAGE_NAMESPACES` 에 `("error", &[...])` 신설 — 파리티 테스트 2종이 4곳 동기를 기계 강제.
- TS: `IpcError.code` 는 **Localized 미노출**(경계에서 `message.kind` 로 정규화 — 기존 code 분기 4곳 보존), 표시 헬퍼는 `i18next.exists(key)` 필수 검사 후 `t(key,args)`/fallback.

### 3.3 미결 8건 — 추천안 자체 채택 (/goal)

1. fallback 언어 = **영어 기술 문구 신규 작성**(잔존 222 영어 지점과 동일 레지스터·fallbackLng 일치).
2. `map_git_err` kind = **Internal 고정**(+Auth/Certificate→Forbidden), 분류는 localeKey 로만 — 기존 code 분기 4곳(commit-file-diff 의 NotFound 해석 등) 의미 불변.
3. AppError 밖 인접 한국어 = **RemoteDenialPolicy::message 9건만 포함**(실제 Forbidden 으로 사용자 노출), LspInstallProgress 9·project warnings 3·vsix warning 1 등은 이월.
4. `remote-ws-client.ts` RemoteDisconnected = **Localized 형태로 정규화**.
5. LspInstallProgress 한국어(진행 표시줄) = **d-35 이월**(AppError 밖 bindings 델타).
6. 소비처 교체 = **78건 전수**(toast 72+지속 렌더 6— 지속 렌더 4곳은 훅 변형으로 언어 전환 대응). 내부 실패사유 3건 통일은 구현 판단·기록.
7. 계약 §0 수치 = §3.1 로 정정 완료.
8. `AppErrorKind` = **Rust enum 으로 export**(TS 는 bindings 소비).

### 3.4 구현 반영 필수 리스크 (설계 13건 중 하중)

- **specta rc.25 의 adjacently-tagged newtype-struct variant 출력 형태 미검증** — R1 첫 단계에서 스파이크(bindings 재생성으로 `{code:"Localized"; message: LocalizedError}` 형태 확인, 어긋나면 struct variant 대안으로 전환·기록).
- Rust 테스트 4개의 `matches!(e, AppError::Variant(_))` 단언 → `kind()` 기반 교체(root_guard 2·file/service 등).
- `RawFileReadError` 가 `super(error.message)` 로 객체를 받으면 "[object Object]" — 경계 정규화 필수(file_read_raw 가 이관 대상 Forbidden 2건에 닿음).
- remote `dispatch.rs` 직렬화 테스트에 Localized 케이스 추가(원격 경로 회귀 보호).
- `docs/ipc-contract.md:38` 의 AppError 표면 서술 갱신(기존 details 오기도 정정).

---

## 4. 구현 완료 기록 (2026-08-25 — wf_5a8482e9-019 순차 3단)

### 4-R1. 인프라 + T2-J 3축

#### R1 완료 기록 (2026-08-25)

#### 스파이크 결과 (§3.4 리스크 1)
cargo test 로 bindings 재생성 후 대조한 결과, AppError::Localized(LocalizedError) newtype variant 의 와이어 형태는 설계 §5 예측과 **정확히 일치**({code:"Localized"; message: LocalizedError}, 이름 있는 타입 export, 3-hunk·+16/-1). 단, 설계가 제시한 thiserror Display 문법 `#[error("{}", .0.fallback)]` 는 이 저장소의 실제 의존성 그래프에서 컴파일 에러(`expected an expression`, error.rs:46:19)를 냈다. 별도 스크래치 크레이트(thiserror 2.0.19 단독)로는 재현되지 않아, 원인은 이 crate 가 끌어오는 다중 syn 메이저버전 조합에서 thiserror-impl 의 `is_syn_full()` 런타임 프로브가 false 로 떨어지며 `scan_expr` 폴백 경로가 leading-dot 체이닝(`.0.fallback`)을 못 삼키는 것으로 격리됨(전용 spike 로 확인, 크레이트 자체 fix 는 이 세션 범위 밖). 대안: `LocalizedError` 에 수동 `impl Display`(fallback 반환)를 구현하고, `AppError::Localized` 변종은 기존 5변종과 동일한 `#[error("{0}")]` 숏핸드(이미 이 파일에서 검증된 경로, 파싱이 포맷 문자열 자체의 `take_int` 스캐너를 쓰므로 syn full 여부에 의존하지 않음)를 쓰도록 전환. 와이어 형태·Display 문자열 출력은 설계안과 100% 동일하고, 구현 세부만 다르다. 델타는 순증만(기존 5변종 표면 불변) — 계약 §1① 충족.

#### 구현
- `AppError::localized(kind, key, fallback)` + `.with_arg(name, impl Display)`(체이닝 빌더, 매크로 미사용) + `.kind() -> AppErrorKind`.
- `MESSAGE_NAMESPACES` 에 `("error", &[...19키])` 신설 — 이번 R1 축(ai.* 7 · git.* 11 · lsp.* 1)만 포함. ⓑ 100키(한국어 119건 전수)는 R2 가 같은 튜플에 이어 붙인다.
- en/ko/ja 792→811키(+19), 완전 파리티, ko/ja 는 자연스러운 실번역(기계 직역 아님).

#### T2-J 3축 이관
1. **git**(`map_git_err`/`run_git`): git2::ErrorCode/ErrorClass 로 9갈래 분류. kind 는 Internal 고정 + Auth/Certificate 만 Forbidden(설계 §3.3-2 그대로) — NotFound/InvalidArgument 는 어떤 분기에서도 나오지 않아 commit-file-diff.tsx:19,30·workspace-edit-applier.ts:76,78 의 기존 code 분기 4곳 의미 불변을 코드 레벨로 보장. `run_git` 2지점(spawnFailed·commandFailed)도 함께 이관 — 이 2지점은 ⓑ B-3 표에도 동일 행으로 등장하므로 R2 는 중복 이관하지 않는다.
2. **provider HTTP**: `domain/ai/providers/mod.rs` 에 `provider_http_error`/`provider_transport_error` 공용 헬퍼(2회 이상 룰 충족, 실사용 17지점) 신설. 상태코드 7갈래(401/403→unauthorized, 404→endpointNotFound, 408/504 또는 timeout→timeout, 429→rateLimited, 5xx(504 제외)→providerUnavailable, 기타4xx→requestFailed, transport→networkFailed). `detail` 은 헬퍼 내부에서 1회만 `mask_provider_error` 적용(호출부 중복 마스킹 제거). codex/ollama/omlx 3파일의 SSE 스트림 **내부** 에러(코덱스 192·197, 올라마 249·252, omlx 353 — extract_chat_text 의 error 필드·truncation)는 설계 매핑 표 밖이라 미접촉.
3. **설치 취소**: `infra/lsp_install.rs:245`·`domain/lsp/commands.rs:1089` 를 `error.lsp.installCancelled` 로 이관. `commands.rs:1087` 의 `LspInstallProgress.message`(진행 표시줄 채널)는 AppError 밖이라 계약 범위 밖 — d-35 이월 유지, 미접촉.

#### 리스크 반영
- `remote/dispatch.rs` 직렬화 테스트에 Localized 케이스 1건 신규 추가(green) — 원격 경로 회귀 보호.
- `matches!(e, AppError::Variant(_))` 4곳(root_guard 2·file/service·search)은 전부 ⓑ 100키 스윕(R2) 대상 함수에 걸려 있고 R1 은 그 함수들을 건드리지 않았으므로 현재 전부 green 유지 — kind() 기반 교체는 해당 호출부가 실제로 Localized 로 바뀌는 시점(R2)에 함께 하는 것이 맞다(지금 미리 바꾸면 대응 코드 변화 없는 공전 diff가 되어 최소변경 원칙 위반).
- `RemoteDenialPolicy::message` 9건은 R2 몫으로 완전히 미접촉(중복 없음, git diff 로 확인).
- `docs/ipc-contract.md:38` 을 6변종 실물 형태로 갱신 + 기존 `details?` 오기 정정. 단 프론트 `describeIpcError`/`IpcError.code` 정규화는 아직 미구현(Phase T)이므로 "이미 되어 있다"고 서술하지 않고 진행 중으로 명시.

#### 검증 게이트 결과
cargo fmt --all --check(0 diff) → cargo test --workspace(전부 green, 1073+3+6+17) → cargo clippy --workspace --all-targets -D warnings(경고 0) → bindings 델타(+16/-1, 3-hunk, 의도된 순증만) 전부 실측 통과.

**openIssues**:
- R2(한국어 생성 지점 119건 → 100키 도메인 스윕)는 이번 세션 범위 밖 — 미착수. root_guard.rs·search/service.rs 의 matches!(AppError::Variant) 4곳은 R2 가 해당 호출부를 Localized 로 옮길 때 kind() 기반 단언으로 함께 교체해야 함(§3.4 리스크 2).
- RemoteDenialPolicy::message 9건(dispatch.rs:323-356) 이관은 R2 몫으로 명시 이월 — 미착수.
- Phase T(TS): unwrap-result.ts/IpcError 확장(code 정규화)·entities/file/file.raw.ts RawFileReadError 수정·ipc-error-message.ts(describeIpcError/useIpcErrorMessage) 신설·소비처 78건 교체가 전부 미착수. Phase T 완료 전까지는 R1 이 이관한 3축(git/provider/lsp취소)의 에러도 toast 에 fallback 영어 문구가 로케일 번역 없이 그대로 뜬다 — 사용자 노출 관점에서 Phase T 선행 필요를 인지.
- LspInstallProgress.message(진행 표시줄) 한국어는 d-35 이월로 재확인, 이번 세션에서도 미접촉 유지.
- map_git_err 의 fallback 텍스트를 사용자가 실제로 볼 가능성은 낮음(en 카탈로그에 전 키가 이미 존재) — 로그(log::warn!) 노출 시에는 영어로 정상 출력됨을 Display 구현으로 확인.

### 4-R2. 한국어 생성 지점 전수 스윕

#### 3.6 Phase R2 실행 기록 (한국어 생성 지점 전수 스윕, Rust)

#### 실행 결과

- 설계 매핑 표 B(§매핑 표, 한국어 119지점→100키) 전수 이관 완료. 이 중 2지점(git.spawnFailed·git.commandFailed, `run_git`)은 R1의 T2-J 축1(`map_git_err`/`run_git`)에서 이미 이관돼 있어 재확인만 수행.
- §3.3-3 미결(추천안 채택) 대로 `RemoteDenialPolicy::message` 9건을 추가 이관 — `String` 반환에서 `AppError` 직접 반환으로 시그니처를 바꿔 kind(Forbidden)·key·fallback을 한 지점에서 결정하도록 재구성(`domain/remote/dispatch.rs`).
- 동일 문구 키 통합(설계 명시 13종 32지점)을 전부 확인: `error.file.trashFailed`(4)·`error.git.noChanges`(4)·`error.plugin.alreadyInstalled`(4)·`error.git.bareUnsupported`(2)·`error.git.hunkNotFound`(2)·`error.git.pathOutsideRepository`(2)·`error.lsp.installCancelled`(2, R1 기완료)·`error.vsix.openFailed`(2)·`error.vsix.unzipFailed`(2)·`error.vsix.packageJsonParseFailed`(2)·`error.vsix.entryPathEscape`(2)·`error.archive.entryReadFailed`(2)·`error.path.invalid`(2) — 전부 동일 키로 정확히 통합됨.
- 로케일 3파일(en/ko/ja)에 신규 106키 추가(792→917, 완전 파리티) + `domain/locale/service.rs`의 `MESSAGE_NAMESPACES("error")` 를 19→125 슬러그로 갱신. ko는 이관 전 원문 한국어 문구를 `{{camelCase}}` 템플릿으로 재사용/다듬었고, en은 잔존 222 영어 지점과 동일 레지스터의 신규 기술 문구, ja는 신규 번역.
- 검증 게이트(계약 §7) 전부 그린: `cargo test -p taide locale::`(내장 3종 로케일 키 집합 동일·en 키 전부 `MESSAGE_NAMESPACES` 포함) · `cargo test typescript_바인딩을_생성한다` · `collect_commands_매크로_출력과_dispatch_테이블은_커맨드_이름_집합이_일치한다` · `에러는_code와_message만_가진_평면_객체로_직렬화된다`/`localized_에러는_message가_객체로_직렬화된다`(R1 기추가) · `cargo test --lib` 전체 1073 passed/0 failed · `cargo fmt --check` 클린 · `cargo clippy --all-targets` 경고 0건.
- 한글 잔존 재확인: `AppError::{Io,NotFound,InvalidArgument,Forbidden,Internal}(` 생성식 내부에 괄호균형 파서로 한글 검색 → 0건. bindings.ts 델타는 R1의 기존 3-hunk(+16/-1)에서 변동 없음(순수 데이터 이관이라 표면 불변, 계약 §3 검증 게이트 충족).

#### 설계 문서가 놓친 회귀 1건 (신규 실측 발견)

설계 §리스크의 "Rust 테스트 4개의 `matches!(e, AppError::Variant(_))` 단언" 목록(root_guard.rs 2곳·file/service.rs·search/service.rs)은 실측 결과 **불완전**했다. `cargo test --lib` 전체 실행에서 `domain/theme/service.rs`의 테스트 2건(`delete_theme는_경로_구분자가_섞인_아이디로_저장소_밖_파일을_지울_수_없다`·`load_theme는_경로_구분자가_섞인_아이디를_거부한다`)이 추가로 실패했다. 두 테스트는 `theme::service::delete_theme`/`load_theme`가 내부적으로 호출하는 `root_guard::ensure_safe_component`의 반환값을 `matches!(result, Err(AppError::InvalidArgument(_)))`로 단언하고 있었는데, `ensure_safe_component`가 이번에 `error.path.invalidIdentifier`로 이관되며 variant가 `AppError::Localized(...)`로 바뀌어 패턴이 더 이상 매치하지 않았다. `AppErrorKind` 도입(§리스크 완화안)과 동일한 방식으로 `assert_eq!(result.unwrap_err().kind(), AppErrorKind::InvalidArgument)`로 교체해 해결했다(테스트 전용 스코프에 `AppErrorKind`를 임포트해 non-test 빌드의 미사용 경고도 함께 제거). 설계 문서의 "나머지 31개 test-region 단언은 대응 생성 지점이 전부 영어라 무영향" 판정에서 이 1건이 누락돼 있었다는 사실을 기록으로 남긴다 — 리스크 목록은 4건이 아니라 **5건**이었다.

#### 이관하지 않은 것 (범위 밖, 실측 확인만)

- `domain/lsp/commands.rs`의 `LspInstallProgress.message`/세션 상태 메시지 9곳(§미결5, d-35 이월) — AppError 밖 채널이라 설계 범위 밖. `error.lsp.installCancelled`로 이관된 AppError 채널과는 별개로, 이 진행 표시줄 채널은 여전히 한국어 원문을 emit함(화면에 언어가 섞이는 문제는 여전히 남아 있음 — d-35 대상).
- `domain/project/commands.rs:170`, `domain/project/service.rs:189,211`의 `warnings: Vec<String>` 3건, `domain/vsix/service.rs`의 log::warn! 1건, `domain/remote/serving.rs:31,43,129`의 HTTP 응답 본문 3건, `domain/remote/ws.rs:97`의 WS 채널 종료 사유 1건, `domain/remote/dispatch.rs:197`의 직렬화 실패 폴백 리터럴 1건, `domain/lsp/manifest.rs`의 번들 매니페스트 검증(프로그래머 오류) 9건 — 전부 §미결3 확정대로 손대지 않음(d-35 이월 또는 범위 밖으로 기록만).
- 영어 지점 222건은 전혀 건드리지 않음(폴백 경로 그대로 동작).

#### 다음 단계(Phase T)에 필요한 사실

- `IpcError.code` 정규화(§4.1 TS 계약)와 `describeIpcError` 헬퍼는 아직 미착수 — Rust 쪽 `AppError::Localized`/`AppErrorKind`/`.kind()` API 는 R1에서 이미 완성돼 있고 이번 R2는 그 위에 데이터(119+9건)만 얹었다.
- `RemoteDenialPolicy` 9키는 `error.remote.denied<PolicyName>` 형태로 신설되었고 args는 `command`(명령 이름) 하나뿐이다.
- 로케일 카탈로그 최종 상태: en/ko/ja 각 917키, `MESSAGE_NAMESPACES` 의 `"error"` 네임스페이스가 125슬러그로 최신화됨.

**openIssues**:
- d-35 이월 확정 항목(설계 §미결에서 이미 결정된 사항, 재확인만): LspInstallProgress.message 9건(진행 표시줄 한국어 잔존 — AppError 코드화와 별개 채널), project/vsix/remote 인접 한국어 27건 중 나머지 18건, remote-ws-client.ts:50 의 RemoteDisconnected TS 정규화.
- Phase T(TS) 미착수: unwrap-result.ts 확장·describeIpcError/isIpcErrorKey/useIpcErrorMessage 헬퍼·소비처 78건 교체·file.raw.ts RawFileReadError 정규화가 전부 남아있음.
- 설계 문서의 '나머지 31개 test-region 단언 전부 영어라 무영향' 판정이 실측과 1건 어긋났음(theme/service.rs) — 향후 유사 캠페인에서는 개별 확인 대신 전체 테스트 스위트 실행으로 검증할 것을 권고.

### 4-T. TS 경계·표시 헬퍼·소비처 전수

d-34 Phase T(TS 경계·표시 헬퍼·소비처 전수, sonnet+xhigh 1) 완료.

**경계**: `shared/api/unwrap-result.ts` 에 `normalizeAppError`(export, entities 재사용 가능)를 신설해 `AppError` 6변종을 `{code: AppErrorKind, message, localeKey, localeArgs}` 로 평탄화 — `Localized` 는 `kind→code`·`fallback→message`·`key`/`args` 노출, 나머지 5변종은 그대로 통과. `IpcError.code` 타입을 `AppError['code']`→`AppErrorKind` 로 좁혔다(`Localized` 자체는 `code` 로 노출되지 않음). `entities/file/file.raw.ts` 의 `RawFileReadError` 도 동일 normalize 를 재사용하도록 고쳐 "[object Object]" 리스크(§3.4 리스크 4 — `file_read_raw` 가 `infra/root_guard.rs:50,69` 의 Forbidden 2건 이관지점에 닿음)를 제거했다. 기존 `IpcError.code` 분기 4곳(`workspace-edit-applier.ts:76,78` · `commit-file-diff.tsx:19` · `snippet-editor.tsx:71,75`)을 실측 재확인 — 전부 무수정 통과.

**표시 헬퍼**: 신설 `shared/lib/ipc-error-message.ts` — `describeIpcError`(일회성, 모듈 싱글톤 `i18next` 사용)·`useIpcErrorMessage`(지속 렌더, `useTranslation` 기반이라 언어 전환에 반응)·`isIpcErrorKey`. 둘 다 계약 §1④ 대로 `i18next.exists(localeKey)` 필수 검사 후 `t(localeKey, localeArgs)`, 미등록/비-Localized/비-Error 는 `message`/`String(error)` 폴백. `shared/constants/error-key.ts` 신설(`ERROR_KEY.LSP_INSTALL_CANCELLED`).

**소비처**: 실측 재확인한 78건(72 toast + 6 지속 렌더) 전수 교체. grep 을 "error.message" 리터럴로만 1차 수행했다가 변수명이 다른 4곳(`diff-pane.tsx:108` mutationError · `use-editor-file-persistence.ts:278` saveError · `use-editor-git-gutter-and-conflicts.ts:91,124,153,252` mutationError/compareSidesErrorValue)이 누락됨을 2차 광역 grep(`\.message\)`)으로 발견해 포함 — 설계 표의 72건과 실물 위치가 파일 이동(bridge/ 하위 재구조화 등 선행 배치 반영)만 다를 뿐 수량은 일치. 지속 렌더 훅 4회(`editor-pane.tsx:265` · `terminal-session.tsx:81,124`(state 를 `string`→`unknown` 으로 바꿔 원본 에러를 보관, 렌더 시점에 훅으로 재번역) · `vsix-import-dialog.tsx:179`)는 설계 §4.2 그대로 적용. `use-explorer-entry-crud.ts:94,136` 은 이미 문자열로 resolve 해 `useState` 에 저장하는 기존 패턴이라 함수형 `describeIpcError` 로 처리(훅 도입 시 상태 타입 리팩터가 추가로 필요해 범위 확대 — 설계 §4.2 가 명시한 "4회 사용" 과도 일치). `settings-lsp-section.tsx:22` 의 `installLspServer` onError 에 `isIpcErrorKey(error, ERROR_KEY.LSP_INSTALL_CANCELLED)` 취소 가드를 달아 사용자 스스로 취소했을 때 빨간 토스트가 뜨는 문제(R7#16)를 해소했다. `plugin-uninstall-dialog.tsx`/`plugin-install-button.tsx` 의 `error.message || t(fallback)` 패턴은 빈 문자열 방어 의도를 살려 `describeIpcError(error) || t(fallback)` 형태로 보존했다.

**내부 실패사유 3건 판단**(계약 §미결 6): `workspace-edit-applier.ts:71`·`lsp/adapters/code-action.ts:101`·`lsp/client.ts:165` 는 통일하지 않고 제외했다. 근거 — 셋 다 사용자 UI 가 아니라 LSP `workspace/applyEdit` 실패 사유(서버로 되돌아가는 프로토콜 데이터)·JSON-RPC 에러 응답(서버로 나가는 wire 데이터)으로 흐르므로, `describeIpcError` 를 적용하면 로케일화된(한국어/일본어일 수 있는) 텍스트가 외부 프로토콜 페이로드에 섞여 나가는 회귀를 만든다.

**remote-ws-client.ts**: `RemoteDisconnected` 를 `{code:'Localized', message:{kind:'Internal', key:'error.remote.disconnected', args:{}, fallback:'The remote connection was lost.'}}` 형태(`AppError` 로 타입 단언, `satisfies AppError`)로 정규화(§3.3-4). 카탈로그에 해당 키가 없음을 실측 확인(R2 산출물 92→100/106 신규키에 remote.disconnected 없음) — **유일한 src-tauri 예외**로 en/ko/ja 3파일 + `domain/locale/service.rs` 의 `MESSAGE_NAMESPACES`("error" 네임스페이스, `deniedUnreachableDesktopWindow` 와 `passwordTooShort` 사이 알파벳 순 삽입) 4곳을 동기 반영하고 `cargo test -p taide domain::locale` 16/16 green(양방향 파리티 테스트 포함) 재확인·기록. 실측 카탈로그 792(계약 §0 기준선) → **918**(error.* 네임스페이스 125→**126**) × 3파일, 완전 파리티.

**문서**: `docs/ipc-contract.md` §1 의 AppError 표면 서술에서 "`Localized` 표시 헬퍼는 이관 진행 중" 각주를 제거하고, 프론트 경계 정규화(`normalizeAppError`/`IpcError`/`RawFileReadError`)와 표시 헬퍼(`describeIpcError`/`useIpcErrorMessage`) 계약을 구체 서술로 갱신했다. 기존 "details" 오기(§3.4 리스크)는 R1 단계에서 이미 정정돼 있음을 실측 확인(재수정 불필요).

**검증 게이트**(계약 §7 대응): `typecheck` 0 error · `prettier --check` 전체 pass · `bun test` 전체 **1476 pass**(0 fail, 기존 1463→+13: `unwrap-result.test.ts` +3(`normalizeAppError` 2건 + Localized `IpcError` 1건) · 신설 `ipc-error-message.test.ts` +8 · 신설 `file.raw.test.ts` +2) · `eslint` (이번 태스크가 실제로 건드린 49개 TS 파일 전수) 0 error(경고 3건은 전부 이 작업과 무관한 기존 코드: `untitled-pane.tsx`/`use-editor-file-persistence.ts` 의 `persistMirror` exhaustive-deps 경고, `commit-graph.tsx` 의 `useVirtualizer` React Compiler 비호환 경고) · `cargo test -p taide domain::locale` 16/16 green.

**미결/이월**: (1) `remote-ws-client.ts` 의 새 Localized 정규화 자체를 커버하는 자동 회귀 테스트는 미신설 — 이 저장소에 WebSocket 테스트 하네스가 없어(기존 `remote-ws-client.test.ts` 도 순수 함수 `isSessionExpiredClose` 만 커버) `rejectAll`/`invoke` 내부 로직은 단위 테스트 대상 밖이었고, 이번에도 같은 제약을 유지했다(타입체크 + 수동 코드 리뷰로 형태 검증). (2) `LspInstallProgress.message`(설치 진행 표시줄) 한국어 잔존은 계약 §미결 5·설계 §미결 대로 d-35 로 이월(범위 밖, AppError 밖 별도 이벤트 페이로드).

**openIssues**:
- remote-ws-client.ts 의 RemoteDisconnected Localized 정규화는 WebSocket 테스트 하네스 부재로 자동 회귀 테스트가 없음 — 타입체크·수동 검증으로만 확인
- LspInstallProgress.message(설치 진행 표시줄) 한국어 잔존은 d-35 이월(범위 밖, 계약 §미결 5)
- 내부 실패사유 3건(workspace-edit-applier.ts:71·lsp/adapters/code-action.ts:101·lsp/client.ts:165)은 LSP/JSON-RPC 프로토콜 채널로 흐름을 확인해 describeIpcError 통일 대상에서 의도적으로 제외 — 이후 회고 시 재검토 여지

## 5. d-34 검토 후속 수정 일괄 (2026-08-25 — 3렌즈 검토 major 2·minor 11 전건 반영)

3렌즈(동등성·폴백 / 로케일·번역 품질 / 경계·표면 델타, opus+xhigh) 검토가 보고한 major 2건(중복 3건 포함해 실질 2지점)·minor 11건을 전건 수정. 검증 게이트(`cargo fmt --all --check`→`cargo test --workspace`→`cargo clippy --workspace --all-targets -D warnings`→`bun run typecheck`→`bunx prettier --check`(변경분)→`bun test`(전체)→`bunx eslint`(변경분)) 전부 통과, bindings.ts 추가 델타 0(재생성 후 기존 +16/-1 그대로).

### major

1. **`error.ai.providerUnavailable` 의 `{{status}}` 미치환** (D34-L1-01/L2-1/L3-01) — `providers/mod.rs`의 `provider_http_error` 를 raw `u16` 매칭에서 `reqwest::StatusCode` 상수 매칭(`UNAUTHORIZED|FORBIDDEN`·`NOT_FOUND`·`REQUEST_TIMEOUT|GATEWAY_TIMEOUT`·`TOO_MANY_REQUESTS`·`is_server_error()`·`_`)으로 전환하면서(L3-06 겸함), 5xx 갈래에 빠져 있던 `.with_arg("status", status_code)` 를 형제 갈래와 동형으로 추가했다. 재발 방지: 카탈로그 `{{placeholder}}` ⊇ `with_arg` 인자 파리티를 기계로 강제하는 전용 테스트는 이번 배치에 신설하지 않았다 — Rust 매크로/derive 만으로는 문자열 리터럴(카탈로그 값)과 `.with_arg` 체인을 정적으로 대조할 수단이 없고, 런타임 스캔 테스트를 만들려면 130개 `AppError::localized` 호출부를 소스 파싱해야 해 이번 수정 범위를 넘어선다고 판단했다 — `docs/quality-assurance`에 체크리스트로 이월(§6).
2. **`terminal.ipc.ts` 의 raw invoke 경계 미정규화** (L3-02) — `spawnPty`/`attachPty`가 `Channel` 인자 때문에 `unwrapResult`를 못 타고 raw `invoke`를 직접 호출해, 거부값이 `IpcError`가 아닌 bare `AppError` 객체로 올라와 `describeIpcError`가 `String(error)`(`"[object Object]"`)로 떨어지던 문제. `file.raw.ts`의 `readFileRaw` 선례대로 `invokeRaw` 헬퍼를 신설해 `catch`에서 `isAppError(error)` 가드 후 `new IpcError(error)`로 재던지도록 정규화했다. `isAppError`는 2번째 사용처(`file.raw.ts`+`terminal.ipc.ts`)가 생겨 `shared/api/unwrap-result.ts`로 승격(공통화 "2회 이상" 룰)하고 `file.raw.ts`는 그 export를 재사용하도록 고쳤다. `terminal.ipc.test.ts` 신설(2건) — `@tauri-apps/api/core`를 `mock.module`로 대체(`invoke`는 mock, `Channel`은 `window.__TAURI_INTERNALS__`가 없는 `bun:test` 환경에서 실제 클래스가 즉시 던지므로 최소 페이크로 교체, 나머지 export는 실제 모듈에서 스프레드)해 `spawnPty`의 정규화 경로를 실측 검증했다. `terminal-session.tsx`의 표시(토스트 2곳·`useIpcErrorMessage` 실패 패널)는 코드 변경 없이 이 정규화만으로 정상 복원됨을 확인.

### minor

3. **provider 5키 detail 미표시** (D34-L1-02) — baseline이 마스킹된 프로바이더 사유를 그대로 보여주던 것과 달리 `unauthorized`·`endpointNotFound`·`timeout`·`rateLimited`·`providerUnavailable` 5키는 `{{detail}}`을 args로 실어보내고도 카탈로그 문구가 렌더하지 않았다. **결정: baseline 동등성을 우선해 5키 en/ko/ja 문구 끝에 `: {{detail}}`을 추가**(반대 옵션 — `.with_arg("detail", …)` 제거 — 는 채택 안 함, 프로바이더가 준 실패 사유는 사용자에게 보여줄 가치가 있다고 판단). `requestFailed`/`networkFailed`와 레지스터 통일.
4. **표시 헬퍼가 `IpcError`에만 게이트** (D34-L1-03/L3-03) — `describeIpcError`/`useIpcErrorMessage`의 좁히기를 `instanceof IpcError`에서 구조 판정(`error instanceof Error && 'localeKey' in error`)으로 확대해 `RawFileReadError`(및 향후 동일 형태 boundary 에러 전부)의 번역 경로를 연결했다. `ipc-error-message.test.ts`에 구조 판정 게이트 테스트 1건 추가.
5. **`RemoteDenialPolicy` 관련 doc·메서드명 stale** (D34-L1-04) — `dispatch.rs`의 타입/메서드 doc 주석에서 "Korean text"·"free-text 메시지" 서술을 제거하고 로케일 키 기반 서술("`error.remote.denied<Variant>` 9키, args는 `command` 하나")로 갱신, 메서드를 `message`→`denial_error`로 개명(반환 타입 `AppError`와 정합, 호출부 `denial_response` 1곳 갱신).
6. **`assert_forbidden_denial` 실패 컨텍스트 소실** (D34-L1-05) — `name: &str` 파라미터를 추가해 두 `assert_eq!`에 `"{name} 의 거부 응답 code/kind 가 …여야 한다"` 컨텍스트를 복원, 9개 호출부(단일 호출 6곳 + 루프 3곳) 전부 갱신.
7. **고아 로케일 키 2개** (D34-L1-06) — `settings.themeImportSaveFailure`·`settings.pluginImportVsixFailed`를 전 저장소 grep으로 소비처 0 재확인 후 en/ko/ja 3파일 + `locale/service.rs`의 `MESSAGE_NAMESPACES` 2곳에서 제거.
8. **ko 신규 키 레지스터 불일치** (L2-3) — 명사형 종결(`…실패`/`…오류`) 18키(`error.secret.*` 4·`error.archive.*` 5·`error.lsp.*` 4·`error.plugin.manifestParseFailed`·`error.vsix.*` 2·`error.git.commandFailed`·`error.ai.networkFailed`)를 하우스 스타일 완결 문장(`…하지 못했습니다`/`…이 발생했습니다`)으로 통일. placeholder 집합은 키마다 그대로 보존(en/ja·자리표시자 파리티 무영향).
9. **ko 조사 이형태 공백 누락** (L2-4) — `error.plugin.manifestNotFound`("{{file}}을(를)"→"{{file}} 을(를)")·`error.ai.providerUnavailable`("{{provider}}을(를)"→"{{provider}} 을(를)") 2건을 기존 하우스 스타일(공백 분리)에 맞췄다.
10. **마침표 처리 3언어 불일치** (L2-5) — `en.json`의 `error.remote.disconnected`("The remote connection was lost." → 마침표 제거)와 `ko.json`의 `error.git.indexNotClean`(마침표 제거)을 각 키의 다른 두 언어와 통일. `remote-ws-client.ts`의 fallback 리터럴도 동기(마침표 제거) — 신규 `ERROR_KEY.REMOTE_DISCONNECTED` 상수를 함께 참조하도록 고쳐 항목 12와 겸행.
11. **`useIpcErrorMessage`가 `shared/lib`에 위치** (L3-04) — 훅을 `shared/hooks/use-ipc-error-message.ts`로 분리(이 저장소의 `use*` 훅은 전부 `shared/hooks` 관행), 순수 함수(`describeIpcError`·`isIpcErrorKey`·구조 판정 게이트 `hasLocaleKey`)는 `shared/lib/ipc-error-message.ts`에 잔류하고 `hasLocaleKey`를 export해 훅이 재사용하도록 공유. 소비처 3곳(`editor-pane.tsx`·`terminal-session.tsx`·`vsix-import-dialog.tsx`) import 경로 갱신.
12. **로케일 키 리터럴 이원화** (L3-05) — `shared/constants/error-key.ts`에 `ERROR_KEY.REMOTE_DISCONNECTED`를 추가하고 `remote-ws-client.ts`가 인라인 리터럴 대신 이 상수를 참조하도록 통일(항목 10과 함께 처리).
13. **HTTP 상태코드 raw 리터럴 매칭** (L3-06) — 항목 1의 `provider_http_error` 재작성에서 `reqwest::StatusCode` 상수/`is_server_error()`로 전환하며 함께 해소.

### 검증 실측 (2026-08-25)

- `cargo fmt --all --check`: clean (최초 1회 `cargo fmt --all`로 `assert_forbidden_denial` 포맷 정리 후 재확인 clean)
- `cargo test --workspace`: **1073 passed**(`-p taide --lib`, 0 fail) + `domain_boundaries` 3 + `session_restore` 6 + `taide_cli` 17, 전부 pass. `domain::locale`(16/16)·`domain::ai::providers`(44/44)·`domain::remote::dispatch`(30/30) 개별 재확인.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warning
- `bun run typecheck`(`tsc --noEmit`): 0 error
- `bunx prettier --check`(변경분 15파일): clean
- `bun test`(전체): **1481 pass**, 0 fail(기존 1476→+5: `unwrap-result.test.ts`의 `isAppError` +2, `ipc-error-message.test.ts`의 구조 판정 게이트 +1, 신설 `terminal.ipc.test.ts` +2)
- `bunx eslint`(변경분 13개 TS 파일): 0 error/warning
- bindings 델타: `typescript_바인딩을_생성한다` 재실행 후 `git diff --stat src/shared/api/bindings.ts` = 이전과 동일한 `+16/-1`(항목 1·2의 Rust 변경은 내부 리팩터라 wire 표면 무영향) — 추가 델타 0 확인

### 6. 이월 — `docs/quality-assurance`로

- 카탈로그 `{{placeholder}}` ⊇ Rust `with_arg` 인자 파리티를 기계로 강제하는 테스트(항목 1의 재발 방지 대상)는 이번 배치에 없음. 130개 `AppError::localized` 호출부 전수를 소스 파싱해야 해 별도 작업으로 분리 — `docs/quality-assurance/2026-08-25-locale-arg-parity-checklist.md` 참고.

### §5 보충 — 적대적 검증 생략 판정 (메인, 2026-08-25)

major 2건({{status}} 누락·raw invoke "[object Object]")은 ① 3렌즈 독립 수렴(+각자 실행 재현)
② 메인 실물 직접 확인(5xx 분기 with_arg 대조·terminal.ipc raw invoke 실물) ③ 수정이 형제
분기/기존 선례(file.raw.ts) 동형의 기계적 1~수 줄 — 세 근거로 **기계 확정**되어 d-30 §4
선례("기계 대조로 확정 — 적대적 라운드 불요")를 적용, 적대적 검증을 생략했다. 메인 2차
verify(bun 1481·Rust 1099·vite exit 0·bindings 델타 +16/-1 불변)로 종결.
