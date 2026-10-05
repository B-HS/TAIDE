# HTML 준비 입력의 stat/read 크기 차이

## 대상과 관찰

`crates/taide-file/src/service.rs::read_raw`는 metadata.len을 기존 20MiB와 비교한 뒤 `std::fs::read`로 전체를 읽습니다. 두 호출 사이 파일이 커지면 metadata 검사를 통과한 추가 bytes가 할당될 수 있습니다. 이번 새 HTML 연결에서 이를 확인했으며 다른 기존 provider/root API를 임의로 수정하지 않았습니다.

## 해결

`native/taide-native-app/src/preview_web.rs`는 기존 root/CLI/read_approved 인가 아래 File을 열고 열린 File의 regular-file/크기를 검사합니다. 실제 읽기는 64KiB 고정 chunk와 누적 checked_add, 입력 상한을 넘기기 전 오류·try_reserve_exact로 수행합니다. metadata가 실제 최종 길이를 보증한다고 가정하지 않습니다. 초과 확인용 chunk는 stack에만 있으며 document Vec에는 상한 초과 bytes를 붙이지 않습니다. logical bytes/requested Vec capacity 경계이지 allocator/OS 전체 RSS 보증은 아닙니다.

## 검증

- [x] 신규 `--lib html_bounded_read` 1 PASS, compile 2.23초·suite 0.01초입니다. 실제 reader의 정상/빈 입력, metadata와 무관한 20MiB+1 입력의 거절, 정확한 20MiB의 bytes/capacity를 검사합니다. 별도 byte fixtures이며 기존 host/approval/client 성공은 반복하지 않습니다.
- [ ] 다른 기존 provider의 동일 read_raw 패턴과 OS symlink/open 경쟁·특수 파일 open blocking은 별도 실제 위험 단위 검증이 필요합니다. HTML renderer/resource 연결 및 전체 M8 완료 판정 전 해당 경계의 admission을 다시 확인합니다. 이번 제한만으로 모든 filesystem race·peak RSS·전체 provider 보안을 완료 처리하지 않습니다.
