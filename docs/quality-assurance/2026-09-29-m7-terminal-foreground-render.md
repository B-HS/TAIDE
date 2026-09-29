# M7 전면 터미널 200만 줄 렌더 단일 표본

## 대상 파일과 환경

`src/shared/lib/perf-mark.ts`, `src/widgets/settings-view/settings-performance-section.tsx`, `docs/quality-assurance/2026-09-04-perf-baseline.md`를 대상으로 판정했습니다. `TAIDE_PERF=1`의 격리 release 앱 `/private/tmp/taide-m7focus.eVkPA8/TAIDE.app`과 합성 프로젝트만 사용했습니다. 원격 서버는 `off`·`Stopped`였습니다.

## 리포트

사용자가 앱을 실제 macOS 전면에 둔 뒤 `animationFrameProbe=fired`와 터미널 문자 표시를 보고했습니다. 같은 앱의 화면 캡처에서 프롬프트와 문자 표시를 확인했고 로컬 스냅샷은 `terminal.render-frames=25`였습니다. 이는 앞선 비전면 실행의 `timed-out`·렌더 0건과 다른 조건입니다. UI 자동 입력 `seq 2000000` 시도는 Rust 출력 378바이트뿐이라 명령 실행 표본으로 사용하지 않았고 같은 입력을 반복하지 않았습니다.

## 단일 출력과 상세

앱 PID 84419와 자식 셸 PID 85850·`/dev/ttys000`을 확인하고 지표를 초기화했습니다. 첫 예약은 TTY 표시값을 `s000`으로 비교한 안전 확인에서 exit 3으로 중단돼 출력이 시작되지 않았습니다. 실제 TTY `ttys000`과 부모·앱 경로를 다시 확인한 후 `/usr/bin/seq 2000000`의 출력을 해당 PTY에 한 번만 보냈습니다. writer exit 0, `/usr/bin/time -p`의 `real 1.42s`입니다.

| 관찰 | 값 |
| --- | --- |
| Rust `pty.output_bytes`·`pty.output_chunks` | 20,777,785바이트·464청크 |
| 프런트 전달·파서 완료 | 각각 20,778,944바이트·466청크 |
| 프런트 `terminal.render-frames` | 89회 |
| 마지막 파서·렌더 시각 | `1790661002144ms`·`1790661002149ms` |
| 측정 후 프레임 probe | `fired` |
| 최종 화면 | 마지막 `2e+06` 행이 보임 |

프런트에 더해진 1,159바이트·2청크에는 터미널 재활성화 출력이 포함돼 순수 `seq` 크기로 취급하지 않습니다. Rust 수신량을 writer 경과로 나눈 약 `14.63 MB/s`는 이 한 번의 PTY 쓰기·수신 참고값입니다. 픽셀 렌더 자체의 바이트/초나 반복 분포는 측정하지 않았습니다. 동일 출력에서 파서 완료 5ms 뒤 렌더 이벤트와 마지막 행 가시성을 함께 확인했으므로 기준선 지표 8의 단일 관찰로 채택합니다. 이전 [비전면 실패 기록](2026-09-29-m7-perf-readout-live.md)은 당시 상태로 보존하며, 이미 통과한 다른 지표는 재실행하지 않았습니다.
