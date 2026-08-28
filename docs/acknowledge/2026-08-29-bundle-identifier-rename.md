# 번들 identifier 변경 — net.gumyo.taide (2026-08-29, 사용자 확정)

## 결정

- 앱 도메인이 `taide.gumyo.net` 으로 확정됨에 따라 번들 identifier 를 역방향 DNS 관례대로
  **`dev.taide.app` → `net.gumyo.taide`**(dev 오버레이는 `dev.taide.app.dev` →
  **`net.gumyo.taide.dev`**) 로 변경한다. 공개 배포 전 마지막 저비용 시점이라는 판단.
- identifier 는 app_data_dir·로그·keyring 서비스명(d-49 에서 identifier 파생화 완료)·TCC 의
  네임스페이스다 — 변경 시 기존 데이터가 다른 앱 취급되므로 로컬 이관을 동반한다.

## 절차

1. 코드·설정: `tauri.conf.json`·`tauri.dev.conf.json` identifier, `e2e/lib/paths.ts` 경로
   상수 3종, 버전 0.1.2 동기(3파일). 레포 전수 grep 으로 잔여 참조 확인(과거 시점 기록 제외).
2. 로컬 데이터 이관(1회): `~/Library/Application Support/dev.taide.app` →
   `net.gumyo.taide` 복사(dev 디렉토리도 존재 시 동일). 로그·캐시는 이관 불요(자동 재생성).
3. **키체인은 이관 불가(시크릿 비접촉 원칙)** — 설치본에서 AI 토큰·원격 접속 비밀번호·GitHub
   PAT 재입력 필요. 원격 비밀번호는 재설정 전까지 원격 접속이 미구성 상태가 된다.
4. 재빌드 → 사용자 재설치·기존 프로젝트 목록 승계 확인 → v0.1.2 릴리스.
