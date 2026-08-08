import { describe, expect, test } from 'bun:test'
import { INSERT_REPLACEMENT_TEXT, INSERT_TEXT, createInsertTextDeduper, resolveImeInput } from '@shared/lib/ime-input'

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

    test('xterm 이 먼저 보낸 insertText 는 중복 전송하지 않는다', () => {
        const deduper = createInsertTextDeduper()
        expect(deduper.onXtermData('ㅇ', 100)).toBe('forward')
        expect(deduper.onInsertText('ㅇ', 101)).toBe('already-sent')
    })

    test('xterm 이 누락한 insertText 는 자체 전송한다', () => {
        const deduper = createInsertTextDeduper()
        expect(deduper.onInsertText('니', 100)).toBe('self-send')
    })

    test('자체 전송 후 늦게 도착한 xterm 중복은 1회만 버린다', () => {
        const deduper = createInsertTextDeduper()
        expect(deduper.onInsertText('니', 100)).toBe('self-send')
        expect(deduper.onXtermData('니', 110)).toBe('drop')
        expect(deduper.onXtermData('니', 120)).toBe('forward')
    })

    test('같은 글자 연타에서 두 번째가 누락돼도 자체 전송한다', () => {
        const deduper = createInsertTextDeduper()
        expect(deduper.onXtermData('ㅇ', 100)).toBe('forward')
        expect(deduper.onInsertText('ㅇ', 101)).toBe('already-sent')
        expect(deduper.onInsertText('ㅇ', 130)).toBe('self-send')
    })

    test('다른 데이터가 지나가면 억제 상태가 풀린다', () => {
        const deduper = createInsertTextDeduper()
        expect(deduper.onInsertText(' ', 100)).toBe('self-send')
        expect(deduper.onXtermData('\r', 110)).toBe('forward')
        expect(deduper.onXtermData(' ', 120)).toBe('forward')
    })

    test('실기 로그 재현 — xterm 이 니 를 누락한 아니 입력이 화면상 아니 로 수렴한다', () => {
        const deduper = createInsertTextDeduper()
        let composing = ''
        let screen = ''
        const send = (text: string) => {
            for (const char of text) {
                screen = char === '\x7f' ? screen.slice(0, -1) : screen + char
            }
        }
        const xtermData = (data: string, at: number) => {
            if (deduper.onXtermData(data, at) === 'forward') send(data)
        }
        const imeInput = (inputType: string, data: string, at: number) => {
            const resolved = resolveImeInput(inputType, data, composing, null)
            if (inputType === INSERT_TEXT && deduper.onInsertText(data, at) === 'self-send') send(data)
            if (!resolved) return
            composing = resolved.composing
            if (resolved.output) send(resolved.output)
        }

        xtermData('ㅇ', 100)
        imeInput(INSERT_TEXT, 'ㅇ', 101)
        imeInput(INSERT_REPLACEMENT_TEXT, '아', 150)
        imeInput(INSERT_REPLACEMENT_TEXT, '안', 200)
        imeInput(INSERT_REPLACEMENT_TEXT, '아', 250)
        imeInput(INSERT_TEXT, '니', 252)
        imeInput(INSERT_REPLACEMENT_TEXT, '니', 300)

        expect(screen).toBe('아니')
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
