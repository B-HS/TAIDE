import { afterEach, describe, expect, test } from 'bun:test'
import { buildImeDebugReport, clearImeDebug, isImeDebugEnabled, recordImeDebug, setImeDebugEnabled } from '@shared/lib/ime-debug'

const sampleEntry = { source: 'data' as const, inputType: 'insertText', data: '한글', rangeLength: null, composing: '', output: '한글' }

afterEach(() => {
    setImeDebugEnabled(false)
    clearImeDebug()
})

describe('imeDebug', () => {
    test('기본값은 비활성화이며 수집하지 않는다', () => {
        expect(isImeDebugEnabled()).toBe(false)

        recordImeDebug(sampleEntry)

        expect(buildImeDebugReport()).toBe('')
    })

    test('활성화하면 이후 기록이 리포트에 포함된다', () => {
        setImeDebugEnabled(true)

        recordImeDebug(sampleEntry)

        expect(buildImeDebugReport()).toContain('한글')
    })

    test('비활성화로 되돌리면 기존 기록도 함께 지워진다', () => {
        setImeDebugEnabled(true)
        recordImeDebug(sampleEntry)

        setImeDebugEnabled(false)

        expect(buildImeDebugReport()).toBe('')
    })
})
