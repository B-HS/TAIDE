import { describe, expect, test } from 'bun:test'
import { REMOTE_WINDOW_LABEL } from '@shared/lib/remote/tauri-internals-shim'
import { isRemoteMirrorLabel } from '@shared/lib/remote/runtime-environment'

describe('isRemoteMirrorLabel', () => {
    test('원격 미러 shim 이 보고하는 라벨이면 참이다', () => {
        expect(isRemoteMirrorLabel(REMOTE_WINDOW_LABEL)).toBe(true)
    })

    test('메인 창 라벨은 거짓이다', () => {
        expect(isRemoteMirrorLabel('main')).toBe(false)
    })

    test('보조 편집기 창 라벨은 거짓이다', () => {
        expect(isRemoteMirrorLabel('editor-1')).toBe(false)
    })

    test('빈 라벨은 거짓이다', () => {
        expect(isRemoteMirrorLabel('')).toBe(false)
    })

    test('정확히 일치해야 한다 — 대소문자·접두/접미 변형은 거짓이다', () => {
        expect(isRemoteMirrorLabel(REMOTE_WINDOW_LABEL.toUpperCase())).toBe(false)
        expect(isRemoteMirrorLabel(`${REMOTE_WINDOW_LABEL}-1`)).toBe(false)
        expect(isRemoteMirrorLabel(` ${REMOTE_WINDOW_LABEL}`)).toBe(false)
    })
})
