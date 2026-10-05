# native 이전 URI의 비활성 marker 소유권 잔존

## 대상·관찰

`native/taide-native-app/src/lsp.rs`와 `lsp-diagnostics-tests.rs`입니다. 두 provider가 같은 모델을 공유할 때 catalogue와 한 provider만 새 URI로 전환되고 다른 provider의 이전 mirror가 닫히면 DocumentId만 보존하는 marker 기록은 이전 URI 폐기에서 제거되지 않았습니다. 실제 두 합성 child 검사에서 marker4개/기대3개 RED(exit101, compile0.23초/suite0.38초)를 확인했습니다.

## 해결

Session marker 기록에 DocumentId와 실제 진단 URI를 함께 저장합니다. retarget에는 해당 ID의 옛 기록을 제거하고 DisposeModels에는 normalized URI가 같은 기록을 모든 session에서 정리합니다. 다른 문서/owner와 같은 ID의 새 URI는 보존합니다. 기존 활성 mirror URI filter와 raw 단일 출처·generation/version gate는 유지합니다.

## 검증·남은 범위

비활성 이전 URI1 PASS(compile11.41초/suite0.20초), 활성 이전 URI 신규1 PASS(7.38초/0.48초), app lib/bin/tests/mock strict16.34초입니다. 실제 Python spec discovery·합성 OS child·core rename/journal·생산 helper/notice·watch/marker Store를 사용했고 실제 Python 언어 도구나 NativeApplication GUI/full dispatcher 검증으로 주장하지 않습니다. 잘못된 Unicode filter의0tests는 PASS에서 제외했습니다. 명령과 범위는 `docs/quality-assurance/2026-10-04-m8-native-problems.md`가 정본입니다. 전체 root 합류/handshake·누적 heap·전체 App/GUI/OS/M8은 미완료이며 이전 성공은 재사용했습니다. manifest/lock·제품 TS·보호 bundle·사용자 앱·OS·Git은 변경하지 않았습니다.
