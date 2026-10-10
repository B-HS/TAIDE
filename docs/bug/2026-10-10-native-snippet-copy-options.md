# 스니펫 기본값 복제 시 변환 옵션 누락

현재 상태: 배치 22에서 실패 재현 후 수정했습니다. 자동완성 화면/입력 연결은 별도 진행 중이며 이 수정의 완료를 배치 완료로 계산하지 않습니다.

`native/taide-native-editor/src/snippet-normalization.rs`의 기본값 복제는 정규식 source와 i/g만 복사했습니다. `${1:${name/(.+)/${1:/upcase}/s}} $1`처럼 변수 변환을 포함한 기본값을 복제하면 작성한 s 옵션이 사라졌습니다. 원본 Monaco의 같은 복제 결함을 강제로 재현하지 않고, compiler가 검증한 d/m/s/u/v/y도 보존합니다. editor/UI에 정규식 엔진을 반입하지 않습니다.

`cargo test --test snippet-options --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`는 복제 경로 수정 전 `/private/tmp/taide-batch22-snippet-options-copy-first.log`에서 1실패(s 기대값과 빈 문자열 불일치), 수정 뒤 `snippet-options-after-copy.log`에서 exit 0·1통과/0실패/0ignored입니다. 단일 변수의 작성 순서를 정규화 순서로 기대한 최초 검사는 실제 복제 경로를 검사하지 않아 수정했으며 성공 근거로 세지 않습니다.

스니펫 삽입 전체 5건과 completion 삽입 5건은 `snippet-session-limits-after-cost.log`와 `completion-insertion-after-session-cost.log`에서 각각 exit 0입니다. 세 명령의 target directory는 위와 같습니다. 실제 클립보드/앱 데이터·OS 입력·보호 앱 번들을 사용하지 않았습니다. 배치 22의 전체 대상/동결 통합 게이트는 아직 남아 있습니다.
