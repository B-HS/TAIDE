# 기능 — remote-control (프로젝트 원격 보기)

> Phase 7.5-F. 사용자 지적 5번. **기술 검토 완료** — 정본은 `docs/research/remote-control.md`.
> capability 추상화는 `architecture.md` §3.

## 1. 검토 결론 요약 (2026-08-06)

### 1.1 "잠자기 중 동작" 은 불가능하다

사용자 요구는 "잠자기나 잠금모드 상태에서도 작동"이었다. 조사 결과:

| 상태 | 가능 여부 | 근거 |
|------|----------|------|
| **화면 잠금(lock)** | **가능** | 잠금은 세션 종료가 아니다. 단 이어지는 아이들 슬립·App Nap 을 assertion 으로 막아야 한다 |
| 아이들 슬립 | 방지 가능 | `kIOPMAssertionTypePreventUserIdleSystemSleep` |
| **뚜껑 닫힘(clamshell)** | **불가** | IOPMLib.h 가 명시: 뚜껑 닫힘·명시적 잠자기·저전력은 못 막는다 |
| **명시적 잠자기** | **불가** | 〃. `kIOPMAssertionTypePreventSystemSleep` 은 10.9 deprecated·미지원 |
| 배터리 상태 상시 | **불가** | `caffeinate -s` 는 **AC 전원 전용** |

→ **배터리 + 뚜껑 닫힘 상태의 원격 접속은 OS 정책상 불가능하다.**
상시 원격이 필요하면 **전원 연결 + 외부 디스플레이(closed-display 모드)** 또는 데스크톱 Mac 이 전제다.
Wake on Demand 는 "슬립 중 동작"이 아니라 "깨어남"이고 sleep proxy·AC 조건이 붙어 일반 해법이 아니다.

**이 제약을 UI 에서 숨기지 않는다.** remote-control 을 켤 때 전원·뚜껑 조건을 명시하고,
sleep 방지 assertion 을 잡는다는 사실과 배터리 영향을 사용자에게 알린 뒤 동의를 받는다.

### 1.2 "화면 그대로" = 상태 미러링 (픽셀 스트리밍 아님)

3안을 비교한 결과 **(a) 상태 미러링** 채택.

- **(b) 픽셀 스트리밍은 잠금 상태에서 탈락한다** — ScreenCaptureKit 이 활성 GUI 세션을 요구한다.
  pre-login 캡처는 LaunchDaemon + LoginWindow 에이전트 + TCC 화면녹화 권한이 필요한 별도 제품 아키텍처다.
  Chrome Remote Desktop 커튼 모드조차 macOS Big Sur+ 지원을 중단했다.
- **(c) 헤드리스 렌더**는 (a) 대비 이득이 없다.
- **ADR-0004 가 결정적 이점**: Rust 가 이미 모든 도메인 상태를 소유하므로 웹은 "두 번째 프로젝션"일 뿐이고,
  FSD 상 교체 지점이 **`entities/*/*.ipc.ts` 한 층**이다.
  VS Code Server / JetBrains Gateway / Codespaces 가 전부 이 구조를 택했다.

### 1.3 "현재 화면 그대로" 의 정의

픽셀 동일이 아니라 **"view reload = 전체 UI 복원"이 성립하는 범위의 재현**으로 정의한다
(ADR-0004 가 이미 보장하는 성질).

→ 현재 IPC 계약에 **없어서 추가가 필요한 것: 에디터 커서·스크롤 위치.**
(`layout` 의 `view_state` 가 있지만 실시간 동기화 대상이 아니다.)

## 2. 지금 당장 해야 할 유일한 일 (구현 아님)

> 리서치의 핵심 권고. 구현은 나중이어도 **이것만은 지금 지켜야 한다.**

**IPC 계약을 설계·확장할 때 "전송 수단이 Tauri IPC" 라는 가정을 넣지 않는다.**

- `entities/*/*.ipc.ts` 는 이미 `invoke` 를 감싸는 얇은 층이다. 이 경계를 유지한다.
- 커맨드 시그니처에 Tauri 전용 타입(`Channel`, `InvokeResponseBody`)이 **불가피하게** 들어간 곳
  (터미널 raw 출력 — `terminal.md` §9)은 **예외로 문서화**하고, 원격에서는 다른 전송을 쓸 수 있게 남긴다.
- 소급 적용하면 entities 전 도메인을 다시 만져야 한다.

## 3. 아키텍처 (구현 시)

`architecture.md` §3 의 `RemoteControl` capability 로 부착한다.

- 프로젝트별 로컬 서버 + 웹 패널 서빙. 코어는 capability 등록 API 외에 어떤 전제도 갖지 않는다.
- 웹 클라이언트는 **같은 React 앱을 재사용**한다 — `entities` 층의 전송만 IPC → WebSocket/HTTP 로 교체.
- 세션 상태는 Rust 가 소유하므로 데스크톱과 웹이 **같은 상태를 본다**(별도 동기화 로직 불필요).

## 4. 보안 (구현 시 필수)

- **LAN 평문 HTTP 는 secure context 가 아니다**(localhost 만 예외) → 폰 브라우저에서 웹 API 가 막힌다.
  TLS(자체 서명 + 페어링) 또는 터널이 필요하다.
- **macOS 15 로컬 네트워크 권한**은 Bonjour 광고를 켜는 순간 걸린다.
- 토큰 기반 페어링 + 요청별 인증. 포트는 임의 고포트, 기본은 루프백 바인드.

## 5. 후속 조사 필요 (미확인 8건)

`docs/research/remote-control.md` 말미 참조 — clamshell 공식 문서, Apple silicon Power Nap 1차 출처,
`keepawake`/`nosleep` 크레이트가 실제로 잡는 assertion 타입, JetBrains Gateway 전송 방식 등.

## 6. 범위 판단

**Phase 7.5 에서는 구현하지 않는다.** §2 의 계약 원칙만 지키고, 실제 구현은
Phase 8 이후 또는 별도 기획으로 넘긴다 — 잠자기 제약이 사용자 기대와 다르므로
**범위를 사용자와 다시 합의한 뒤** 착수해야 한다.
