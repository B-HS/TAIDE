# 언어 pack 경로 검증 결정

## 사용자 결정

기존 locale 경로 이탈 재현에 대해 제시한 두 선택 중 사용자가 A, 즉 경로 검증 추가와 별도 보안 회귀를 승인했습니다. B인 현행 동작 유지·미완료 gate 기록은 선택하지 않았습니다. 적용 범위는 `load_locale`의 외부 ID, `locale_exists`/`resolve_language`의 선택, `list_locales`의 pack 노출 및 `save_locale`의 ID 규칙 일치입니다.

## 유지 조건

내장 locale 우선순위, 정상 사용자 pack의 저장·조회·목록, 공개 command의 인수/반환 형태는 유지합니다. 자기 전용 fixture만 사용하고 사용자 파일·실제 앱·원격 서버는 실행하지 않습니다. 파일 링크의 정적 차단과 동시 교체 경쟁, Windows 실기를 구분해 검증합니다. M6 전체 완료 전 일반 push는 하지 않습니다.
