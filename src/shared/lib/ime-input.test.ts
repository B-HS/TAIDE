import { describe, expect, test } from 'bun:test'
import { INSERT_REPLACEMENT_TEXT, INSERT_TEXT, resolveImeInput } from '@shared/lib/ime-input'

describe('resolveImeInput', () => {
    test('새 음절 시작은 xterm 이 보내므로 출력 없이 조합 상태만 기록한다', () => {
        expect(resolveImeInput(INSERT_TEXT, 'ㅎ', '')).toEqual({ output: '', composing: 'ㅎ' })
    })

    test('교체 입력은 이전 조합 길이만큼 지우고 새 문자열을 보낸다', () => {
        expect(resolveImeInput(INSERT_REPLACEMENT_TEXT, '호', 'ㅎ')).toEqual({ output: '\x7f호', composing: '호' })
    })

    test('조합이 길어져도 이전 조합 전체를 지운다', () => {
        expect(resolveImeInput(INSERT_REPLACEMENT_TEXT, 'こんに', 'こん')).toEqual({ output: '\x7f\x7fこんに', composing: 'こんに' })
    })

    test('환 을 입력하는 전 과정이 화면상 환 으로 수렴한다', () => {
        const steps = [
            [INSERT_TEXT, 'ㅎ'],
            [INSERT_REPLACEMENT_TEXT, '호'],
            [INSERT_REPLACEMENT_TEXT, '화'],
            [INSERT_REPLACEMENT_TEXT, '환'],
        ]

        let composing = ''
        let screen = ''

        for (const [inputType, data] of steps) {
            const resolved = resolveImeInput(inputType, data, composing)
            if (!resolved) continue
            const emitted = inputType === INSERT_TEXT ? data : resolved.output
            for (const char of emitted) {
                screen = char === '\x7f' ? screen.slice(0, -1) : screen + char
            }
            composing = resolved.composing
        }

        expect(screen).toBe('환')
    })

    test('그 외 입력 타입은 조합 상태를 비운다', () => {
        expect(resolveImeInput('deleteContentBackward', '', '환')).toEqual({ output: '', composing: '' })
    })
})
