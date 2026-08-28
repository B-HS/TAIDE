#!/usr/bin/env bash
set -euo pipefail

echo "공증 자격 3종(APPLE_ID / APPLE_APP_SPECIFIC_PASSWORD / APPLE_TEAM_ID)을 Apple 공증"
echo "서버에 실제 인증해 검증한다 (제출 없음 — notarytool history 호출)."
echo

if [ -n "${APPLE_ID:-}" ]; then
    NOTARY_ID="$APPLE_ID"
else
    read -r -p "APPLE_ID (Apple 계정 이메일): " NOTARY_ID
fi
if [ -n "${APPLE_TEAM_ID:-}" ]; then
    NOTARY_TEAM="$APPLE_TEAM_ID"
else
    read -r -p "APPLE_TEAM_ID (10자리, 예: SN98P5V7J4): " NOTARY_TEAM
fi
if [ -z "${NOTARY_PASSWORD:-}" ]; then
    read -r -p "APPLE_APP_SPECIFIC_PASSWORD (형식 xxxx-xxxx-xxxx-xxxx): " NOTARY_PASSWORD
    echo
fi

NOTARY_ID="$(printf '%s' "$NOTARY_ID" | tr -d ' \t\r\n')"
NOTARY_TEAM="$(printf '%s' "$NOTARY_TEAM" | tr -d ' \t\r\n')"
NOTARY_PASSWORD="$(printf '%s' "$NOTARY_PASSWORD" | tr -d ' \t\r\n')"
PW_LEN=$(printf '%s' "$NOTARY_PASSWORD" | wc -c | tr -d ' ')
APP_PASSWORD_LEN=19
if [ "$PW_LEN" -ne "$APP_PASSWORD_LEN" ]; then
    echo "[경고] 앱 암호 길이 ${PW_LEN}자(정상 ${APP_PASSWORD_LEN}자 = 16글자+하이픈 3) — 오입력 의심, 계속 진행"
fi

case "$NOTARY_ID" in
    *@*.*) ;;
    *) echo "[경고] APPLE_ID 가 이메일 형식이 아니다: 계정 이메일이어야 한다" ;;
esac
if ! printf '%s' "$NOTARY_TEAM" | grep -Eq '^[A-Z0-9]{10}$'; then
    echo "[경고] APPLE_TEAM_ID 형식 이상(영대문자·숫자 10자리가 정상) — 계속 진행"
fi
if ! printf '%s' "$NOTARY_PASSWORD" | grep -Eq '^[a-z]{4}-[a-z]{4}-[a-z]{4}-[a-z]{4}$'; then
    echo "[경고] 앱 암호 형식 이상(xxxx-xxxx-xxxx-xxxx 가 정상 — 계정 비밀번호를 넣은 것 아닌지"
    echo "       확인. 앱 암호는 account.apple.com > 로그인 및 보안 > 앱 암호에서 생성) — 계속 진행"
fi

echo
echo "[검증] Apple 공증 서버 인증 시도 중..."
set +e
OUTPUT="$(xcrun notarytool history --apple-id "$NOTARY_ID" --team-id "$NOTARY_TEAM" --password "$NOTARY_PASSWORD" 2>&1)"
STATUS=$?
set -e

if [ "$STATUS" -eq 0 ]; then
    COUNT="$(printf '%s' "$OUTPUT" | grep -c 'id:' || true)"
    echo "[통과] 3종 자격 전부 유효 — 공증 서버 인증 성공 (과거 제출 이력 ${COUNT}건)"
    echo "       이 값 그대로 GitHub secrets(APPLE_ID / APPLE_APP_SPECIFIC_PASSWORD /"
    echo "       APPLE_TEAM_ID)에 등록돼 있으면 CI 공증 스텝은 통과한다."
    exit 0
fi

echo "[실패] 공증 서버 인증 실패 — notarytool 출력 요지:"
printf '%s\n' "$OUTPUT" | tail -6
echo
echo "  원인 후보:"
echo "  - 앱 암호 폐기/만료 또는 오타 → account.apple.com 에서 새로 생성해 재등록"
echo "  - APPLE_ID 오타 또는 해당 계정이 팀($NOTARY_TEAM) 미소속"
echo "  - APPLE_TEAM_ID 불일치 → 개발자 계정 Membership 페이지에서 확인"
exit 1
