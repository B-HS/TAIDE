# Native Settings 저장 실패 출처 유실

## 대상 파일

`native/taide-native-app/src/{host,application,presentation,toast}.rs`

## 리포트

키맵/글자 크기 저장 실패가 `HostReply::Failed`로 합쳐져 원본 Settings mutation의 error toast 제목/description을 재현하지 못했습니다. 실제 합성 Settings 쓰기 실패 검사에서 expected SettingsFailed assertion RED(exit101, suite0.06초)를 재현했습니다.

## 원인·해결

비동기 reply가 요청 종류를 잃었고 App은 일반 오류를 status에만 표시했습니다. `SettingsFailed(AppError)` typed reply를 추가하고 Settings 즉시 submit 오류와 비동기 disk 오류를 같은 toast로 연결했습니다. 성공은 기존 state/disk/event 경로이며 실패에 낙관적 값을 반영하지 않습니다. localized 오류는 key/args/fallback, 비localized 오류는 IPC raw 문자열을 보존합니다.

## 검증·잔여

keybinding host1 PASS(0.03초), font host 영향1 PASS(0.04초), raw/localized title/description model과 actual native card 검사는 toast QA에 기록했습니다. strict app lib/bin/test exit0(11.32초)입니다. 모든 Settings UI/다중 창/late/AX·원본 toast 전체 animation 완료는 아니며 `docs/quality-assurance/2026-10-03-m8-native-toasts.md`가 정본입니다.
