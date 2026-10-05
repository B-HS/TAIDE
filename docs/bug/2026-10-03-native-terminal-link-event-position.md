# Native terminal 링크 이벤트 위치 재사용

대상: `native/taide-native-app/src/terminal_surface.rs`의 `show_external_link`, `tests/terminal-host.rs`의 `terminal_links는`.

같은 프레임에서 OSC8 위치를 press하고 다른 URL 위치에서 release하면 잘못된 URL command가 생성됐습니다. 각 PointerButton에 최종 hover link를 재사용했기 때문에 down/up이 동일한 것처럼 보였습니다. 기존 서로 다른 프레임 검사는 이 입력을 덮지 않았습니다.

이벤트별 실제 `pos`로 같은 grid의 링크를 다시 읽고 원본 Linkifier처럼 down/up의 URI/range 값이 같을 때만 활성화합니다. 최종 hover는 cursor/underline에만 사용합니다. 단순 output revision의 증가를 다른 링크로 해석하는 과잉 거절도 제거했습니다.

새 같은-frame RED 1건(suite 0.19초)을 확인한 뒤 관련 actual PTY/egui/host 통합 1건이 0.53초에 통과했습니다. 종료 exit 0·host/Hub join·tracked task 0입니다. 원본 TS·기존 앱·OS browser는 수정/호출하지 않았습니다. Multi-button·출력과 이동 혼합·전체 OS matrix는 terminal-links QA에 남깁니다.
