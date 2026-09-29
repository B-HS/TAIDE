# M7 부팅 화면 표시 준비 대리지표 결정

## 대상 파일과 결정

`docs/quality-assurance/2026-09-04-perf-baseline.md`의 지표 1과 `src/shared/lib/perf-mark.ts`·`src/shared/hooks/use-reveal-window.ts`의 현행 계측을 대상으로 합니다. 사용자는 M7·Phase 0의 첫 가시 페인트 직접 픽셀 시각 대신, 동일 부팅에서 측정한 WebKit 첫 콘텐츠 준비와 Tauri 창 표시 완료를 결합한 **화면 표시 준비 시각**을 기준선으로 채택했습니다. 이후 native 비교도 이 경계를 사용하며 실제 픽셀 페인트 시각으로 부르지 않습니다.

## 리포트와 상세

[단일 release 부팅 실측](../quality-assurance/2026-09-29-m7-perf-readout-live.md)의 OS 프로세스 시작 시각부터 WebKit `first-contentful-paint`까지 약 `1119.420ms`, Tauri `show()` 완료까지 약 `1120.420ms`였습니다. 같은 부팅에서 콘텐츠 준비가 창 표시 완료보다 약 1ms 먼저였고 `first-paint` 항목은 없었습니다. 따라서 약 `1120.420ms`는 두 조건을 모두 충족한 표시 준비 경계이지 화면 픽셀 도달 시각이 아닙니다. 사용자의 후속 실제 전면 앱 확인은 프레임 probe `fired`와 터미널 문자 가시성을 입증하지만 부팅 직후 픽셀 시각을 소급해 산출하지 않습니다.

이 결정은 M7의 단일 기준선만 닫습니다. 이후 native 성능 동등성을 정량 비교할 때는 같은 기기·fixture의 현행 앱과 native 앱에서 동일한 표시 준비 경계를 다시 측정해야 하며, 화면 픽셀·프레임 시간과 통계적 비악화는 별도 검증입니다.
