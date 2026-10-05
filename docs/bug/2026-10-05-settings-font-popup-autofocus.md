# Settings 폰트 popup의 브라우저 자동 포커스 누락

## 대상

`native/taide-native-ui/src/settings-code-view.rs`, `native/taide-remote-web/tests/settings-resources.rs`, `tools/m8-remote-rust-file-probe.ts`

## 관찰

실제 Chrome에서 Editor의 폰트 popup을 연 뒤 font-search 표시가 있어도 focused=[]였습니다. 기다리는 검사 10초가 실패했습니다. 공용 UI는 빈 목록/실제 두 목록·press/release 분리 입력 모두 포커스가 있어 플랫폼 동작과 구별했습니다. 검색창을 명시 클릭하면 입력되는 기존 성공은 자동 포커스 성공 근거가 아닙니다.

## 수정

기존 id_salt를 trigger 기반 안정 input ID로 바꾸고, focus_search는 UI가 보이는 실제 표시 pass에서만 소비합니다. 숨겨진 sizing pass에서 포커스 요청을 소모하지 않고 다음 repaint를 요청합니다. 크기 계산 pass와 DOM 기여를 각각 분리해 계측하지 않았으므로 하나의 원인만 단독 증명했다고 주장하지 않습니다.

## 결과

수정한 실제 Chrome 자동 포커스 경계만 GREEN: seq11·설정 쓰기0·focused=[font-search]·Drop/socket0/page error0/panic false/failures=[]입니다. 전체 글꼴 로딩/저장 성공을 다시 실행하지 않았습니다. 원자료/정확한 명령·최종/이전 source 구분은 `docs/quality-assurance/2026-10-05-m8-settings-font-preview.md`입니다. 원본 전체 픽셀/접근성 재현 완료를 의미하지 않습니다.
