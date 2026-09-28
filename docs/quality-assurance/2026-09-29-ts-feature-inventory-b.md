# TS view 하위 feature 경계 inventory B

## 범위와 판정

Outline 2개, plugin 2개, preview 10개, problems 3개, search 5개의 비테스트 `.tsx` 경로를 소스의 상태·조작과 연결했습니다. 자동 근거는 기존 테스트의 실제 범위만 적었으며, 형식별 파일·OS 서비스·키보드·시각·접근성 실기는 별도입니다.

## Outline·plugin

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/outline/outline-panel.tsx` | 문서 symbol tree의 접힘·선택·가상 목록을 관리하고 화살표 키·선택으로 위치 이동을 요청합니다. | `src/features/outline/outline-rows.test.ts`는 행 계산 검사. 실제 LSP 갱신·tree 키보드·발화 필요. |
| `src/features/outline/outline-symbol-row.tsx` | symbol treeitem의 선택·포커스·하위 접기 버튼과 들여쓰기를 표시합니다. | 직접 컴포넌트 테스트 미확인. 깊은 중첩·포커스·VoiceOver 필요. |
| `src/features/plugin/plugin-list-body.tsx` | plugin 목록의 loading/empty 상태와 항목별 uninstall 요청을 렌더합니다. | 직접 컴포넌트 테스트 미확인. 실제 설치 목록·제거 확인·오류 상태 필요. |
| `src/features/plugin/vsix-import-grammars-section.tsx` | grammar import의 진행·오류·완료 항목과 import 버튼 비활성화를 표시합니다. | 직접 컴포넌트 테스트 미확인. 유효/손상 VSIX·중복 import·오류 안내 필요. |

## Preview

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/preview/audio-preview.tsx` | 파일명과 브라우저 audio controls를 표시합니다. | 직접 컴포넌트 테스트 미확인. 길이·seek·코덱·키보드 재생 필요. |
| `src/features/preview/html-preview.tsx` | 전달받은 HTML 문서를 제목 있는 `iframe srcDoc`과 `allow-same-origin` sandbox로 표시합니다. | 직접 컴포넌트 테스트 미확인. 외부 링크·스크립트 격리·내부 focus 필요. |
| `src/features/preview/hwp-preview.tsx` | HWP 변환의 loading/ready/error, 페이지 이동과 외부 열기 fallback을 관리합니다. | 직접 컴포넌트 테스트 미확인. 실제 HWP 파일·손상·긴 문서·페이지 focus 필요. |
| `src/features/preview/image-preview.tsx` | alt가 있는 이미지를 양축 스크롤 컨테이너에 크기 맞춰 표시합니다. | 직접 컴포넌트 테스트 미확인. 대형 이미지·투명도·alt·스크롤 필요. |
| `src/features/preview/pdf-preview.tsx` | PDF loading/ready/error, 페이지 이동·확대/축소·외부 열기 fallback을 관리합니다. | `src/features/preview/pdf-preview.test.tsx` 직접 검사. 실제 암호/손상 PDF·긴 문서·키보드 필요. |
| `src/features/preview/presentation-preview.tsx` | 발표 자료 outline 로딩·오류, 슬라이드 선택과 외부 열기 fallback을 관리합니다. | 직접 컴포넌트 테스트 미확인. 실제 PPTX·빈/손상 slide·목록 키보드 필요. |
| `src/features/preview/preview-status.tsx` | preview의 상태 메시지·아이콘·선택적 action 버튼을 그립니다. | 직접 컴포넌트 테스트 미확인. loading/error 안내 발화·action focus 필요. |
| `src/features/preview/spreadsheet-preview.tsx` | 여러 sheet의 tablist 선택과 표 표시·외부 열기를 관리합니다. | 직접 컴포넌트 테스트 미확인. 실제 XLSX·수식/병합·대형 sheet·tab 키보드 필요. |
| `src/features/preview/unsupported-preview.tsx` | 미지원 형식의 파일명·안내와 외부 열기 버튼을 표시합니다. | 직접 컴포넌트 테스트 미확인. 파일명 길이·외부 앱 실패·발화 필요. |
| `src/features/preview/video-preview.tsx` | 브라우저 video controls로 미디어를 재생합니다. | 직접 컴포넌트 테스트 미확인. 코덱·seek·전체화면·키보드 필요. |

## Problems·search

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/problems/problem-row.tsx` | 문제의 위치·심각도·메시지를 행으로 표시하고 클릭/키보드 이동을 전달합니다. | `src/features/problems/problem-list-rows.test.ts`는 행 변환 검사. 실제 LSP 진단·긴 메시지·키보드 필요. |
| `src/features/problems/problem-severity-filter.tsx` | 심각도별 건수와 활성 토글을 접근성 group으로 표시합니다. | 직접 컴포넌트 테스트 미확인. 오류/경고 혼합·키보드/발화 필요. |
| `src/features/problems/problems-panel.tsx` | 문제 목록의 필터·가상 행·닫기 및 대상 파일 이동을 조립합니다. | `src/features/problems/problem-list-rows.test.ts`는 행 변환 검사. 대량 진단·빈 상태·패널 focus 필요. |
| `src/features/search/search-exclude-glob-input.tsx` | 제외 glob 문자열의 입력 값을 상위 검색 상태로 전달합니다. | 직접 컴포넌트 테스트 미확인. 잘못된 glob·IME·입력 라벨 필요. |
| `src/features/search/search-history-dropdown.tsx` | 이전 검색어 목록을 dropdown으로 보여주고 선택을 전달합니다. | 직접 컴포넌트 테스트 미확인. 빈/긴 이력·키보드 focus 필요. |
| `src/features/search/search-match-row.tsx` | 일치 줄의 위치·강조 문맥을 클릭/키보드 활성화로 엽니다. | `src/features/search/search-result-rows.test.ts`는 행 변환 검사. 긴 줄·다중 일치·키보드 필요. |
| `src/features/search/search-option-toggles.tsx` | 대소문자·단어·정규식·gitignore 옵션의 현재 값과 토글을 표시합니다. | 직접 컴포넌트 테스트 미확인. 조합 변경·접근성 pressed 상태 필요. |
| `src/features/search/search-results-list.tsx` | 파일별 그룹 접기·선택과 일치 행을 가상 스크롤로 표시하고 결과 이동을 전달합니다. | `src/features/search/search-result-rows.test.ts`는 행 계산 검사. 대량 결과·재정렬·스크린리더 목록 필요. |

이 문서의 22개 경로를 더하면 여섯 문서의 중복 제외 직접 연결은 161/212개이며 남은 51개입니다. 실제 preview 형식별 렌더·사용자 동작·접근성의 통과를 뜻하지 않습니다.
