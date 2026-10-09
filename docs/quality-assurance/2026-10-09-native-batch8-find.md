# 배치 8 찾기·치환 코어와 엔진 게이트

현재 상태: 앱 전용 엔진과 순수 찾기·치환 코어를 검증했습니다. 위젯·명령 연결, 배치 전체 대상 검사는 미완료입니다. 전체 기능 완료율이나 실기 검증 통과를 뜻하지 않습니다.

## 결정·경계

기준은 `../acknowledge/2026-10-09-native-find-regex-decisions.md`, `../acknowledge/2026-10-09-native-full-resume.md`, 실제 Monaco 0.56.0 소스입니다. 패키지의 실제 기준 필드는 `vscodeRef`이며 `f487add297079a02eb836810185b165e50cadabc`입니다. 원본의 오류·버그·내부 수치를 강제로 재현하지 않습니다.

`regress =0.12.0`은 syntax에만 선언합니다. 앱·syntax lockfile에 regress 한 패키지와 syntax의 의존 항목만 추가하고 기존 패키지 버전은 바꾸지 않습니다. 새 전이 패키지는 없으며 memchr를 재사용합니다. editor/UI에는 엔진 의존성을 추가하지 않습니다. remote-web의 코드·manifest·lockfile은 수정하지 않습니다. 기존 ferriki-textmate 0.12.0·ferroni 1.8.1 구문 강조 경로는 유지합니다. 고지는 `../../THIRD_PARTY_LICENSES.md`에 있습니다.

API는 [regress 0.12.0 공식 문서](https://docs.rs/regress/0.12.0/regress/)와 실제 registry 소스의 `src/api.rs`를 확인했습니다. `u`를 항상 켜고 `i`·`m`을 조건부로 적용하며 바이트 시작 위치와 캡처를 주고받습니다. 예외적으로 잘못된 UTF-8 시작 위치는 엔진 호출 전에 거부합니다. 오류 문장은 엔진의 영어 문장을 사용합니다.

## 원본 비교와 재현

`../utils/2026-10-09-monaco-find-oracle.js`는 설치된 원본 SearchParams·TextModelSearch·ReplacePattern을 직접 호출해 `native/taide-native-syntax/tests/fixtures/find-reference.json`을 생성합니다. DOM·OS·실제 앱 데이터는 사용하지 않습니다. 위치는 UTF-16에서 원본 UTF-8 바이트로 변환합니다.

| 대상 | 기준값·결과 |
| --- | --- |
| 엔진 | 문법 오류, u/iu/mu/imu, 문자 클래스·유니코드 속성·폴딩·전후방 탐색·후방 참조·캡처·시작 위치 3,800개 일치, target 2건 통과 |
| 찾기 | 리터럴/정규식·대소문자·단어 옵션·LF/CRLF·빈 일치·보충 문자·선택 범위·탐색 2,432개 검증 |
| 치환 패턴 | 정규식/리터럴, 없는·참여하지 않은 그룹, 두 자리 번호, 달러·이스케이프·UTF-16 대소문자 지시·대소문자 보존 2,624개 일치 |
| 순수 코어 | engine 없는 리터럴, 범위 정렬·중복 방지, 경계 오류, 시간 초과, 순환, 선택/단어 시드, 20,000건의 전체 단어 치환·undo/redo, CRLF·다른 뷰의 선택, stale revision·읽기 전용·선택 영역 치환 13건 통과 |

줄바꿈을 단어 경계의 안쪽 구분자로 취급한 초기 구현은 원본과 116개 차이가 났습니다(exit 101). 원본은 바깥의 CR/LF 경계와 내부 구분자 검사를 따로 하므로 수정한 뒤 원본 비교가 통과했습니다. NBSP·U+2028·U+2029와 `.*`·`\s+`도 기준값에 추가했습니다.

이전 탐색에서 잘린 접두 문자열의 `$`와 단어 경계를 인정하는 경로는 원본 결함입니다. `^$`를 `cat`의 처음에서 찾거나 `cat$`·전체 단어 `cat`을 `catapult`의 3바이트 위치에서 이전 탐색하면 실제 문서에 없는 일치가 생깁니다. 추가 회귀가 수정 전 실패(exit 101)했으며 전체 문맥에서 일치 끝이 커서 이하인 결과를 선택하도록 수정했습니다. fixture에는 원본 `previous`와 전체 문맥으로 구한 `previousWithFullContext`를 함께 보존하고 후자를 기대값으로 검증합니다. 원본의 여러 줄 이전 탐색 9,990개 잘림도 강제하지 않습니다.

대량 치환은 강조 한도와 관계없이 같은 query로 전체 결과를 구합니다. 시간 초과 시 치환 계획을 반환하지 않으며 적용 단계는 revision·읽기 전용·뷰 소유권을 기존 store 경계에서 검증합니다. 모든 치환을 하나의 별도 undo 그룹으로 적용합니다.

실행한 관련 명령은 다음과 같습니다. 공통으로 `--locked --offline --target-dir experiments/native-shell-spike/target`을 사용했습니다.

1. `cargo test --manifest-path native/taide-native-syntax/Cargo.toml --test find-regex-gate -- --nocapture`: exit 0, 2 통과, 실행 0.11초. 로그 `/private/tmp/taide-batch8-regress-gate-20261009.log`입니다.
2. `cargo test --manifest-path native/taide-native-syntax/Cargo.toml --test find-model -- --nocapture`: 마지막 exit 0, 3 통과, 실행 1.54초. 로그 `/private/tmp/taide-batch8-find-model-context-20261009.log`입니다.
3. `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test find`: 마지막 exit 0, 13 통과입니다. 로그 `/private/tmp/taide-batch8-find-core-seed-after-20261009.log`입니다.
4. `cargo check --tests --manifest-path native/taide-native-app/Cargo.toml`: exit 0, 23.71초입니다. 로그 `/private/tmp/taide-batch8-find-app-check-20261009.log`이며 기존 vendored wry 경고가 남습니다.

전체 크레이트 테스트가 아닌 변경 위험을 덮는 관련 target 검사입니다. 전체 대상은 배치 통합 단계에서 한 번 직접 실행합니다.

## 성능·폭주

`examples/find-regex-probe.rs`와 `../utils/2026-10-09-monaco-find-performance.js`는 같은 ASCII 300,000줄·20,971,520바이트 문서를 만듭니다. 문서 구성과 컴파일 시간은 검색 시간에 포함하지 않습니다. 설치된 Monaco를 Bun/JSC에서 실행한 비교이며 실제 WKWebView의 제품 화면 검사는 수행하지 않았습니다. 한 번의 로컬 관찰값이며 실기 UI 지연·메모리 게이트를 대신하지 않습니다.

| 경로 | 전체 무일치 검색 | 19,999개 수집 |
| --- | ---: | ---: |
| 엔진 직접, debug | 124.432291ms | 32.264833ms |
| 코어 초기, debug | 5,408.966791ms | 398.656042ms |
| 줄 iterator 수정 후, debug | 922.543666ms | 106.380208ms |
| 최적화 코어, 상한 없음 | 25.759417ms | 4.201541ms |
| 최적화 코어, 1초 상한 | 32.256333ms | 4.6865ms |
| Monaco/JSC, 같은 문서 | 11.545458ms | 1.769584ms |

초기 코어가 줄마다 반복한 rope 트리 조회를 순차 줄 iterator로 바꿨습니다. 사용자 합격 기준은 두 최적화 검색 모두 100ms 이내이며 최종 실행에서 실제 assert를 통과했습니다(exit 0). 한도 수집 19,999건과 무일치 0건, 시간 초과 없음도 검사했습니다.

최종 실행: `cargo run --release --manifest-path native/taide-native-syntax/Cargo.toml --example find-regex-probe --locked --offline --target-dir experiments/native-shell-spike/target -- model`. 로그 `/private/tmp/taide-batch8-find-model-performance-accepted-20261009.log`입니다. Monaco 비교 로그는 `/private/tmp/taide-batch8-monaco-performance-20261009.log`입니다.

`(a+)+b`와 32개의 `a`는 5초 안에 종료되지 않았습니다. `../utils/2026-10-09-regress-backtracking-probe.js`가 별도 검사 프로세스를 5초 후 SIGKILL로 종료했으며 exit 137, 경과 5,001.834ms입니다. 제품에 이 프로세스 전략을 도입하지 않습니다. 0.12.0은 실행 중단 API가 없고 줄·일치 사이의 1초 확인으로 한 번의 엔진 호출을 멈출 수 없다는 승인 조건을 확인한 것입니다. 로그 `/private/tmp/taide-batch8-regress-backtracking-20261009.log`입니다.

## 남은 완료 조건

- [x] 정확한 엔진 버전·기준값·성능 기준·폭주 조건 확인
- [x] 찾기·치환 순수 코어와 관련 회귀 검증
- [ ] 일치 선택/탐색 상태·문서 revision 추적과 위젯 구현
- [ ] 앱 명령·키·포커스·다중 뷰 수명 연결
- [ ] 변경 크레이트 전체 대상·동결 컴파일·포맷·디스크·실기 부채를 배치 통합 문서에 기록

실제 OS 입력·클립보드·Keychain·Trash를 호출하지 않았습니다. 화면 잠금 상태의 실제 UI/IME 검증 부채는 보존하며 합성 OS 입력으로 우회하지 않습니다.
