# Wave B 구현 계약 — 하드닝 마감 (2026-08-15)

> 정찰 wf_e628c338-e87(opus+high 2축: Remote 하드닝·Hot Exit 미세). 하중 주장 4건(gist 인바운드
> shell_override·게이트 필드 미필터, 저장 왕복 타이핑 소실, cross-file 편집 미러 구멍)은 메인이
> Rust·프론트 소스로 직접 재검증 완료(전부 확정). 캠페인 계약: 2026-08-14-remaining-features-pro-qa-plan.md.

## 1. 사용자 결정 (2026-08-15, 전부 추천안 채택)

| # | 결정 | 선택 |
|---|------|------|
| 범위 | Wave B 대상 | **신규 중대 4건 + 원래 잔여 10건(Remote 7·Hot Exit 3) 전부** |
| Remote | 보안 정책 패키지 | **추천 패키지** (아래 §3.1) |
| Hot Exit | 수정 방식 패키지 | **추천 패키지** (아래 §3.2) — 미러 부활 A안·락 해제 A안·timeoutMs A안 |

## 2. 확정 결함 (메인 재검증 완료 — 근거 실물)

### 신규 중대 4건 (정찰이 목록 밖에서 발견)

1. **[RCE급] gist 인바운드 shell_override 미필터** — `sync/service.rs:apply_payload_settings`(206-207)가
   `settings_service::apply_patch` 를 그대로 재사용, `apply_patch:173` 이 `patch.shell_override` 를 우선
   적용. 조작된 gist 의 shell_override → `terminal/commands.rs:267` → `pty.rs:150 CommandBuilder::new(shell)`
   로 사용자가 터미널 열 때 임의 실행파일 spawn. 업로드는 제외(`sync/service.rs:61`)하나 다운로드 필터 부재.
2. **gist 인바운드 게이트 2필드 미필터** — `apply_patch:228-229` 가 remote_access_enabled·
   remote_password_only_login 을 patch 값으로 덮어씀. 조작 gist 의 `remotePasswordOnlyLogin:true` 즉시
   반영(링크 없이 비밀번호만 로그인 전환) + 원격 세션이 sync_download 로 dispatch strip 가드(dispatch.rs:235-238) 우회.
3. **[데이터 손실] 저장 왕복 중 타이핑 소실** — `editor-pane.tsx` onSuccess(325-333)가 `draftRef.current
   !== finalContent` 비교 없이 무조건 setDirty(false)+pendingMirrorRef=false+mirror 타이머 clear. 저장
   invoke(320) 이후 친 글자가 FILE.CONTENT 재조회 → applyExternalContent 로 에디터에서 지워지고, 미러도
   dirty 도 없어 그 구간 종료 시 0-손실 계약도 깨짐. git push 등 전역 락 보유 시 창 확대.
4. **[데이터 손실·Wave A 회귀] cross-file 편집 hot-exit 미러 구멍** — `workspace-edit-applier.ts:182`
   이 미부착 모델 편집을 `markModelDirtyExternally`(표시만)로 처리, 미러 즉시 기록 없음. mirror-flush-registry
   는 마운트된 EditorPane 만 등록 → 백그라운드 모델의 코드액션/rename 편집은 미러도 tab.dirty 도 없이 종료 시 유실.

### 원래 잔여 — Remote 7 (정찰 실증, 근거 파일:라인은 스크래치 recon)

1. 개별 세션 TTL 만료의 라이브 WS 미단절(ws_upgrade_route 가 세션 미추출, epoch 만 단절 트리거) — 상한 무한.
2. Host 허용목록 부재(is_allowed_origin 은 자기정합성 검사 → DNS rebinding 통과, 서버 127.0.0.1 바인드).
3. 비밀번호 검증이 is_empty 하나뿐(1자·공백만·제어문자 통과), UI trim 과 로그인 폼 비trim 불일치.
4. 잠금 전역 카운터 DoS(익명 6회 실패로 정당 사용자 무기한 잠금 가능).
5. stale nonce 실패(5분 만료·중복·재시작)가 잠금 카운터 집계 + '비밀번호 오류' 오표시.
6. X-Forwarded-Proto 무검증 신뢰(insecure 고지 스푸핑 — 고지 소비만·인증 결정엔 미사용).
7. gist 인바운드 게이트 필드(신규 2와 동일 계열 — 통합 처리).

### 원래 잔여 — Hot Exit 3

1. 저장 직후 디바운스 발화 미러 부활 — 창은 [file_save invoke, onSuccess] 사이, 전역 락 FIFO 가 clear→write
   나쁜 순서를 고정. 재시작 시 디스크 동일 파일에 충돌 배너 오탐 + 캐시 오염.
2. 미러 쓰기 begin_mutation 락 — file_mirror_dirty 는 AppState 무변이·write_atomic(UUID temp+rename) 자체
   직렬화. 진짜 비용은 종료 플러시가 장기 락 보유자(git push 등) 뒤에 줄서 2.5초 타임아웃 → 0-손실 위반.
3. HotExitFlushRequested.timeoutMs 프론트 완전 미사용(핸들러가 payload 인자 미선언) — events.rs 계약과 코드 불일치.

## 3. 확정 설계

### 3.1 Remote 보안 패키지

- **Host 허용목록**: auth_middleware(server.rs:104) 진입부에서 Host 를 (hostname, port) 파싱 →
  hostname ∈ {127.0.0.1, localhost, ::1} ∪ Settings `remote_allowed_hosts: Vec<String>`(기본 []) 이고
  port == 현재 바인드 포트인지 검사, 불일치·부재 시 403(사유 로그). Origin 검사는 직교 유지(login_post
  중복 검사는 미들웨어로 통합 정리). remote_allowed_hosts 는 **sync 업로드 제외 + dispatch strip 대상**
  (원격 세션 자가 확장 차단). 기존 사용자(터널) 마이그레이션 안내 문구.
- **비밀번호**: remote_set_password 에서 trim 후 `chars().count() >= REMOTE_PASSWORD_MIN_LEN`(=8, types.rs
  신규 상수) 검사, trim 값 해시. 실패는 InvalidArgument + 신규 locale 키(4곳). UI 는 같은 최소 길이로 버튼
  사전 비활성. 로그인 측은 비trim 유지(저장값이 이미 trim). **기존 짧은 비밀번호는 그대로 두고 다음 변경 시에만 강제.**
- **잠금 완화**: IP 분리 **안 함**(루프백 수렴으로 무효). 카운터를 'nonce 쿠키 보유 여부'로 분리 —
  2요소(nonce 보유) 경로와 익명 password_only 경로의 실패 카운터를 나눠 익명 공격이 정당 기기를 잠그지
  못하게. login_post_route 순서 재배치(잠금 검사 전 nonce 추출). 잠금 상한 60초 유지. 하드락 유지(지연 전환
  안 함 — 동시 요청 상한 부재 시 지연이 더 약함).
- **stale nonce**: promote_nonce_to_session None 분기에서 record_login_failure 제거, 별도 상태
  (신규 locale remote.loginLinkExpired, 4곳) 렌더 — '새 링크 발급' 안내. 브루트포스 신호 아님(128bit nonce).
- **X-Forwarded-Proto**: Host 허용목록 연동 — Host 가 loopback 계열이면 헤더 무관 insecure 고지, 등록된
  터널 호스트일 때만 XFP 참조(콤마 분리 첫 토큰·trim·대소문자 무시). secure 판정 시에만 세션 쿠키 Secure 속성.
- **gist 인바운드 sanitize**: `strip_non_syncable(patch)` 단일 함수 신설 — remote_access_enabled·
  remote_password_only_login·**shell_override**·remote_allowed_hosts 를 None 강제. settings_to_sync_patch
  (업로드)와 apply_payload_settings(다운로드) 양쪽에서 재사용(목록 발산 방지). 손으로 만든 JSON 역직렬화
  실패 테스트 선작성. (ai_omlx_base_url 등 네트워크 필드는 이번 미포함 — 추천안 범위.)

### 3.2 Hot Exit 패키지

- **미러 부활(§HotExit-1) = A안**: `saveEpochRef` 추가, onSuccess·handleViewDisk 에서 ++.
  persistMirror(content, epoch) 로 변경 — 호출 직전 epoch 불일치면 즉시 반환, await mirrorDirty 이후
  불일치면 setQueryData 스킵 + clearMirror 로 되살아난 미러 되돌림 + MIRRORS 무효화. 타이머·flush 콜백이
  스케줄 시점 epoch 캡처. untitled 도 동일. **onError 에서는 epoch 올리지 않음**(저장 실패 후 편집분 미러
  보존). 저장 왕복 중에도 미러 계속 기록(0-손실 유지), 저장 성사 시에만 되돌림.
- **저장 타이핑 소실(신규 3) = 근본 수정**: onSuccess 에서 `draftRef.current !== finalContent` 이면 dirty
  유지 + mirror 타이머·pendingMirrorRef 미조작(저장 이후 친 글자 보존). §HotExit-1 부활 보정과 일관.
- **cross-file 미러 구멍(신규 4) = 근본 수정**: markModelDirtyExternally 시점에 해당 path 미러 즉시 1회
  기록(모델 값 + 알려진 디스크 mtime), 또는 플러시 시 externallyDirtyPaths 순회해 모델 값을 미러로 밀어넣기.
  (구현 시 두 방식 중 회귀 적은 쪽 선택 — 즉시 기록 우선.)
- **락 해제(§HotExit-2) = A안**: file_mirror_dirty·file_mirror_untitled **쓰기 2종만** begin_mutation 제거
  (lsp_send 선례 doc comment). clear/prune 계열은 락 유지. **§HotExit-1 A안 선행 후 적용**(락 떼면 순서
  비결정 — 부활 보정이 백엔드 순서 비의존이어야 안전). spawn_blocking 전환은 이번 제외(파일 도메인 일관성).
- **timeoutMs(§HotExit-3) = A안**: 핸들러가 payload 받아 `budget = max(0, timeoutMs -
  HOT_EXIT_FLUSH_SAFETY_MARGIN_MS)`(마진 상수 신설, ~300ms), `Promise.race([flushAllMirrors(), sleep(budget)])`
  후 반드시 flushMirrorsComplete 호출. 멈춘 패인 격리 + events.rs 계약 준수.
- **프론트 재접속 동반(Remote-1 연계)**: 세션 만료로 WS 끊길 때 remote-ws-client 가 401 재시도 루프에
  빠지지 않게 닫힘 사유 판별 후 로그인 페이지 이동(location 처리). Remote-1(개별 세션 WS 단절: ws_upgrade_route
  쿠키 추출 + handle_socket select! 에 sleep_until 데드라인 브랜치)과 세트로 구현.

### 3.3 실행 구조

- **Phase S 스파인(sonnet+xhigh, Rust 단독)**: Settings 신규 2필드(remote_allowed_hosts: Vec<String> 기본
  []·검토 결과 password_only 는 기존) + REMOTE_PASSWORD_MIN_LEN·HOT_EXIT_FLUSH_SAFETY_MARGIN_MS 상수 +
  strip_non_syncable 함수(+실패 테스트) + locale 신규 키(비밀번호 최소길이·loginLinkExpired·Host 관련, 4곳
  동기) + sync/service.rs 리터럴 + bindings 재생성 + cargo fmt/clippy/test.
- **Phase B1 Remote Rust(sonnet+xhigh 단독)**: server.rs(Host 허용목록·미들웨어 통합·XFP·Secure 쿠키)·
  commands.rs(비밀번호 검증·잠금 카운터 nonce 분리·순서 재배치·세션 만료 WS 단절 배선)·ws.rs(sleep_until
  데드라인·쿠키 추출)·service.rs·dispatch.rs strip 확장. 원격 차단·파리티 유지.
- **Phase B2 Hot Exit(sonnet+xhigh 단독)**: file/commands.rs(쓰기 2종 락 해제 + doc comment)·
  editor-pane.tsx(에포크 가드·저장 타이핑 보존·persistMirror 시그니처·flush budget)·untitled-pane·
  workspace-edit-applier·mirror-flush-registry(cross-file 미러)·hot-exit-flush-provider(timeoutMs)·
  remote-ws-client(재접속 사유 처리). B1·B2 는 파일 소유 분리(Rust file vs remote 도메인, 프론트 editor vs remote).
- **Phase C 검토**: 4렌즈(계약·정확성·**보안**(gist 필터·Host·잠금·세션 만료 타이밍·쿠키)·설계, opus+xhigh)
  → 적대적 검증(opus+high) → 수정(sonnet+xhigh) → 메인 2차(verify 전체+vite build+하중 재검증) → 커밋.

## 4. 기각·보류

| 안 | 처리 |
|----|------|
| ConnectInfo IP 분리 | 기각 — 127.0.0.1 바인드로 peer IP 루프백 수렴, 변별력 0 |
| 하드락 → 지연 전환 | 기각 — 동시 요청 상한 없이는 브루트포스 내성 약화 |
| argon2id KDF | 기각(직전 계약 유지) — 온라인 방어는 잠금 로직에 의존 전제 |
| 미러 락 해제 B안(읽기까지) | 기각 — 쓰기 2종만(A안). clear/prune 락 유지 |
| timeoutMs 필드 제거(B안) | 기각 — 소비(A안)로 계약 준수 |
| gist 필터에 ai_omlx_base_url 등 | 보류 — 추천안은 게이트2+shell_override+allowed_hosts만 |
| spawn_blocking 미러 IO | 보류 — 파일 도메인 전체 일관성, 별도 안건 |
| 슬라이딩 세션 TTL | 보류 — 고정 7일 유지 |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 커맨드 없음 — 필드/상수만).
- 4렌즈+적대적 검증+메인 2차 통과. 초점: gist 인바운드 필터 실효(역직렬화 테스트)·Host 허용목록
  터널 회귀·잠금 카운터 분리 정확성·미러 부활 에포크 가드·저장 타이핑 보존·cross-file 미러·락 해제 종료 플러시.
- 문서: data-model(신규 필드)·features/editor(미러 부활·cross-file)·research/remote-control(Host·잠금·gist
  sanitize)·acknowledge 계약·qa6-checklist Wave B 실기 항목. 하드닝 minor backlog 항목 종결 표기.

---

## 6. 검토 후속 — Host 허용목록 UI 완결 (2026-08-15 결정)

> Wave B 검토(wf_8e6238d4-bd2)가 L0-0 으로 지적: Host 허용목록 강제가 외부 호스트 보존 터널
> (cloudflared/ngrok — W6 "폰 터널 접속")을 403 으로 막는데 등록 GUI·링크 반영이 없다(fixer 가 죽은
> 키까지 제거). 사용자 결정(2026-08-15): **UI+링크까지 완결**. #2(잠금 축 선택)는 L2-0 수정으로 해소 확인.

- **포트 요구 완화**: `is_allowed_host` — loopback(127.0.0.1/localhost/::1)은 포트 == bind_port 유지
  (로컬 방어), **등록된 allowed_hosts 는 hostname 일치 시 포트 무관 통과**(443 종단·임의 포트 터널 지원).
- **링크 URL**: `remote_issue_link` — allowed_hosts 비어있지 않으면 첫 호스트로 `https://<host>/?t=...`,
  비어있으면 현행 `http://127.0.0.1:{port}` 유지.
- **locale 복원+확장**: remote.allowedHostsLabel·allowedHostsDescription 4곳 재추가 + 입력 placeholder·
  추가/삭제 액션 라벨(필요분).
- **편집 위젯**: features/settings/remote-section 에 allowed hosts 목록 편집(추가/삭제, settings_update
  로 저장 — 기존 remote 설정 저장 경로 재사용). remote_allowed_hosts 는 이미 sync 제외·dispatch strip
  대상(원격 세션 자가 확장 차단 유지).
- 검증: 4렌즈 축소(보안+정확성 opus+xhigh) → 수정 → 메인 2차. 포트 완화가 loopback 방어를 훼손하지
  않는지·등록 호스트 매칭 우회 없는지 집중.
