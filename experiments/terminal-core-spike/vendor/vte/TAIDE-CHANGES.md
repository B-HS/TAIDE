# vte 0.15.0 격리 실험용 수정

crates.io vte 0.15.0의 source·manifest·README·license·example을 보존한 실험용 복사본입니다. 외부 링크로만 참조되는 doc와 변경 이력은 포함하지 않습니다. 기존 라이선스·SPDX·저작권·주석은 보존합니다. 특히 `src/ansi.rs`는 Alacritty에서 유래한 Apache-2.0 모듈이므로 `LICENSE-APACHE`를 함께 보존하며 MIT-only 코드라고 주장하지 않습니다.

`src/lib.rs`는 표준 라이브러리 활성화 시에도 const OSC raw buffer 상한을 적용하고 기본값을 4096 bytes로 둡니다. 초과 OSC는 종료 전까지 버퍼에 추가하지 않으며 잘린 명령을 dispatch하지 않습니다. 이후 정상 OSC는 다시 처리합니다. 기존 oversized OSC unit의 기대값도 “무제한/잘린 명령 실행”에서 “명령 폐기”라는 강화된 계약으로 변경했습니다.

`src/ansi.rs`에는 `OscObserver`와 Processor의 설정 경계를 추가합니다. 기존 단일 ANSI parser가 OSC를 dispatch하는 시점에 observer를 호출하며 terminal grid 처리는 기존 Handler로 이어집니다. raw bytes를 별도 scanner로 다시 파싱하지 않습니다. synchronized update에서도 같은 ProcessorState가 observer를 소유합니다. 미지원 OSC debug 로그는 외부 payload를 출력하지 않습니다.

추가 원형 payload buffer는 structured params의 16개 제한으로 본문 구분자가 유실되는 문제를 피합니다. 같은 parser의 OSC 상태에서 구분자·C0를 포함한 원형을 보존하며 두 buffer는 각각 const 상한 이내입니다. 상한은 원형 전체를 기준으로 검사하므로 무시되는 문자로 우회하지 못합니다. 원형 callback은 BEL 또는 완전한 ST 뒤에만 호출하며 ESC만 수신·CAN/SUB·다른 escape로 전환되는 OSC를 소비하지 않습니다. 기존 params dispatch의 upstream 동작은 유지합니다.

`StreamObserver`와 setter는 같은 Performer의 print·execute·CSI도 관찰하도록 확장합니다. 기존 `OscObserver` 이름과 setter는 호환 경계이며 미등록 상태의 Handler 처리에는 변화가 없습니다. text consumer는 구조화된 callback으로만 정규화하므로 raw stream의 두 번째 parser를 추가하지 않습니다. 제품 렌더·효과의 동등성과 callback 비용은 아직 판정하지 않습니다.

이번 복사본은 제품 의존성이 아니라 `terminal-core-spike`와 격리 `native/taide-native-terminal`에서 사용합니다. 후속 typed core의 grid/borrowed content·feed/effect·OSC52·제목 복원 기본 결과는 `docs/quality-assurance/2026-10-02-m8-native-terminal-core.md`입니다. 실제 PTY/native renderer consumer·정규화/총메모리/log·전체 제품 동등성은 미완료입니다. 다른 Cargo root는 이 patch를 자동 상속하지 않으므로 기존 제품/native app과 WezTerm 비교는 이 patch를 사용하지 않습니다. 선행 실험 결과와 남은 조건은 `docs/quality-assurance/2026-09-30-m8-editor-terminal-headless.md`에 기록합니다.

native-retained의 기본 noop `Handler::command_marker`는 원형 OSC payload의 `133;` prefix 뒤에만 같은 parser의 완전 dispatch 시점에서 호출합니다. observer의 기존 C/D notification 경계를 변경하지 않으며 structured params가 C0를 제거하거나 16개 뒤를 잃는 문제를 marker 파싱에 반입하지 않습니다. feature-off Handler/API에는 새 callback이 없고 별도 scanner도 없습니다. 실제 marker/fragmented/Number·기본 app/PTY 결과와 overview/remount/전체 VT 미완료는 `docs/quality-assurance/2026-10-03-m8-native-terminal-commands.md`에 기록합니다.
