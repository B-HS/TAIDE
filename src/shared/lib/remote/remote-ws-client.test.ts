import { describe, expect, test } from 'bun:test'
import { isSessionExpiredClose } from '@shared/lib/remote/remote-ws-client'

describe('isSessionExpiredClose', () => {
    test('세션 만료 close 코드(4001, Rust REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED)면 true 를 반환한다', () => {
        expect(isSessionExpiredClose(4_001)).toBe(true)
    })

    test('정상 종료(1000)나 비정상 종료(1006) 등 다른 close 코드는 false 를 반환한다 — 재접속 루프를 그대로 탄다', () => {
        expect(isSessionExpiredClose(1_000)).toBe(false)
        expect(isSessionExpiredClose(1_006)).toBe(false)
        expect(isSessionExpiredClose(1_001)).toBe(false)
    })
})
