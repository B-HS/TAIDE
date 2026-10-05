# 원격 저장 성공 후 복원 배너 잔존

## 대상

`native/taide-remote-web/src/files.rs`의 성공 save 응답과 같은 문서의 restore notice, `tests/files.rs`의 실제 두 view surface입니다.

## 증상·원인

신규 readonly/conflict surface 구현 중 restore_notice를 가진 두 view의 실제 shared 배너가 저장 성공 뒤에도 남았습니다. 성공 mark_saved는 baseline/dirty를 갱신했으나 새 notices map을 정리하지 않았습니다. 기존 native App의 SaveFinished는 같은 문서의 모든 tab restore_notices를 제거합니다.

## 해결·관찰

성공 저장과 디스크 선택이 같은 clear_notices(document)를 사용하도록 연결했습니다. unrelated document notice와 실패 notice는 유지하며 단순 저장 요청 때 미리 제거하지 않습니다.

새 actual surface 테스트에서 저장 전 [true,true], 저장 성공 후 관찰 [true,true]/기대[false,false] RED를 확인했습니다(build0.35초/suite0.03초). 수정 후 실패1만 exact 실행해 [false,false] GREEN입니다(build0.60초/suite0.03초). 상세 명령/경계는 `docs/quality-assurance/2026-10-05-m8-rust-remote-file-banners.md`입니다. 전체 mirror/persistence 소유자는 아직 미완료이며 이 해제만으로 그 계약 완료를 주장하지 않습니다.
