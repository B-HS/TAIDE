#!/usr/bin/env bash
set -euo pipefail

INPUT="${1:-}"
if [ -z "$INPUT" ] || [ ! -f "$INPUT" ]; then
    echo "사용법: bash scripts/verify-signing-cert.sh <cert.p12 | base64.txt>"
    echo "  cert.p12   : .p12 원본 파일 — 임포트 검증 후 GitHub secret 용 한 줄 base64 를 클립보드에 복사"
    echo "  base64.txt : secret 에 넣(었)을 base64 값을 담은 텍스트 파일 — CI 와 동일한 방식으로 검증"
    echo "비밀번호는 CERT_PASSWORD 환경변수 또는 실행 중 프롬프트로 입력한다 (인자로 넘기지 말 것)."
    exit 2
fi

WORK="$(mktemp -d)"
KEYCHAIN="$WORK/probe.keychain-db"
cleanup() {
    security delete-keychain "$KEYCHAIN" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

PKCS12_DER_MAGIC="3082"
MAGIC="$(head -c 2 "$INPUT" | od -An -tx1 | tr -d ' \n')"
P12="$WORK/cert.p12"
STRICT_OK="n/a"

if [ "$MAGIC" = "$PKCS12_DER_MAGIC" ]; then
    MODE="p12"
    echo "[모드] .p12 원본 감지"
    cp "$INPUT" "$P12"
else
    MODE="base64"
    echo "[모드] base64 텍스트 감지"
    RAW="$(cat "$INPUT")"
    CLEAN="$(printf '%s' "$RAW" | tr -d ' \t\r\n')"
    TOTAL=$(printf '%s' "$RAW" | wc -c | tr -d ' ')
    NL=$(printf '%s' "$RAW" | tr -cd '\n' | wc -c | tr -d ' ')
    CR=$(printf '%s' "$RAW" | tr -cd '\r' | wc -c | tr -d ' ')
    SP=$(printf '%s' "$RAW" | tr -cd ' \t' | wc -c | tr -d ' ')
    BAD=$(printf '%s' "$CLEAN" | tr -d 'A-Za-z0-9+/=' | wc -c | tr -d ' ')
    CLEANLEN=$(printf '%s' "$CLEAN" | wc -c | tr -d ' ')
    echo "[진단] 길이 ${TOTAL}B / 개행 ${NL} / CR ${CR} / 공백·탭 ${SP} / base64 밖 문자 ${BAD}"
    echo "       (로컬 base64 는 관대해 CI 재현이 안 되므로 아래는 플랫폼 무관 엄격 형식 검사다)"
    if [ "$BAD" -gt 0 ]; then
        echo "[1/3 실패] base64 에 올 수 없는 문자가 ${BAD}개 섞여 있다 — CI 의"
        echo "          'error decoding base64 input stream' 의 확정 원인이다 (붙여넣기 시"
        echo "          따옴표·헤더 줄·다른 텍스트 혼입 의심). .p12 원본으로 이 스크립트를"
        echo "          다시 실행해 올바른 값을 새로 만들어라."
        exit 1
    fi
    if [ $((CLEANLEN % 4)) -ne 0 ]; then
        echo "[1/3 실패] base64 길이가 4의 배수가 아니다(${CLEANLEN}B) — 값이 잘렸다(truncation)."
        echo "          .p12 원본으로 이 스크립트를 다시 실행해 새 값을 만들어라."
        exit 1
    fi
    printf '%s' "$CLEAN" | base64 -d > "$P12" 2>/dev/null || { echo "[1/3 실패] 디코드 불가 — 값 손상"; exit 1; }
    ROUNDTRIP="$(base64 < "$P12" | tr -d '\n')"
    if [ "$ROUNDTRIP" != "$CLEAN" ]; then
        echo "[1/3 실패] 라운드트립 불일치 — 값 내부 손상. .p12 원본으로 새 값을 만들어라."
        exit 1
    fi
    if [ "$NL" -eq 0 ] && [ "$CR" -eq 0 ] && [ "$SP" -eq 0 ]; then
        STRICT_OK="yes"
        echo "[1/3 통과] 형식 완전 정상(한 줄·순수 base64·라운드트립 일치)"
    else
        STRICT_OK="no"
        echo "[1/3 경고] 값은 유효하나 개행/공백이 섞여 있다 — 엄격한 base64 구현은 거부할 수"
        echo "          있으니 정제본으로 재등록을 권장한다(스크립트 말미에 클립보드 복사)."
    fi
fi

if [ -n "${CERT_PASSWORD:-}" ]; then
    PW="$CERT_PASSWORD"
else
    read -rs -p ".p12 비밀번호(MACOS_CERTIFICATE_PASSWORD 값): " PW
    echo
fi

security create-keychain -p probe "$KEYCHAIN"
security unlock-keychain -p probe "$KEYCHAIN"
if ! security import "$P12" -k "$KEYCHAIN" -P "$PW" -T /usr/bin/codesign >/dev/null 2>&1; then
    echo "[2/3 실패] 임시 키체인 임포트 실패 — 비밀번호 불일치 또는 p12 손상"
    exit 1
fi
echo "[2/3 통과] 임시 키체인 임포트 성공 (CI 'Configure Apple code signing' 와 동일 절차)"

IDENTITY="$(security find-identity -p codesigning "$KEYCHAIN" | sed -n 's/.*"\(.*\)".*/\1/p' | head -1)"
if [ -z "$IDENTITY" ]; then
    echo "[3/3 실패] codesigning 아이덴티티 없음 — Developer ID Application 인증서인지 확인"
    exit 1
fi
echo "[3/3 통과] 아이덴티티: $IDENTITY"

echo
if [ "$MODE" = "p12" ]; then
    openssl base64 -A -in "$INPUT" | tr -d '\n' | pbcopy
    echo "[완료] GitHub secret 용 한 줄 base64 를 클립보드에 복사했다."
    echo "       레포 Settings > Secrets > Actions 의 MACOS_CERTIFICATE_P12 에 그대로 붙여넣어라."
elif [ "$STRICT_OK" = "yes" ]; then
    echo "[완료] 이 값은 CI 에서 그대로 동작한다 — secret 재등록 불필요일 수 있으나,"
    echo "       CI 가 실패했다면 등록 과정에서 개행이 섞였을 가능성이 크다. 아래로 재등록 권장:"
    printf '%s' "$RAW" | tr -d ' \t\r\n' | pbcopy
    echo "       (정제된 한 줄 값을 클립보드에 복사해 뒀다)"
else
    printf '%s' "$RAW" | tr -d ' \t\r\n' | pbcopy
    echo "[완료] 정제된 한 줄 base64 를 클립보드에 복사했다 — MACOS_CERTIFICATE_P12 를 이 값으로"
    echo "       재등록하면 CI 엄격 디코드를 통과한다."
fi
