# remote-control 모드 기술 검토 — macOS 잠자기·잠금 제약과 "웹에서 현재 화면"

> 조사 기준일: 2026-08-06. 방법: Apple 공식 문서(developer.apple.com, support.apple.com), Apple 오픈소스
> 헤더(`IOPMLib.h`), Apple Developer Forums DTS 답변, macOS man page, MDN, VS Code / JetBrains / Google 공식 문서,
> crates.io API. 검색 결과 요약만 있고 원문을 열지 못한 항목은 본문에 `미확인` 으로 표기했다.
>
> 전제 문서: `docs/architecture.md` §3(프로젝트 capability 추상화 — remote-control 은 `RemoteControl` capability),
> `docs/adr/0004-rust-owned-state.md`(모든 도메인 상태는 Rust 소유, view 는 프로젝션).

---

## 0. 결론 먼저

| 사용자 요구 | 판정 |
|---|---|
| 화면 **잠금(lock)** 상태에서 remote-control 동작 | **가능.** 잠금은 세션을 종료하지 않는다. 단 잠금 후 이어지는 아이들 슬립을 power assertion 으로 막아야 하고, 화면 **캡처**는 불가(→ 아래 표 3행). |
| **잠자기(sleep)** 상태에서 remote-control 동작 | **불가능.** 시스템 슬립 중에는 사용자 프로세스가 실행되지 않는다. "슬립 중에도 도는 서버"는 OS 정책상 만들 수 없다. 유일한 길은 **슬립을 막는 것**(대가 있음) 또는 **네트워크로 깨우는 것**(제약 있음). |
| 잠금/잠자기 상태에서 **화면 픽셀을 그대로 전송** | **잠금 상태에서 사실상 불가능.** 활성 GUI 세션이 없으면 ScreenCaptureKit 이 프레임을 주지 않는다. 로그인윈도우 대응은 데몬+에이전트 + TCC 권한이 필요한 별개 제품 아키텍처다. |
| "각 프로젝트를 웹에서 현재 화면 그대로" | **상태 미러링 방식으로 가능.** 단 정의를 "픽셀 동일"이 아니라 **"동일 상태의 재현"** 으로 확정해야 한다. ADR-0004 덕분에 TAIDE 는 이 방식에 구조적으로 유리하다. |

권장안: **(a) 상태 미러링 + 전원 연결 시 idle-sleep assertion + 사용자 동의 UI.** 근거와 대가는 §6.

---

## 1. macOS 슬립 중 프로세스 동작

### 1.1 슬립은 "느려짐"이 아니라 "정지"다

시스템 슬립(S3 / Apple silicon 의 저전력 슬립) 중에는 CPU 가 정지하고 사용자 프로세스는 스케줄되지 않는다.
따라서 슬립 중 TCP 리스너가 응답하는 시나리오는 성립하지 않는다. macOS 가 제공하는 예외는 두 가지뿐이다.

- **Power Nap** — 슬립 중 메일 확인, Time Machine, 소프트웨어 업데이트 등 **시스템이 지정한 유지보수 작업**만
  수행한다. 임의의 서드파티 서버를 깨워 두는 수단이 아니다.
  [Turn Power Nap on or off on Mac](https://support.apple.com/guide/mac-help/turn-power-nap-on-or-off-mh40774/mac)
  - Apple silicon Mac 에는 Power Nap 설정 자체가 없다(항시 구동 프로세서 구조로 흡수). — 검색 결과 다수가 일치하나
    Apple 지원 문서 원문 확인은 실패, **부분 미확인**.
- **Wake for network access (Wake on Demand / Bonjour Sleep Proxy)** — 슬립 중 다른 기기가 공유 서비스에 접근하면
  Mac 을 **깨운다**. "슬립 중 동작"이 아니라 "슬립에서 깨어남"이다.
  [Set sleep and wake settings on Mac](https://support.apple.com/guide/mac-help/mchle41a6ccd/mac)
  - 동작 조건: 같은 로컬 네트워크에 Bonjour Sleep Proxy 가 존재해야 하고(AirPort/Time Capsule/Apple TV 계열),
    노트북은 **전원 어댑터 연결 + 디스플레이 열림 또는 외부 디스플레이 연결** 상태에서만 Wake on Demand 를 수행한다.
    [Understanding Sleep Proxy Service (Stuart Cheshire, Apple)](https://stuartcheshire.org/sleepproxy/)
  - 현대적 홈 환경(일반 공유기 + Apple TV 없음)에서 sleep proxy 가 존재하는지, 인터넷 경유 원격에서 깨울 수 있는지는
    환경 의존이며 **일반 해법이 아니다**. TAIDE 가 이걸 전제로 기능을 설계할 수 없다.

### 1.2 슬립을 막는 방법과 그 한계 (핵심)

Apple 의 power assertion API 는 `IOPMAssertionCreateWithName` 이며, 실질적으로 쓸 수 있는 타입은 하나다.

| assertion type | 효과 | 상태 |
|---|---|---|
| `kIOPMAssertionTypePreventUserIdleSystemSleep` | 사용자 무활동으로 인한 **아이들 시스템 슬립**만 방지 | 현행 권장 |
| `kIOPMAssertionTypePreventUserIdleDisplaySleep` | 디스플레이 아이들 슬립 방지 | 현행 |
| `kIOPMAssertionTypePreventSystemSleep` | 모든 슬립 방지 + Dark Wake 상주 | **deprecated(10.9), OS X 릴리즈에서 미지원** |
| `kIOPMAssertionTypeNoIdleSleep` | 아이들 슬립 방지(구형) | deprecated(10.7) |

출처: [kIOPMAssertionTypePreventUserIdleSystemSleep](https://developer.apple.com/documentation/iokit/kiopmassertiontypepreventuseridlesystemsleep),
[kIOPMAssertionTypePreventSystemSleep](https://developer.apple.com/documentation/iokit/kiopmassertiontypepreventsystemsleep?language=objc),
[IOPMLib.h 원문](https://github.com/opensource-apple/IOKitUser/blob/master/pwr_mgt.subproj/IOPMLib.h)

`IOPMLib.h` 헤더 주석이 한계를 명시한다: `PreventUserIdleSystemSleep` 은 "사용자 활동 부재로 인한 자동 슬립"만 막고,
**뚜껑 닫힘(lid close), Apple 메뉴의 명시적 잠자기, 저전력(배터리 부족) 등 다른 이유로는 시스템이 여전히 슬립한다.**

CLI 로 같은 동작을 하는 것이 `caffeinate(8)` 이다.
[caffeinate man page](https://keith.github.io/xcode-man-pages/caffeinate.8.html)

| 플래그 | 의미 |
|---|---|
| `-i` | 아이들 시스템 슬립 방지 (플래그 없을 때 기본) |
| `-d` | 디스플레이 슬립 방지 |
| `-m` | 디스크 아이들 슬립 방지 |
| `-s` | 시스템 슬립 방지 — **AC 전원일 때만 유효** |
| `-u` | 사용자 활동 선언(디스플레이 켬), 기본 5초 |
| `-t` | 타임아웃(초) |
| `-w pid` | 해당 프로세스 종료까지 유지 |

정리하면:

- **배터리 구동 + 뚜껑 닫힘 → 슬립을 막을 수단이 사실상 없다.** (`-s` 는 AC 전용, assertion 은 lid close 를 못 막음)
- 전원 연결 + 뚜껑 열림/외부 디스플레이 → assertion 으로 계속 깨어 있게 유지 가능. 이때 대가는 **배터리·전력 소모와
  "닫으면 자는" 사용자 기대의 위반**이다. IDE 가 사용자 모르게 이걸 켜면 안 된다.
- **뚜껑 닫힘(clamshell)** 상태 상시 구동은 "외부 디스플레이 + 외부 입력장치 + 전원 어댑터" 라는 Apple 의 closed-display
  모드 조건에 해당한다. Apple 지원 문서 원문(closed-display mode) URL 확인은 실패 — **미확인**. 2차 자료들은 전원 어댑터
  연결을 권장 조건으로 일관되게 기술한다([Macworld](https://www.macworld.com/article/673295/how-to-use-macbook-with-lid-closed-stop-closed-mac-sleeping.html)).
  즉 "노트북 덮고 가방에 넣은 채 원격 접속"은 **불가능**으로 봐야 한다.

### 1.3 App Nap

백그라운드 앱은 CPU 우선순위 강등·타이머 스로틀·I/O 스로틀을 받는다. App Nap 대상이 되는 조건은
**전부 만족**할 때다: 포그라운드가 아님, 보이는 창 내용을 최근에 갱신하지 않음, 소리를 내지 않음,
**IOKit power management 또는 NSProcessInfo assertion 을 하나도 잡지 않음**, OpenGL 미사용.
[Energy Efficiency Guide for Mac Apps — Extend App Nap](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/AppNap.html)

→ **회피법은 assertion 을 잡는 것**이다. remote-control 세션이 활성인 동안
`NSProcessInfo.beginActivityWithOptions(.userInitiated…)` 또는 IOKit assertion 을 유지하면 App Nap 대상에서 빠진다.
[Schedule Background Activity](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/SchedulingBackgroundActivity.html)

권장 조합: **원격 세션 연결 중에만** `NSActivityUserInitiatedAllowingIdleSystemSleep`(App Nap 만 회피, 슬립은 허용)
또는 사용자가 명시적으로 "원격 접속 유지"를 켰을 때만 `PreventUserIdleSystemSleep`(슬립까지 방지)으로 승격.

### 1.4 화면 잠금(lock)은 슬립과 다르다

잠금은 loginwindow 가 세션 UI 를 가리는 것이고, **로그인 세션과 그 프로세스는 계속 존재한다.**
Apple DTS 는 화면 공유 제품 아키텍처를 설명하며 에이전트를 `Aqua` 세션과 `LoginWindow` 세션에 각각 로드하는 방식을
제시한다 — 즉 잠금/로그인윈도우는 **세션 종류의 문제이지 프로세스 종료가 아니다.**
[Apple Developer Forums #814152 (DTS 답변)](https://developer.apple.com/forums/thread/814152)

- "잠그면 백그라운드 앱이 멈춘다"는 취지의 커뮤니티 글이 다수 있으나, **잠금 자체가 프로세스를 정지시킨다는 1차 출처는
  확인하지 못했다 — 미확인.** 실무적으로 관측되는 정지는 (i) 잠금 이후 이어지는 디스플레이 슬립 → 아이들 시스템 슬립,
  (ii) App Nap 스로틀 두 가지로 설명된다. 둘 다 §1.2·§1.3 의 assertion 으로 다룰 수 있다.
- 따라서 검증 항목(QA)으로 남긴다: **잠금 상태에서 assertion 을 잡은 TAIDE 프로세스의 리스너가 계속 응답하는지 실측.**

---

## 2. "현재 화면 그대로 웹에서" 구현 방식 비교

핵심 판정 기준은 **잠금/잠자기 상태에서 실제로 동작하는가** 다.

### (a) 상태 미러링 — Rust 상태를 웹 클라이언트에 투영

Rust 코어가 이미 보유한 도메인 상태(프로젝트, 레이아웃, 탭, 파일 dirty, 터미널 스크롤백, git 상태)를
로컬 서버로 노출하고, 웹 클라이언트가 **같은 React 코드베이스**로 렌더한다.

- 잠금 상태: **동작.** GUI·윈도우 서버·화면 캡처가 전혀 필요 없다. 순수 데이터 + 네트워크.
- 잠자기 상태: 동작 불가(§1). 그러나 이건 모든 방식의 공통 제약이다.
- 대역폭: 텍스트/JSON 수준. 모바일 회선에서도 실용적.
- 비용: **웹 전용 전송 계층(WebSocket) + 인증**을 새로 만들어야 한다. Tauri IPC 는 웹뷰 전용이라 그대로 못 쓴다.
  → 완화책: entities 계층의 `*.ipc.ts` 어댑터만 교체하고 widgets/features 는 그대로 재사용(§6).
- 선례: 업계 표준. VS Code Remote 는 UI 를 로컬에, 서버를 원격에 두고 명령/확장 실행만 오간다
  ([VS Code Remote Development](https://code.visualstudio.com/docs/remote/remote-overview),
  [The Visual Studio Code Server](https://code.visualstudio.com/blogs/2022/07/07/vscode-server)).
  JetBrains Remote Development 도 원격 호스트에서 백엔드 IDE 를 돌리고 로컬은 씬 클라이언트다
  ([JetBrains Remote Development](https://www.jetbrains.com/help/idea/remote-development-overview.html)).
  다만 **JetBrains 이 픽셀을 보내는지 UI 모델을 보내는지는 공식 문서에 서술이 없다 — 미확인.**

### (b) 픽셀 스트리밍 — 화면 캡처 + WebRTC

- 잠금 상태: **사실상 불가.** ScreenCaptureKit 은 활성 GUI 세션을 요구한다. 로그인윈도우/세션 없는 컨텍스트에서
  데몬이 캡처를 시도하면 "graphical context 없음"으로 초기화 실패하거나 NULL/빈 프레임을 받는다.
  Apple DTS 의 정답 아키텍처는 **데몬(네트워크) + 에이전트(GUI)** 분리이고, 에이전트를
  `LimitLoadToSession = [Aqua, LoginWindow]` 로 로드해야 하며, **pre-login ScreenCaptureKit 은 macOS 14.4 에서야
  버그가 수정**됐다. [Apple Developer Forums #814152](https://developer.apple.com/forums/thread/814152)
  즉 이건 IDE 기능이 아니라 **원격 데스크톱 제품 한 개를 더 만드는 일**이다(LaunchDaemon 설치, XPC, TCC 화면 녹화 권한).
- 잠자기 상태: 불가.
- 참고로 Chrome Remote Desktop 의 커튼 모드조차 **macOS Big Sur 이상에서 지원 중단**됐다.
  [Control use of Chrome Remote Desktop (Google)](https://support.google.com/chrome/a/answer/2799701?hl=en)
- 대역폭·지연: 코드 편집(고DPI 텍스트)에 비디오 코덱은 비효율적이고, 텍스트 가독성이 손상된다.
- 결론: **TAIDE 범위에서 탈락.**

### (c) 원격 헤드리스 렌더 — 별도 웹뷰를 헤드리스로 띄워 렌더 결과 전송

- 두 번째 웹뷰 인스턴스를 백그라운드로 띄우고 그 화면을 캡처/전송하는 방식.
- 잠금 상태: 캡처를 하는 순간 (b)와 같은 벽에 부딪힌다. 오프스크린 렌더링으로 우회한다 해도
  **Tauri/WKWebView 에서 오프스크린 프레임 추출 지원 여부는 확인하지 못했다 — 미확인.**
- 상태 동기화 문제는 (a)와 동일하게 남는데, 렌더 파이프라인만 하나 더 늘어난다.
- 결론: (a) 대비 이득 없음. **탈락.**

### 비교 요약

| 기준 | (a) 상태 미러링 | (b) 픽셀 스트리밍 | (c) 헤드리스 렌더 |
|---|---|---|---|
| 잠금 상태 동작 | O | X (세션·TCC·데몬 필요) | X~미확인 |
| 잠자기 상태 동작 | X (OS 제약) | X | X |
| 대역폭 | 낮음 | 높음 | 높음 |
| 텍스트 품질 | 네이티브 | 열화 | 열화 |
| 구현 범위 | 전송계층 + 인증 | 원격데스크톱 제품 1개 | (b) + 렌더 스택 |
| ADR-0004 와의 적합성 | 매우 높음 | 무관 | 낮음 |
| 업계 선례 | VS Code / JetBrains / Codespaces | 게임 스트리밍 | 없음 |

### 선례가 상태 미러링을 택한 이유

VS Code 계열이 "화면 그대로"가 아니라 서버-클라이언트를 택한 이유는 문서에서 드러난다: VS Code 는 원래
**UI 프로세스와 백엔드(확장 호스트·터미널·디버거) 프로세스가 분리된 다중 프로세스 구조**이고, 원격화는 그 경계를
네트워크로 늘린 것뿐이다. 소스가 로컬에 없어도 되고, 클라이언트는 네이티브 렌더 품질을 유지한다.
([VS Code Remote Development](https://code.visualstudio.com/docs/remote/remote-overview))
**TAIDE 는 ADR-0004 로 이 경계를 이미 갖고 있다.** 그 경계가 Tauri IPC 냐 WebSocket 이냐의 차이만 남는다.

---

## 3. "현재 화면 그대로"의 정의를 확정해야 한다

(a)를 택하면 웹은 픽셀 복제가 아니라 **동일 상태의 재현**이 된다. 재현 가능성은 상태 소유권에 달려 있고,
ADR-0004 가 이미 답을 정해 뒀다.

| 화면 요소 | Rust 가 소유? | 웹 재현 |
|---|---|---|
| 프로젝트 목록·탭·스플릿·포커스 | O (`domain/layout`) | 그대로 |
| 파일 트리·git 상태·검색 결과 | O | 그대로 |
| 에디터 열린 파일·커서/스크롤 | 일부 — dirty 내용은 debounce 미러(ADR-0004 예외) | 미저장 변경까지 재현. **커서/스크롤 위치는 현재 계약에 없음 → 추가 필요** |
| 터미널 화면 | O (스크롤백 ring buffer, replay) | 그대로 |
| Monaco 데코레이션(gutter/blame/진단) | 원천 데이터는 Rust | 같은 코드로 재계산 |
| 호버·드래그 중 표시 등 순수 UI 상태 | X (view 로컬) | **재현 안 됨 — 사양으로 명시** |

즉 "view reload = 전체 UI 복원"(ADR-0004 §4)이 성립하는 범위가 곧 웹 미러의 범위다.
**remote-control 은 ADR-0004 를 재사용하는 기능이지, 새 요구를 만드는 기능이 아니다.** 이 점이 (a) 채택의 최대 근거다.

미해결로 남는 결정 지점(사용자 확인 필요):

1. 웹에서 **편집·터미널 입력까지 가능**한가, 아니면 **읽기 전용 관측**인가. (보안·경합 난이도가 크게 다르다)
2. 데스크톱과 웹이 **같은 세션을 공유**(양쪽 커서가 서로 보임)하는가, **독립 뷰**인가.
3. 웹 접속을 **동일 LAN 한정**으로 할 것인가, 인터넷 경유까지 허용할 것인가.

---

## 4. 보안

### 4.1 노출 범위와 인증

- 기본은 **`127.0.0.1` 바인딩**. LAN 노출은 프로젝트별 opt-in(capability 부착 시 명시 선택).
- 페어링: 일반적 패턴은 **단기 페어링 코드 → 장기 디바이스 토큰 교환**. 토큰은 프로젝트 단위로 스코프하고
  발급 목록·개별 폐기를 UI 에 노출한다. 토큰은 Rust 가 소유·영속화(ADR-0009), view/localStorage 에 두지 않는다.
- 원격 제어는 **파일시스템·터미널 전권**과 같다. 인증 실패 = 임의 코드 실행이다. 터미널/편집을 허용한다면
  토큰 유출은 곧 머신 침해이므로, 최소한 프로젝트 루트 밖 접근 차단과 감사 로그가 필요하다.

### 4.2 TLS 와 secure context (놓치기 쉬운 함정)

브라우저는 `http://localhost` / `http://127.0.0.1` 만 "potentially trustworthy" 로 취급한다.
**LAN IP(`http://192.168.x.x`)나 `.local` 호스트명은 secure context 가 아니다.**
[MDN — Secure Contexts](https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts)

→ 다른 기기 브라우저에서 열면 Service Worker, `crypto.subtle`, 클립보드 API 등 상당수가 막힌다.
선택지는 셋이다.

1. **터널 경유 HTTPS** (Tailscale / Cloudflare Tunnel 등) — 유효한 인증서를 외부가 제공. 추가 의존성.
2. **로컬 CA + 자체 서명 인증서**(mkcert 류) — 클라이언트 기기마다 신뢰 설치 필요. UX 나쁨.
3. **평문 HTTP + 기능 축소** — secure context 요구 API 를 안 쓰는 설계로 한정.

권장은 1) 을 "외부 접속" 옵션으로, 3) 을 "동일 LAN 기본"으로 두는 이중 구성. 결정 필요 사항이다.

### 4.3 macOS 방화벽 / 로컬 네트워크 권한

- macOS 응용프로그램 방화벽은 **앱 단위로 수신 연결을 통제**한다. 목록에 없는 앱이 수신 연결을 받으면 허용/차단
  경고가 뜨고, 신뢰된 인증서를 가진 앱은 별도 개입 없이 허용될 수 있다.
  [Block connections to your Mac with a firewall](https://support.apple.com/guide/mac-help/block-connections-to-your-mac-with-a-firewall-mh34041/mac)
  → **코드 서명·공증된 릴리즈 빌드에서는 프롬프트 경험이 달라진다.** 개발 빌드에서만 뜨는 프롬프트를 보고 판단하지 말 것.
- macOS 15(Sequoia)부터 앱의 **로컬 네트워크 접근에 사용자 동의**가 필요하고, `NSLocalNetworkUsageDescription` 을
  Info.plist 에 넣어야 한다. 이는 주로 **앱이 로컬 네트워크로 나가는 연결**에 적용되며, 데몬/루트/터미널 도구는 대상이 아니다.
  Apple 1차 문서 원문 확인은 실패 — **부분 미확인**. 정리 자료:
  [Local Network access on macOS 15 Sequoia (Rogue Research)](https://www.rogue-research.com/2025/05/local-network-access-on-macos-15-sequoia/),
  [Apple Developer Forums #764374](https://developer.apple.com/forums/thread/764374)
  → TAIDE 가 리스너만 여는 경우 해당 없을 가능성이 높지만, mDNS/Bonjour 로 자신을 광고하는 순간 대상이 된다.
  **Bonjour 광고 채택 여부는 이 권한 프롬프트를 감수할지와 함께 결정해야 한다.**

---

## 5. Rust/Tauri 구현 수단

- power assertion: Apple 공식 API 는 `IOPMAssertionCreateWithName` (IOKit). Rust 에서 직접 호출하려면
  [`objc2-io-kit` 0.3.2](https://crates.io/crates/objc2-io-kit) 또는 [`io-kit-sys` 0.5.0](https://crates.io/crates/io-kit-sys) 로 FFI.
- 상위 래퍼 크레이트: [`keepawake` 0.6.0](https://crates.io/crates/keepawake) (2025-10 갱신, 크로스플랫폼),
  [`nosleep` 0.2.1](https://crates.io/crates/nosleep) (2022 갱신 — 유지보수 정체).
  **두 크레이트가 어떤 assertion 타입을 잡는지는 소스 확인 전까지 미확인.** 어떤 타입을 잡느냐가 §1.2 의
  동작 차이를 그대로 만들므로, 채택 전 반드시 확인하거나 IOKit 직접 호출로 통제한다.
- App Nap 회피용 `NSProcessInfo.beginActivityWithOptions` 는 Foundation API 이며 `objc2-foundation` 으로 호출 가능
  (구체 바인딩 시그니처 **미확인**).
- `fruitbasket` 0.10.0 은 2022 년 이후 갱신이 없다 — 신규 채택 비권장.
- 최후 수단으로 `caffeinate` 프로세스를 자식으로 띄우는 방법도 있으나, 외부 프로세스 수명 관리가 추가되고
  assertion 소유자가 TAIDE 가 아니게 된다. **비권장.**

---

## 6. TAIDE 적용 가이드

### 권장안 — (a) 상태 미러링 + 조건부 sleep 방지

`RemoteControl` capability 를 프로젝트에 부착하면(architecture.md §3):

1. 프로젝트 스코프의 **로컬 HTTP/WebSocket 서버**를 연다(기본 `127.0.0.1`, 포트는 Rust 가 할당·상태 보관).
2. 웹 클라이언트는 **같은 React 코드베이스**를 쓴다. FSD 상 교체 지점은 `entities/{domain}/{domain}.ipc.ts` 한 층뿐이다
   — `invoke()`/`listen()` 을 WebSocket RPC/구독으로 바꾸는 어댑터. widgets·features 는 무변경.
   → 이를 가능하게 하려면 **ipc-contract 의 command/event 계약을 전송 수단과 분리**해 두어야 한다.
   ADR-0011 의 Rust→TS 타입 생성 산출물을 두 전송에 공유한다.
3. 서버는 Tauri IPC 와 **같은 도메인 service 를 호출**한다. command 핸들러가 얇다는 architecture.md §2 규칙이
   여기서 값을 한다 — 전송만 다른 두 번째 진입점이 된다.
4. 이벤트는 Rust 의 내부 broadcast 를 두 구독자(웹뷰, 웹 세션들)에게 팬아웃한다.

전력 정책:

| 상황 | 정책 |
|---|---|
| 원격 세션 연결 없음 | assertion 없음. 평소대로 잠자기 허용 |
| 원격 세션 연결 중 | App Nap 회피 activity 만 유지(슬립은 허용) |
| 사용자가 "원격 접속 대기 유지"를 명시적으로 켬 | `PreventUserIdleSystemSleep` assertion. **전원 연결 시에만** 자동 활성, 배터리에서는 경고 + 기본 비활성 |

UI 의무: assertion 이 활성인 동안 **잠자기를 막고 있다는 사실과 그 이유를 상시 표시**하고 원클릭 해제를 제공한다.
사용자 몰래 Mac 을 깨워 두는 것은 배터리 소모와 신뢰 문제로 직결된다.

### 명확히 "불가능/비현실적"인 것

1. **잠자기 상태에서의 동작은 불가능하다.** OS 가 프로세스를 재우기 때문이며 우회는 없다. 제품 문구에
   "잠자기 중에도 접속"이라고 쓸 수 없다. 쓸 수 있는 문구는 "잠자기를 막아 접속 상태를 유지"다.
2. **배터리 + 뚜껑 닫힘 상태의 원격 접속은 불가능하다.** assertion 이 lid close 를 막지 못하고 `-s` 는 AC 전용이다.
   상시 원격이 필요하면 **전원 연결 + 외부 디스플레이(closed-display 모드)** 또는 데스크톱 Mac 이 전제다.
3. **잠금 상태에서 화면 픽셀 전송은 하지 않는다.** LaunchDaemon + LoginWindow 에이전트 + TCC 화면 녹화 권한이
   필요한 별도 제품 아키텍처이며, IDE 의 부수 기능 범위를 벗어난다.

### 대안 (사용자 제시용)

- **A. 슬립 방지 + 동의 UI (권장)** — 전원 연결 시 자동, 배터리에서는 경고. 대가: 전력.
- **B. Wake on Demand** — 같은 LAN + sleep proxy 환경 한정. 첫 접속에 수 초~수십 초 지연, 환경 의존.
  기본 기능으로 삼을 수 없고 **보조 옵션**이 상한이다.
- **C. 관측 전용 폴백** — 슬립으로 서버가 죽어도 웹은 **마지막 스냅샷**(Rust 가 영속화한 세션 상태, ADR-0009)을
  읽기 전용으로 보여 주고 "오프라인" 배지를 단다. 슬립 중 접속의 체감 실패를 완화하는 현실적 절충.
- **D. 원격 상시 접속이 진짜 요구라면** 그건 데스크톱 IDE 가 아니라 **항상 켜진 호스트(서버/데스크톱 Mac)** 를
  전제로 하는 기능이다. VS Code Server / Codespaces 가 그 전제를 택한 이유가 이것이다.

### 로드맵 배치 제안

- Phase: `RemoteControl` capability 는 **core 계약(ipc-contract)의 전송 독립성**이 확보된 뒤에 착수한다.
  전송 독립성을 나중에 소급 적용하면 entities 전 도메인을 다시 만져야 한다.
- 지금 당장 해야 할 일은 구현이 아니라 **계약 설계 시 전송 가정을 넣지 않는 것** 하나다.

---

## 7. 함정·주의

1. **"잠금"과 "잠자기"를 한 단어로 다루지 말 것.** 잠금은 가능, 잠자기는 불가. 요구사항 문서에서 분리해 적는다.
2. **`kIOPMAssertionTypePreventSystemSleep` 을 쓰지 말 것.** deprecated 이며 OS X 릴리즈에서 미지원이다.
   실제로 쓸 수 있는 건 `PreventUserIdleSystemSleep` 뿐이다.
3. **assertion 은 lid close·명시적 잠자기·저전력을 못 막는다.** 헤더 주석에 명시돼 있다. 이걸 모르고 QA 하면
   "가끔 끊긴다"로 보인다.
4. **`caffeinate -s` 는 AC 전원에서만 유효**하다. 배터리 테스트 결과로 판단하면 오판한다.
5. **assertion 을 잡으면 App Nap 도 같이 회피된다** — 반대로 assertion 을 안 잡으면 서버가 살아 있어도
   타이머·I/O 스로틀로 응답이 느려질 수 있다. "죽었다"와 "스로틀됐다"를 구분해 로깅한다.
6. **assertion 해제 누락 = 배터리 버그.** 세션 종료·프로세스 크래시 경로 전부에서 해제를 보장한다.
   architecture.md §6.3 의 "Rust 자원은 세션 구조체가 소유(Drop + 명시적 shutdown 이중화)" 규칙을 그대로 적용한다.
7. **LAN 평문 HTTP 는 secure context 가 아니다.** 웹 클라이언트가 쓰는 브라우저 API 를 secure context 요구 목록과
   대조하지 않으면 "노트북에선 되는데 폰에선 안 된다"가 된다.
8. **macOS 15 로컬 네트워크 권한**은 Bonjour 광고를 켜는 순간 걸린다. 광고 없이 포트만 여는 설계면 회피 가능성이 크다
   (단정하지 말고 실측할 것 — 미확인 항목).
9. **방화벽 프롬프트는 서명 상태에 따라 다르게 보인다.** 개발 빌드 경험을 릴리즈 경험으로 일반화하지 말 것.
10. **웹 세션도 도메인 상태를 만들면 안 된다.** ADR-0004 는 전송 수단과 무관하게 유지된다. 웹 클라이언트도
    프로젝션이며, 편집 의도는 mutation 으로만 전달한다. 여기서 예외를 두면 두 뷰가 갈라진다.
11. **터미널 스크롤백·에디터 dirty 미러는 이미 Rust 에 있다**(ADR-0004 예외 2개). 웹 미러를 위해 새 미러를 만들지 말 것.
    대신 **커서/스크롤 위치**처럼 지금 계약에 없는 항목만 식별해 추가한다.
12. **원격 제어 = RCE 권한.** 인증 설계를 "나중에"로 미루고 포트부터 열지 않는다. LAN 노출은 opt-in.
13. **포트 하드코딩 금지.** 프로젝트 다중 오픈 시 충돌한다. Rust 가 할당하고 상태로 노출한다.
14. **`nosleep`(2022) / `fruitbasket`(2022) 은 정체 상태**다. 크레이트 채택 전 최신 갱신일과 실제 assertion 타입을 확인한다.

---

## 8. 남은 미확인 항목 (후속 조사 필요)

- Apple 공식 closed-display(clamshell) 모드 지원 문서 원문 — 전원 어댑터 필수 여부 확정.
- Apple silicon 에서 Power Nap 부재를 명시한 Apple 1차 문서.
- 화면 잠금이 사용자 세션 프로세스 스케줄링에 미치는 영향에 대한 Apple 1차 서술(현재는 DTS 답변의 세션 구분만 확인).
- `keepawake` / `nosleep` 이 실제로 잡는 assertion 타입.
- `objc2-foundation` 으로 `beginActivityWithOptions` 호출하는 정확한 시그니처.
- macOS 15 로컬 네트워크 권한이 **수신 리스너만 여는 앱**에도 적용되는지에 대한 Apple 1차 문서.
- JetBrains Remote Development 의 클라이언트-백엔드 전송 방식(UI 모델 vs 픽셀).
- WKWebView/Tauri 의 오프스크린 렌더 프레임 추출 가능 여부.

---

## 참고 링크

- IOPMLib.h 원문(assertion 타입 주석): https://github.com/opensource-apple/IOKitUser/blob/master/pwr_mgt.subproj/IOPMLib.h
- kIOPMAssertionTypePreventUserIdleSystemSleep: https://developer.apple.com/documentation/iokit/kiopmassertiontypepreventuseridlesystemsleep
- kIOPMAssertionTypePreventSystemSleep (deprecated): https://developer.apple.com/documentation/iokit/kiopmassertiontypepreventsystemsleep?language=objc
- caffeinate(8) man page: https://keith.github.io/xcode-man-pages/caffeinate.8.html
- Energy Efficiency Guide — Extend App Nap: https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/AppNap.html
- Energy Efficiency Guide — Schedule Background Activity: https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/SchedulingBackgroundActivity.html
- Set sleep and wake settings on Mac: https://support.apple.com/guide/mac-help/mchle41a6ccd/mac
- Turn Power Nap on or off on Mac: https://support.apple.com/guide/mac-help/turn-power-nap-on-or-off-mh40774/mac
- Understanding Sleep Proxy Service (Stuart Cheshire): https://stuartcheshire.org/sleepproxy/
- Apple Developer Forums #814152 — pre-login ScreenCaptureKit, daemon+agent: https://developer.apple.com/forums/thread/814152
- Apple Developer Forums #764374 — macOS 15 Local Network: https://developer.apple.com/forums/thread/764374
- Block connections to your Mac with a firewall: https://support.apple.com/guide/mac-help/block-connections-to-your-mac-with-a-firewall-mh34041/mac
- MDN Secure Contexts: https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts
- VS Code Remote Development: https://code.visualstudio.com/docs/remote/remote-overview
- The Visual Studio Code Server: https://code.visualstudio.com/blogs/2022/07/07/vscode-server
- JetBrains Remote Development: https://www.jetbrains.com/help/idea/remote-development-overview.html
- Control use of Chrome Remote Desktop (커튼 모드 macOS 지원 중단): https://support.google.com/chrome/a/answer/2799701?hl=en
- crates: https://crates.io/crates/keepawake , https://crates.io/crates/nosleep , https://crates.io/crates/objc2-io-kit , https://crates.io/crates/io-kit-sys
