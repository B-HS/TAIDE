# Native toast 위치 변경 수명 차이

## 대상 파일

`native/taide-native-app/src/{toast.rs,application.rs}`

## 리포트

원본 Sonner은 목록 position key 변경으로 child Toast를 remount하지만 native는 기존 timer/motion을 유지한 채 geometry만 옮겼습니다. baseline에서2초가 지난 bottom-right를 top-right로 바꾸면 remaining2초였고 원본 key 계약의 기대는4초였습니다.

## 상세

1. Top/Middle은 원본에서 동일 Sonner key이며 horizontal 또는 Top/Bottom 변경만 local 상태를 초기화합니다. native에 마지막 parsed/canonical key와 widget mount generation을 연결했습니다.
2. 기존 dismissed_at은 local exit 그림과 부모200ms 제거를 한 필드로 표현했습니다. 위치 remount가 local removed를 초기화해도 원본의 예약 callback은 살아 있으므로 scheduled_removal_at을 분리하고 최초 deadline을 유지합니다.
3. 현재 Settings를 App tick 전에 읽고, show도 같은 관찰을 적용합니다. 기존 bounds/hit cache와 capture를 새 위치 수명에 맞추고 stale AX widget ID를 적용하지 않습니다. parent pause와 queue 내용은 보존합니다.
4. 최초 RED와 변경 영향21 PASS/1 FAIL·fixture 수정 뒤 focus1 PASS의 정확한 결과는 `docs/quality-assurance/2026-10-03-m8-native-toast-position-lifetime.md`에 있습니다. identity constructor 치환 컴파일 오류와 과거 즉시 ring 기대 오류도 기록했습니다. 실제 browser/full App/OS/전체 M8 동등성은 미완료입니다.
