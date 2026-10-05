# Wry native preview 권한 경계

Upstream: Wry 0.55.1, crates.io checksum `186f9871daa55fd9c016578b810d149de58367113db7fb72b462d2323ce19514`, MIT/Apache-2.0. 설치된 crates.io source를 그대로 복사하고 기존 license·source·upstream provenance를 보존했습니다. 원본 Tauri/root registry dependency는 변경하지 않았습니다.

추가 feature `native-preview-deny-permissions`는 기본 비활성입니다. 격리 native app의 macOS preview dependency에서만 켭니다. macOS WKUIDelegate의 file upload panel completion을 nil로 끝내 선택을 거절하고, camera/microphone capture decision을 Deny로 반환합니다. upstream은 실제 NSOpenPanel을 열고 capture permission을 Grant로 반환했으며, JS disabled/CSP만으로 이 native callback 계약을 수정하지 않습니다.

기존 delegate 함수·원래 unsafe 영역을 재사용하며 새 unsafe 영역·IPC·script·private API·OS 설정 변경은 추가하지 않았습니다. feature가 꺼진 upstream 기본 동작은 유지합니다. 이는 두 delegate 경계의 코드 계약이며 실제 OS permission prompt·codec·WebKit 모든 보안/CPU/RSS·Windows/Linux gate 완료 증거는 아닙니다.
