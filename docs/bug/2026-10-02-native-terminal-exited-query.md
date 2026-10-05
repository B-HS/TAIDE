# 종료한 child의 query가 최종 terminal grid를 폐기한 오류

## 대상·원인

`native/taide-native-app/src/{terminal_host,terminal_dispatch}.rs`입니다. 실제 PTY completion join 뒤 flush한 마지막 synchronized Frame에도 color/geometry query가 남을 수 있습니다. 종료한 child에 응답하려다 발생한 port/IO 오류를 FinalFrame 실패로 처리하면서 유효한 Core/grid까지 retire했습니다.

## 재현·해결

합성 helper의 `TAIDE_NATIVE_FIXTURE_FINAL_QUERY=1`에서 마지막 sync payload에 color·geometry query를 넣고 종료했습니다. 종료 후 query port 호출을 명시적으로 거절하는 actual Hub 검사 1건이 `Some(FinalFrame)`으로 실패했습니다(exit 101, compile 2.61초/suite 0.30초). 실패 assertion 전에 Hub와 root workers를 회수합니다.

실제 exit callback의 `exit_code` 보고로 응답 가능 여부를 판단합니다. 종료 보고 뒤에는 PtyWrite·typed color/geometry 응답만 생략하고 metadata/agent/stream/title/UI effect·revision 순서는 유지합니다. clipboard/opaque effect의 금지는 유지하며 기존 Dispatcher API는 live 응답 정책으로 위임합니다.

수정 뒤 같은 실패 검사만 1회 실행해 1 PASS(compile 4.38초/suite 0.34초), app lib/terminal-host/terminal-dispatch strict exit 0(1.98초)을 확인했습니다. 기존 성공 runtime은 반복하지 않았습니다. child가 live 확인 직후 종료하는 OS race·전체 GUI/성능은 이 검사로 완료 주장하지 않습니다.
