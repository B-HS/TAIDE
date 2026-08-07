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

    test('교체 범위가 주어지면 조합 추적 대신 그 길이만큼만 지운다', () => {
        expect(resolveImeInput(INSERT_REPLACEMENT_TEXT, '냐', '가나', 1)).toEqual({ output: '\x7f냐', composing: '가냐' })
    })

    test('음절 분리 커밋(간 → 가+나)에서 확정 글자를 지우지 않는다', () => {
        expect(resolveImeInput(INSERT_REPLACEMENT_TEXT, '가나', '간', 1)).toEqual({ output: '\x7f가나', composing: '가나' })
    })

    test('교체 범위 0 은 지우지 않고 덧붙인다', () => {
        expect(resolveImeInput(INSERT_REPLACEMENT_TEXT, 'ㄴ', '가', 0)).toEqual({ output: 'ㄴ', composing: '가ㄴ' })
    })

    test('교체 범위가 없으면 기존처럼 조합 전체를 지운다', () => {
        expect(resolveImeInput(INSERT_REPLACEMENT_TEXT, '호', 'ㅎ', null)).toEqual({ output: '\x7f호', composing: '호' })
    })

    test('가나 를 빠르게 입력하는 분리 시나리오가 화면상 가나 로 수렴한다', () => {
        const steps: Array<[string, string, number | null]> = [
            [INSERT_TEXT, 'ㄱ', null],
            [INSERT_REPLACEMENT_TEXT, '가', 1],
            [INSERT_REPLACEMENT_TEXT, '간', 1],
            [INSERT_REPLACEMENT_TEXT, '가나', 1],
        ]

        let composing = ''
        let screen = ''

        for (const [inputType, data, replaceLength] of steps) {
            const resolved = resolveImeInput(inputType, data, composing, replaceLength)
            if (!resolved) continue
            const emitted = inputType === INSERT_TEXT ? data : resolved.output
            for (const char of emitted) {
                screen = char === '\x7f' ? screen.slice(0, -1) : screen + char
            }
            composing = resolved.composing
        }

        expect(screen).toBe('가나')
    })
})
