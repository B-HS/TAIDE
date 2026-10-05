# Native App 종료의 서버 정리 누락

## 대상과 증상

대상은 `native/taide-native-app/src/application.rs`의 shutdown/on_exit입니다. 기존 native shutdown은 Host/LSP/Web 작업과 draft/layout flush 뒤 ExitDrain만 실행했습니다. 감독 task는 종료돼도 IDE/hooks/remote store의 명시적 중지·IDE lockfile 삭제·pending 응답·원격 세션 회수는 호출되지 않았습니다.

## 재현과 원인

합성 UUID data 디렉터리에서 실제 IDE/hooks/remote localhost 서버를 시작하고 IDE lockfile·save/diff pending·원격 세션을 만든 뒤 실제 App shutdown을 호출했습니다. 종료 admission=true/task0을 통과했지만 IDE running=true인 RED입니다. sandbox bind 실패는 별도 환경 실패로 기록하며, loopback 한정 escalation에서 실제 RED를 확인했습니다.

on_exit fallback은 is_shutting_down=false인 경우에만 재정리했습니다. shutdown 시작 표시가 true라는 사실은 flush/drain 성공을 증명하지 않으므로, 이미 shutdown 표시가 있는 오류 경로에서는 자원 정리를 건너뛸 수 있었습니다.

## 수정과 검증

정상 shutdown과 직접 종료 오류 fallback이 같은 drain_services를 사용합니다. begin_shutdown→기존 validated CLI wait marker cleanup→remote/hooks/IDE stop→search cancel→ExitDrain이며, 정상 종료에서 draft/layout flush는 선행합니다. 직접 Exit는 pending 결과의 실제 성공/오류로 fallback 여부를 판단하고 기존 오류를 보존합니다. state flag를 완료 증거로 사용하지 않습니다.

`application::exit_tests`의 수정 후 고유1 PASS(compilation5.99초/suite0.08초)입니다. 정상 경로와 이미 shutdown 표시가 있는 합성 draft 오류 경로에서 실제 서버·세션·IDE pending/lockfile·marker·검색 cancellation·listener 폐쇄/admission 거절/task0을 확인했습니다. 실제 OS 종료/GPU/GUI·원격 활성 WS/UI·제품 startup은 해당 기존 gate에 남습니다.
