# 기능 확장 3차 구현 계약 — Hot Exit·Remote 비밀번호 (2026-08-14)

> 정찰 wf_d7cc8511-850(opus 2축) 결과를 메인이 검수하고 사용자 결정을 확정한 계약.
> 정찰 원문: 세션 스크래치패드 `recon3-{hotExit,remoteAuth}.md`. 핵심 사실(미러 복원 경로 부재·
> auth_middleware 구조)은 메인이 사전 grep 으로 독립 확인.

## 1. 사용자 결정 (2026-08-14, 전부 추천안 채택)

| # | 결정 | 선택 |
|---|------|------|
| o | Hot Exit | 충돌은 기존 ConflictBanner 재사용 / **untitled 포함**(탭 비휘발성화+탭ID 미러) / 디바운스 500ms+언마운트·블러 플러시+**CloseRequested 인터셉트 0-손실**(타임아웃 폴백) / 탭 닫기 시 미러 삭제+프로젝트 열 때 prune / baseline=디스크 mtime / 복원은 탭 활성화 시 lazy |
| p | Remote 게이트 | **링크+비밀번호 2요소** + "비밀번호만 허용" 토글(기본 off) / keyring+salt+sha256(신규 의존 0) / 세션 7일 만료 / 변경 시 전 세션 무효화 / 원격 클라이언트에서 변경 차단 / 로그인 페이지=중립 다크·JS 없는 폼·3언어(locale 테이블 재사용) / 실패 지수 백오프 잠금. **비밀번호 미설정 시 현행(링크만) 유지 — 하위호환** |
| p-스코프 | 프로젝트별 비밀번호 | **전역 1개만. 프로젝트 스코프 세션은 backlog**(커맨드 90+·이벤트 22종 필터링 재설계 — 게이트의 10배 규모·확정 계약 번복이라 별도 안건) |

## 2. 확정 사실 (정찰·메인 확인)

- **Hot Exit 원인**: mirror_dirty(1.5s 디바운스)는 앱데이터에 원자적 영속되나 **읽기 IPC 부재**
  (list_mirrors 는 pub 인데 커맨드 미등록). 종료 플러시 없음(마지막 ≤1.5s 손실), 언마운트 시
  타이머 취소만. 정리(clear)는 저장 성공·IDE diff 뿐 — GC 부재. `tab.dirty` 는 레이아웃에
  영속되어 "dirty 점은 켜졌는데 내용은 원본" 불일치가 이미 발생 중. untitled 는 탭 자체가
  휘발성(strip_volatile)이라 리로드만 해도 소실.
- **복원 소비 지점**: 활성 탭만 마운트(lazy 성립), applyExternalContent + setDirty + setTabDirty
  경로 존재, ConflictBanner(`dirty && file.content !== syncedContent`) 재사용 가능 —
  syncedContent 는 디스크 내용 유지가 계약.
- **Remote 인증 현행**: auth_middleware 전 라우트 일괄(예외 0) — 로그인 라우트는 미들웨어 예외
  필수. 1회용 링크 슬롯 1개·세션 sha256 digest 메모리 저장·**TTL 없음**·rate limit 없음.
  consume_link_token 이 검증+세션 발급 단일 함수 → "토큰 소모=폼 통과권(단기 nonce)"과
  "비밀번호 확인=세션 발급"으로 분리 필요. CSP 가 inline script 금지 → 로그인 폼은 JS 없는
  `<form method=post>`. `remote_access_enabled` 는 이미 gist 동기화 제외(화이트리스트) —
  그래도 비밀번호는 keyring(평문 파일·유출 사고 경로 차단).
- 미확인 1건(구현 중 검증): axum 0.8 현 feature set 에서 Form 추출 가능 여부 — 불가 시 바디
  직접 파싱(의존 추가 금지).

## 3. 확정 설계 (요지 — 상세는 정찰 보고서를 구현 에이전트가 정독)

### 3.1 Hot Exit (o)

- Rust(file 도메인): `MirrorFile` 에 `disk_modified_ms: Option<f64>` baseline(serde default),
  `MirrorEntry` 에 Serialize+Type + 디스크 현재 mtime 비교로 `conflict: bool` 계산(러스트 판정).
  신규 커맨드 `file_list_mirrors(projectId)` / `file_clear_mirror(projectId, path)` /
  `file_prune_mirrors(projectId, keepPaths)` + **untitled 축**: `file_mirror_untitled(projectId,
  tabId, content)` / untitled 미러 list·clear (경로 해시 대신 탭ID 키, ensure_within_root 비대상).
  layout `is_volatile` 에서 Untitled 제외(비휘발성화 — 레이아웃 복원 대상화).
- lib.rs: 커맨드 등록 + **CloseRequested 인터셉트**: prevent_close → 프론트에 flush 요청 이벤트
  emit → 프론트가 dirty 모델 전량 미러 플러시 후 완료 커맨드(신규 1종) 호출 → 종료 재개.
  **타임아웃(수 초) 폴백으로 강제 종료** — 하드행 금지.
- 프론트: 디바운스 1500→500ms, 언마운트·window blur 시 즉시 플러시. 프로젝트 활성화 시
  list_mirrors 1회 → path→entry 맵. 탭 활성화 시 미러 있으면 applyExternalContent + dirty 세팅
  (syncedContent 는 디스크 내용 유지), conflict=true 면 ConflictBanner(복원 문구 variant).
  "디스크 보기" 선택 시 clear_mirror. 탭 닫기 시 clear, 프로젝트 열기 시 prune(열린 탭 keepPaths).
  untitled-pane 도 미러 배선 + 복원.
- 저장 성공 경로는 Rust 가 이미 clear — 미러 쿼리 무효화만 추가.

### 3.2 Remote 비밀번호 (p)

- 저장: `SecretAccount::RemoteAccess`("remote-access") — `salt$sha256(salt+password)` 문자열 1개.
  검증은 constant_time_eq. `RemoteStatus.password_configured: bool` + `password_only_login: bool`
  노출(설정 토글은 Settings 신규 필드 — gist 화이트리스트 비포함 확인).
- 미들웨어 재배치: Origin 검사 → 유효 세션 쿠키 통과 → `/__taide/login` GET·POST 예외 →
  링크 토큰: 비밀번호 미설정=즉시 세션(현행) / 설정=토큰 소모+단기 nonce 쿠키 발급 후 로그인
  폼 302 → 그 외: password_only_login=on 이면 로그인 폼 302, off 면 401.
- 로그인 POST: nonce(2요소 모드) 또는 password_only 확인 + Origin 필수 강제 + 비밀번호 검증
  (실패 지수 백오프 잠금: `REMOTE_LOGIN_MAX_ATTEMPTS`·`REMOTE_LOGIN_LOCKOUT_MS` 상수) → 세션
  쿠키 발급. 세션 저장을 `HashMap<digest, 만료 Instant>`(7일)로 교체, has_active_session 에서
  만료 스윕. 비밀번호 설정/변경/해제 시 revoke_all_sessions.
- 신규 IPC: `remote_set_password` / `remote_clear_password` — **dispatch 원격 차단**(허용목록에
  넣되 원격 요청 거부 arm 또는 미등록 정책은 파리티 테스트와 정합하게 — 기존 파리티는 bindings
  전 커맨드 일치 강제이므로 arm 에서 명시 거부 응답으로 구현).
- 로그인 페이지: `login_page.rs` — self-contained HTML(인라인 style, JS 0), 중립 다크 +
  prefers-color-scheme, settings.language 로 locale 테이블(remote ns) 문자열 선택, http 접속 시
  비암호화 고지, 잠금 중 남은 시간 표시.
- UI: `features/settings/remote-password-row.tsx`(ai-provider-token-row 패턴), 경고문 아래·링크
  버튼 위 배치, 미설정 시에만 "링크를 아는 누구나 접속" 경고, 변경 시 세션 무효화 토스트.

### 3.3 실행 구조

Phase A 스파인(sonnet+high): locale 4곳 — Hot Exit 2키(editor.mirrorRestored·
editor.mirrorRestoredConflict) + Remote ~15키(remote ns: passwordLabel/Placeholder/Set/Clear/
Configured/NotConfigured/Hint/OnlyToggle/OnlyToggleDescription/SessionsRevokedToast +
loginTitle/loginPasswordLabel/loginSubmit/loginFailed/loginLocked({{seconds}})/
loginInsecureNotice) + `SecretAccount::RemoteAccess` + Settings `remote_password_only_login: bool`
(기본 false, sync 화이트리스트 비포함) 배관 + bindings 재생성. 컴파일 그린 종료.
Phase B(단독): B1 Hot Exit Rust(file 도메인+layout is_volatile+lib.rs CloseRequested·등록+
dispatch+bindings — lib.rs 대변경이라 단독).
Phase C(병렬): C1 Remote Rust(remote 도메인 전체+lib.rs 등록 2종+dispatch+bindings) ∥
C2 Hot Exit 프론트(editor-pane·entities/file·untitled·flush provider).
Phase D(단독): D1 Remote 프론트(remote-password-row·section·ipc/query).
Phase E 검토: 렌즈 3종(계약·정확성·**보안**(비밀번호 경로·타이밍·rate limit·nonce·세션 만료·
gist 비유출·로그 비노출), opus+high) → 적대적 검증(opus+medium) → 수정(sonnet+high).
Phase F 메인 2차: verify 전체+vite build+커밋.

## 4. 기각된 대안

| 기각안 | 사유 |
|--------|------|
| 프로젝트별 암호(전체 접근) | 어느 암호로도 전체가 보여 격리 착각만 유발 — 사용자 기각 |
| 프로젝트 스코프 세션(이번) | 게이트의 10배 규모·"상태 완전 공유" 확정 계약 번복 — backlog |
| 비밀번호를 settings.json 에 | 평문 파일 노출 + 화이트리스트 누락 1줄이면 gist 유출 |
| argon2id | 신규 의존 승인 필요 — sha256+salt 로 시작, 저장 형식 동일해 승격 용이 |
| SPA 내 로그인 라우트 | 미인증에 전체 번들·WS 노출 + 재연결 루프 배경 가동 — 독립 HTML 채택 |
| Rust 단독 종료 플러시 | 미저장 내용은 웹뷰 monaco 모델에만 존재 — 프론트 왕복 필수 |
| 미러 내용 해시 baseline | 대용량 비용 — mtime 으로 시작 |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. 파리티(dispatch 신규 커맨드 반영)·locale 4곳·en⊆required.
- 문서: data-model(미러 스키마)·ipc-contract(신규 커맨드·인증 흐름)·features/editor·
  research/remote-control 관련 절 갱신. qa6-checklist 실기 항목 추가.
- 실기(사용자): 입력→강제종료→재시작 복원(파일·untitled), 충돌 배너, 비밀번호 설정→링크 접속
  →로그인 폼→세션, 잠금 백오프, 비밀번호만 토글, 변경 시 세션 무효화.
