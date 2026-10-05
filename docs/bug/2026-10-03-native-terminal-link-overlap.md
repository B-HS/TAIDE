# Native terminal OSC8 우선순위의 URL 겹침

대상: `native/taide-native-app/src/terminal_links.rs`, `tests/terminal-links.rs`.

유효 OSC8이 URL 표시 문자열 중간 일부 cell에 걸려 있을 때, 그 cell 밖의 URL 부분을 hover하면 낮은 우선순위 URL 전체가 살아 있었습니다. 단순히 현재 cell의 OSC8만 우선했기 때문입니다. 원본 xterm Linkifier는 hover 물리 행에서 상위 provider와 겹친 하위 link의 전체 범위를 제외합니다.

현재 hover 행에 속하는 URL 범위 전체를 같은 Core grid의 유효 http(s) OSC8과 대조해 겹치면 URL을 제외합니다. Non-http/invalid OSC8은 우선 provider의 승인 대상이 아니므로 평문 URL을 차단하지 않습니다. 다른 물리 행의 OSC8까지 임의로 확장하지 않습니다.

`--test terminal-links 같은_물리행`은 수정 전 `Some(plain URL) != None`으로 실패했고, 수정 후 유효/invalid OSC8 두 경우를 포함한 1건이 suite 0.01초에 통과했습니다. 변경 영향을 받지 않은 기본 4 PASS는 재사용합니다. OS/전체 cell·wrap matrix는 terminal-links QA에 남깁니다.
