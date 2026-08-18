import type { APIRequestContext } from '@playwright/test'
import { HTTP_STATUS, LOGIN_ENDPOINT, LOGIN_PASSWORD_ENV_VAR } from './constants'

export type LoginOutcome = { kind: 'success' } | { kind: 'locked' } | { kind: 'mismatch' } | { kind: 'not-configured'; status: number }

const LOGIN_OUTCOME_GUIDANCE: Record<Exclude<LoginOutcome['kind'], 'success'>, string> = {
    locked: '로그인이 잠겨 있습니다(5회 실패 후 최대 60초). 새로 재시도하면 잠금이 연장될 수 있으니 ' + '잠시 기다린 뒤 다시 실행하세요.',
    mismatch: `${LOGIN_PASSWORD_ENV_VAR} 값이 앱에 설정된 REMOTE 비밀번호와 일치하지 않습니다. 재시도는 잠금 위험을 키우므로 값을 다시 확인한 뒤 재실행하세요.`,
    'not-configured':
        '로그인 응답이 303/401/429 가 아니었습니다. REMOTE 설정에서 비밀번호 설정과 ' + '"비밀번호만으로 접속 허용" 이 켜져 있는지 확인하세요.',
}

export const describeLoginFailure = (outcome: Exclude<LoginOutcome, { kind: 'success' }>) => LOGIN_OUTCOME_GUIDANCE[outcome.kind]

/**
 * Logs in against the app's remote-control server exactly once per harness run (§ 잠금 회피 규약).
 * Never retries internally — a mismatch or lockout is surfaced to the caller as a hard-stop outcome
 * instead, because the failure counter only resets on success and repeated automated retries would
 * risk tripping the 60-second lockout for the user's own future logins too.
 *
 * The password is read from `TAIDE_E2E_PASSWORD` only — never accepted as a parameter, written to a
 * file, or logged, so it cannot leak into Playwright's HTML report or trace files.
 */
export const loginOnce = async (request: APIRequestContext, baseURL: string): Promise<LoginOutcome> => {
    const password = process.env[LOGIN_PASSWORD_ENV_VAR]
    if (!password) {
        throw new Error(
            `${LOGIN_PASSWORD_ENV_VAR} 환경변수가 설정되지 않았습니다. docs/quality-assurance/2026-08-18-e2e-harness.md 의 준비 절차를 따르세요.`,
        )
    }

    const response = await request.post(`${baseURL}${LOGIN_ENDPOINT}`, {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded', Origin: baseURL },
        data: `password=${encodeURIComponent(password)}`,
        maxRedirects: 0,
    })

    const status = response.status()
    if (status === HTTP_STATUS.TOO_MANY_REQUESTS) return { kind: 'locked' }
    if (status === HTTP_STATUS.UNAUTHORIZED) return { kind: 'mismatch' }
    if (status !== HTTP_STATUS.SEE_OTHER) return { kind: 'not-configured', status }

    return { kind: 'success' }
}
