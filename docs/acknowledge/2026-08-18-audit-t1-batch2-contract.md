# 아키텍처 감사 T1 정비 — 2차 배치(T1-G·asset scope·T1-F) 수정 계약 (2026-08-18)

> 감사 보고서 정본: `docs/quality-assurance/2026-08-18-architecture-audit.md`(§6.2 T1 묶음·§C10·§C16).
> 사용자 결정(2026-08-18): T1 2차 배치 착수. 1차(T1-E·T1-J·T1-B) 완료(main=682ca6a) 위.
> 이번 배치 = **보안 하드닝(T1-G) + asset scope 재등록(X1#7, 1차 이월분) + 레이어 이동(T1-F)**.
> 프론트 대규모(T1-C 서버상태·T1-D 레지스트리)는 3차로 분리. 감사 하중 주장은 2단 종합 재검증 완료.

## 1. 2차 배치 범위

### T1-G 인프라·보안 하드닝 (C10·C11 일부, Rust)
전부 국소·되돌릴 수 있는 보안 강화. 앱 실기 없이 검증 가능한 항목 우선.
- **R3#4**: remote nonce 쿠키에 `Secure` 누락 — 세션 쿠키는 조건부 부여인데 nonce 는 미부여. 비대칭 해소
  (loopback insecure 판정과 정합 — is_insecure_connection 경로 확인).
- **R3#5**: `/__taide/file` 응답에 CSP·`X-Content-Type-Options: nosniff` 부재 — 동일 오리진 SVG 실행 표면.
  Range 라우트 응답 헤더에 추가.
- **R2#11**: shell_integration 이 셸 source 대상 스크립트를 공용 `/tmp` 기본 umask 로 기록 — 0o600 권한 명시
  (다른 사용자 읽기 차단).
- **R2#4**: `resolve_owning_project` 가 HashMap 순회 순서 의존 — 결정적 선택(최장 prefix 등 명시 규칙).
- **R6#7·R2#6·R2#5**: `create_outbound_http_client` 매 호출 생성 → 프로필별 싱글톤(인라인 컴플리션 타이핑마다
  호출 경로 체감). 동일 근원 3건.
- **R7#13**: lsp manifest `expect()`(IPC 핸들러 경로 패닉 잔존) 제거 + 매니페스트 캐시(매 호출 파싱 회피).
- **R6#4**: `ai_inline_complete` 요청 바이트 상한(무제한 입력 차단).
- **R3#6**(난이도 평가): 원격 파일 전체 메모리 적재 → 스트리밍. 규모 크면 이 항목만 3차 분리 보고.

### asset scope 재등록 (X1#7, T1-J 1차 이월분 — 보안·자원 회수)
닫힌 프로젝트 트리를 webview 가 계속 읽는 표면. Tauri asset scope 는 add-only 라 forbid 불가(1차 확인).
- **근본안**: `register_uri_scheme_protocol("asset", ...)` 로 asset 읽기를 열린 프로젝트 root 집합 상태 기반
  자체 핸들러로 재등록(닫으면 집합 제거→즉시 반영). convertFileSrc·CSP(asset:/http://asset.localhost) 재사용.
- **필수 요건**: 내장 핸들러의 **Range 요청(video/audio 탐색)** 을 안전하게 재현해야 함. root_guard 로 경로
  봉쇄. **위험 재평가 필수**: 파일 서빙은 보안 민감 표면이고 에이전트는 앱 실기 불가(preview-pane video/audio
  실동작을 못 켬). Range·CSP·convertFileSrc 정합에 확신이 안 서면 **재이월 + qa6 사용자 실기 항목화**(무리한
  강행 금지 — 1차와 동일 원칙). 구현 시 KNOWN ISSUE(실기 미검증)로 명시.

### T1-F 레이어 배치 정정 (C1, 프론트 — 저위험 기계적)
eslint 가 방향만 검사하고 레이어 성격(query 보유)은 미검사 → 한 슬라이스가 두 레이어에 갈림. import 경로만.
- **features→widgets 승격**(query/mutation 소유): agent-hooks-project-row·agent-hooks-project-list·
  agent-cli-status-row·use-zen-mode.
- **widgets→features 강등**(순수 props 컴포넌트): tab-context-menu·sortable-tab·file-tree·problems-panel·
  search-panel·outline-panel·plugin-list-body·vsix-import-grammars-section·snippet-entry-editor·snippet-file-list.
- **features→shared 강등**(React 무관·소비처 widgets): vsix-theme-import·selection-line-range.
- barrel 금지·파일 직접 경로 import 유지. 이동 후 `architecture.md §5`(fsd.md §2.1)에 배치 기준 명문화.
- **판정 재확인**: 각 파일이 실제로 그 레이어 이동이 맞는지(감사 C1 근거) 이동 전 소비처·의존 확인. 애매하면
  더 아래 레이어(fsd.md §2.1) — 무리한 이동 금지, 확신 없는 파일은 보고 후 보류.

## 2. 실행 구조

- **Phase R1(Rust 하드닝 단독, sonnet+xhigh)**: T1-G 전량. 각 항목 회귀 테스트. R3#6 난이도 평가·분리 보고.
- **Phase R2(Rust asset scope 단독, R1 순차)**: X1#7 재등록. **난이도 재평가 먼저** — Range/CSP/convertFileSrc
  정합 확신 없으면 재이월+실기 항목화(코드 미변경 또는 KNOWN ISSUE 표기). 구현 시 프로토콜 핸들러+root_guard.
- **Phase F(프론트 레이어 이동 단독, sonnet+xhigh)**: T1-F. **파일 이동은 import 그래프 전역 영향이라 단독**
  (병렬 시 충돌). git mv + import 경로 갱신 + props 리프팅. tsc·eslint 로 방향 위반 0 확인.
- **Phase D 통합(단독)**: architecture.md(§5 배치 기준·§6.3 자원)·문서 + verify + vite build.
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — 보안: nonce Secure·CSP·nosniff·asset 핸들러 경로 봉쇄·
  http 싱글톤 동시성 / 설계: 레이어 이동 정당성·순환 미발생) → 적대적 → 수정 → 메인 2차 → 커밋(dev).

## 3. 범위 밖 (T1 3차 이후)

- T1-C 서버상태(effect fetch·무효화 책임·캐시 store 전용, 14건)·T1-D 레지스트리 정리(bridge 싱글톤·멀티윈도우
  격리, 17건 최대)·T1-A 잔여(monaco 키바인딩·키맵 브리지 프로바이더).
- T1-H 락 IO(위험 높음)·T1-I 도메인 경계(C13)·T1-K 원격 기본거부·X-A 배선·T2 전체·T2-E AppError(별도 캠페인).
- **T1-H·T1-I 착수 시 위험 재고지**(사용자 결정 — T1 포함이나 신중).

## 4. 완료 조건

- `bun run verify` 전체 + vite build. 신규 회귀 테스트 통과·레이어 방향 위반 0(eslint)·bindings 정합.
- 4렌즈+적대적+메인 2차. 초점: nonce Secure/CSP/nosniff 실효·asset 핸들러 경로 봉쇄 우회 불가(root_guard·..
  ·심링크)·http 싱글톤 동시성·shell temp 권한·레이어 이동 후 순환/역참조 0·asset 재등록 시 CSP·convertFileSrc
  무변경(또는 재이월 판정 타당성).
- asset scope 실기 미검증(도입 시) KNOWN ISSUE + qa6 항목. R3#6 분리 시 3차 이월 기록.

---

## 5. Phase E 검토 결함 수정 반영 (2026-08-18)

> 검토 wf_1500d5f6-f8f(4렌즈 opus+xhigh, asset 프로토콜 보안 집중) — 발견 12(major 1·minor 11) →
> 적대적 검증 major **1건 기각**(m4v nosniff 재생 회귀 — nosniff 는 script/style 만 게이트, 미디어
> 요소는 Content-Type 무관하게 컨테이너 스니핑으로 디코드. 정확한 반증) → confirmed 0. minor 11
> 판정(수정 9·기각 2). 메인 2차: 실물 재검증 + verify·vite build.

- **[근본] asset_protocol ↔ serving.rs 중복 → `infra/range_file.rs` 공유 모듈**(2회 이상 룰 위반
  해소): RANGE_CHUNK_LIMIT·RANGE_RESPONSE_CSP·extension_mime·parse_range·read_slice 를 양쪽이 import
  (domain→infra 방향 정합). **한 곳 수정으로 아래 2건이 serving.rs(원격 도달)까지 자동 반영**.
- **[minor→실질] parse_range end<start 언더플로 패닉** — 뒤집힌 Range 헤더가 `end - start + 1` u64
  언더플로 → 릴리스 abort. **serving.rs 의 `/__taide/file` 은 원격 브라우저 도달 가능**이라 DoS 표면.
  `requested_end < start` 거부(공유 모듈, 양쪽 수혜) + 416 회귀 테스트.
- **[minor 수정 8]**: asset no-store 추가(닫힌 프로젝트 캐시 잔존 방어, serving 대칭)·m4v MIME
  (video/mp4, 프론트 분류기 정합)·**asset_protocol AppState 디커플링**(respond 가 `&HashMap<ProjectId,
  Project>` 파라미터 — infra 의 crate::state 직접 의존 제거·AppHandle 없이 단위 테스트 가능, 등록
  클로저가 lib.rs 에서 주입)·resolve_owning_project 이중 canonicalize 제거·http.rs 인라인 주석→doc·
  qa6 이미지 asset 항목 정정(이미지는 fileRawQueryOptions+blob, asset 은 video/audio 만)·CORS 생략
  문서화(UriSchemeContext 가 window_origin 미노출 — 헤더 재현 불확실, 문서화 채택).
- **[기각 2건 정당]**: asset 비-Range 전체 메모리 적재 — Tauri 내장 핸들러(vendored asset.rs)도 동일
  `read_to_end` 미러링이고 asset:// 는 로컬 webview(CSP script-src 'self') 전용이라 원격 스트리밍
  하드닝(R3#6, 인증 없는 네트워크 라우트)과 위협 모델 다름. 진짜 스트리밍은 async 프로토콜 전환
  필요·앱 실기 미검증이라 근거 없는 변경 위험.
- **KNOWN ISSUE 유지**: asset:// 실 webview 라운드트립(WKWebView video/audio Range 탐색·닫힌
  프로젝트 즉시 차단·CORS 생략 무해성)은 코드/단위 테스트로만 검증 — qa6 최우선 실기 항목.
