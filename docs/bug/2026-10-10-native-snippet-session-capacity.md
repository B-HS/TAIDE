# 스니펫 세션 용량의 수락 전 검증

현재 상태: 배치 22에서 실패 재현 후 수정했습니다. 자동완성 화면/입력 연결과 배치 전체 게이트는 별도 진행 중입니다.

`native/taide-native-editor/src/snippet-insertion.rs`는 삽입 텍스트·placeholder 개수와 줄 들여쓰기의 용량을 각각 검사했습니다. `Session::new`가 사용하는 choice/transform marker 비용과 들여쓰기의 합계, 여러 커서의 choice marker 합계는 문서를 바꾼 뒤에야 거절될 수 있었습니다. 삽입 전에 같은 `own_cost` 계약으로 총 bytes/markers를 합산하고 줄 들여쓰기도 포함합니다. 용량 거절은 문서·선택·undo를 바꾸지 않습니다.

`cargo test --test snippet-insertion snippet_session_limits --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`는 `/private/tmp/taide-batch22-snippet-session-limits-first.log`에서 2실패였습니다. 들여쓰기 768바이트/미선택 choice 512바이트와 두 커서의 65개 choice로 각각 재현했습니다.

수정 뒤 같은 manifest/flags의 `cargo test --test snippet-insertion`은 `snippet-session-limits-after-cost.log`에서 exit 0·5통과/0실패/0ignored입니다. 실패 재현 2건과 기존 삽입 3건이 모두 통과했습니다. 영향 `cargo test --test completion-insertion`은 `completion-insertion-after-session-cost.log`에서 exit 0·5통과/0실패/0ignored입니다. 선택·공유뷰·readonly/IME·stale/겹침·원자적 거절·다중 커서/들여쓰기/CRLF·mirror/tabstop 회귀를 확인했으며 실제 사용자 데이터나 OS 클립보드/합성 입력은 사용하지 않았습니다.
