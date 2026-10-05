# M8 native 충돌 배너와 파일 관찰

## 대상과 원본

- 원본은 `src/features/editor/conflict-banner.tsx`, `src/widgets/editor-pane/{editor-pane,use-editor-file-persistence}.tsx`/`.ts`, `src/entities/editor/model-registry.ts`입니다.
- 구현은 `native/taide-native-editor/src/{document,store}.rs`, `native/taide-native-ui/src/{conflict_banner,document_admission}.rs`, `native/taide-native-app/src/{file_sync,host,application}.rs`입니다.
- 새 의존성·root MSRV/제품 manifest 변경은 없습니다. 기존 실기 bundle과 시스템 설정을 유지했고 GUI 앱·OS 포트는 실행하지 않았습니다.

## 구현 경계

1. 원본의 세 배너와 실제 메시지 카탈로그를 연결했습니다. `mirrorRestored`는 warning 색과 닫기만 제공하고, `changedOnDisk`/`mirrorRestoredConflict`는 error 색과 View Disk/Keep Mine을 제공합니다. 원본의 12px 본문·14px 아이콘·24px 버튼·12/6px padding·8px gap을 사용합니다. 아이콘/버튼의 실제 픽셀 동등성은 아직 검증하지 않았습니다.
2. 프로젝트 파일 이벤트를 문서별로 합쳐 bounded host에서 다시 읽습니다. clean 실제 내용 교체는 공유 view의 선택을 clamp하고 composition/fold/undo를 초기화합니다. 같은 body의 관찰은 metadata만 갱신해 저장 후 undo를 유지합니다. dirty 문서는 초안/revision/undo를 유지하고 최신 disk baseline과 비교합니다. 이미 회수된 문서의 reply는 drop합니다.
3. View Disk/Keep Mine은 admission token의 root guard와 operation lease를 GUI commit까지 유지합니다. canonical identity·revision·정책 검사가 실패하면 body와 root layout을 바꾸지 않습니다. 성공하면 main/보조 창 및 동일 canonical layout 참조의 dirty 표시를 함께 정리합니다. View Disk는 새 revision으로 교체하고 이전 save 승인을 거절하며, Keep Mine은 body/undo/dirty/mirror를 유지합니다.
4. View Disk 뒤 mirror cleanup은 비교한 버전과 현재 버전이 같을 때만 수행합니다. 최신 mirror/중복 canonical mirror/변경된 경계는 삭제하지 않습니다. 큐 포화나 cleanup 실패 시 데이터가 남습니다. 기존 live 문서에는 stale 복원 안내를 재설치하지 않습니다. 같은 내용의 mirror도 원본처럼 dirty로 복원합니다.
5. 저장은 immutable snapshot의 canonical key와 현재 경로를 재확인합니다. symlink가 다른 파일을 가리키면 이전 문서 초안을 새 대상에 쓰지 않습니다. 실제 파일 정책을 재검사하고 기존 atomic write/self-write/mode/mirror 경로를 재사용하며, 성공 후 실제 disk metadata를 반환합니다.

## 변경 위험별 검증

공통 인자는 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 같은 성공 검사를 반복하지 않았습니다.

- [x] editor `--test disk`: 초기 3건 중 2건 성공, 0.00초입니다. 나머지는 readonly 문서의 undo를 `false`로 잘못 기대한 fixture였습니다. 기존 계약인 `ReadOnly` 기대값으로 수정하고 `--test disk view_disk` 1건만 재실행해 0.00초 통과했습니다. dirty 관찰/Keep Mine/undo, stale/canonical/cap/refused 선택, 공유 view 정리/readonly/old save 거절, 동일 저장의 undo 유지·실제 교체 초기화·동일 mirror dirty를 확인했습니다.
- [x] UI `--test conflict_banner`: 1건, 0.04초입니다. 초기 fixture는 headless frame의 texture delta를 소비하지 않아 egui Drop 검사에서 실패했습니다. 기존 headless fixture처럼 명시적 `clear` 후 같은 검사 1회 통과했습니다. 세 variant의 정확한 action 집합과 실제 pointer press/release 선택을 확인했으며 OS 실기를 뜻하지 않습니다.
- [x] app `--test disk`: 실제 host 1건, 0.04초입니다. restore notice/관찰/Keep Mine, main+보조 alias layout, 늦은 편집의 stale View Disk 거절, 새 disk 승인과 공유 body, 최신 mirror 보존/동일 버전 cleanup, reply drop과 tracked worker 0을 확인했습니다.
- [x] app `--test host native_save는_retarget`: 수정 전 1건이 실제 잘못된 저장으로 실패했습니다. canonical snapshot 검사 수정 후 1건, 0.01초 통과했습니다. 대상 두 파일 모두 보존하고 live 초안을 dirty로 유지합니다.
- [x] 영향받은 기존 검사만 재확인했습니다. app `--test host native_host` 2건 0.04초, editor `--test store mirror_복원` 1건 0.00초 통과했습니다. 나머지 성공 결과는 재사용했습니다.
- [x] 최종 strict clippy: app `--lib --bin taide-native-app --test disk --test host` exit 0/0.66초, editor `--lib --test disk` exit 0/0.16초, UI `--lib --test conflict_banner` exit 0/0.47초입니다. 최초 app 검사에서 신규 restore notice의 nested if를 지적해 let-chain으로 바꾼 뒤 해당 검사만 재실행했습니다.

## 남은 범위

- [ ] 편집 중 mirror debounce/epoch·autosave와 모든 닫기/프로젝트/창 진입점, 전역 canonical alias dirty settlement는 아직 완전 연결되지 않았습니다. 이번 결과를 그 성공으로 계산하지 않습니다.
- [ ] 이미 열린 파일이 외부에서 삭제됐을 때 missing-source 읽기 전용/Save As 전환은 남습니다. 현재는 실패를 알리고 live body를 유지합니다.
- [ ] 독립 외부 프로세스가 검사와 write 사이에 경로를 바꾸는 filesystem TOCTOU와 mirror 파싱 전 크기 상한은 남습니다. 이번 symlink 회귀는 검사 전에 완료된 identity 변경만 확인했습니다.
- [ ] atomic write가 성공한 뒤 reopen 또는 mirror cleanup이 실패하면 reply 실패와 실제 disk 쓰기가 공존할 수 있습니다. root 서비스의 이 경계를 rollback 성공으로 주장하지 않습니다.
- [ ] 실제 배너 픽셀·OS 입력/동작·대형 본문 shaping·전체 view 대응·terminal/기타 surface·최종 TS 제거/서명/배포는 남습니다. IME/VoiceOver와 fallback은 코드 구현 후 마지막 순서입니다.

N2-A2c4의 코드 연결 범위만 기록합니다. N2-A2c/N2/M8 전체 완료는 아닙니다.
