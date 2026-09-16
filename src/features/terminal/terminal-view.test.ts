import { describe, expect, test } from 'bun:test'
import { fitIfMeasurable, shouldActivateTerminalLink, shouldTranslateShiftEnterToLineFeed } from '@features/terminal/terminal-view'

describe('shouldActivateTerminalLink — WebLinksAddon URL·path:line:col·OSC 8(linkHandler) 세 링크 종류가 공유하는 수식키 게이트', () => {
    test('metaKey 만 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: true, altKey: false, ctrlKey: false })).toBe(true)
    })

    test('altKey 만 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: false, altKey: true, ctrlKey: false })).toBe(true)
    })

    test('둘 다 눌렸으면 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: true, altKey: true, ctrlKey: false })).toBe(true)
    })

    test('둘 다 눌리지 않았으면 활성화하지 않는다', () => {
        expect(shouldActivateTerminalLink({ metaKey: false, altKey: false, ctrlKey: false })).toBe(false)
    })

    test('비 macOS 에서는 ctrlKey 만 눌려도 활성화한다', () => {
        expect(shouldActivateTerminalLink({ metaKey: false, altKey: false, ctrlKey: true }, false)).toBe(true)
    })

    test('비 macOS 에서는 metaKey(Win 키)만 눌리면 활성화하지 않는다', () => {
        expect(shouldActivateTerminalLink({ metaKey: true, altKey: false, ctrlKey: false }, false)).toBe(false)
    })
})

describe('shouldTranslateShiftEnterToLineFeed', () => {
    const shiftEnterKeydown = {
        type: 'keydown',
        key: 'Enter',
        shiftKey: true,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        isComposing: false,
    }

    test('조합 키 없는 Shift+Enter keydown 이면 변환한다', () => {
        expect(shouldTranslateShiftEnterToLineFeed(shiftEnterKeydown)).toBe(true)
    })

    test('keydown 이 아니면(keypress·keyup) 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, type: 'keypress' })).toBe(false)
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, type: 'keyup' })).toBe(false)
    })

    test('shift 없는 Enter 는 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, shiftKey: false })).toBe(false)
    })

    test('alt·ctrl·meta 가 섞이면 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, altKey: true })).toBe(false)
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, ctrlKey: true })).toBe(false)
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, metaKey: true })).toBe(false)
    })

    test('IME 조합 중이면 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, isComposing: true })).toBe(false)
    })

    test('Enter 이외의 키는 변환하지 않는다', () => {
        expect(shouldTranslateShiftEnterToLineFeed({ ...shiftEnterKeydown, key: 'a' })).toBe(false)
    })
})

describe('fitIfMeasurable — 0px 로 눌린 pane 이 PTY 를 애드온 하한(2×1)으로 리사이즈하는 것을 막는다', () => {
    const MEASURABLE_CONTAINER = { clientWidth: 800, clientHeight: 600 }
    const COLLAPSED_CONTAINER = { clientWidth: 800, clientHeight: 0 }
    const TERMINAL = { cols: 80, rows: 24 }

    const createFitAddonStub = (dimensions: { cols: number; rows: number } | undefined) => {
        const state = { fitCallCount: 0, proposeCallCount: 0 }
        const addon = {
            fit: () => {
                state.fitCallCount += 1
            },
            proposeDimensions: () => {
                state.proposeCallCount += 1
                return dimensions
            },
        }
        return { state, addon }
    }

    test('컨테이너 높이가 0 이면 제안을 보기도 전에 건너뛴다', () => {
        const { state, addon } = createFitAddonStub({ cols: 120, rows: 40 })
        fitIfMeasurable(TERMINAL, addon, COLLAPSED_CONTAINER)
        expect(state.fitCallCount).toBe(0)
        expect(state.proposeCallCount).toBe(0)
    })

    test('컨테이너 너비가 0 이면 건너뛴다', () => {
        const { state, addon } = createFitAddonStub({ cols: 120, rows: 40 })
        fitIfMeasurable(TERMINAL, addon, { clientWidth: 0, clientHeight: 600 })
        expect(state.fitCallCount).toBe(0)
    })

    test('애드온 하한 (2,1) 제안이면 건너뛴다 — 유한값이라 Number.isFinite 가드는 통과한다', () => {
        const { state, addon } = createFitAddonStub({ cols: 2, rows: 1 })
        fitIfMeasurable(TERMINAL, addon, MEASURABLE_CONTAINER)
        expect(state.fitCallCount).toBe(0)
    })

    test('한 축만 하한이어도 건너뛴다', () => {
        const rowsAtFloor = createFitAddonStub({ cols: 120, rows: 1 })
        fitIfMeasurable(TERMINAL, rowsAtFloor.addon, MEASURABLE_CONTAINER)
        expect(rowsAtFloor.state.fitCallCount).toBe(0)

        const colsAtFloor = createFitAddonStub({ cols: 2, rows: 40 })
        fitIfMeasurable(TERMINAL, colsAtFloor.addon, MEASURABLE_CONTAINER)
        expect(colsAtFloor.state.fitCallCount).toBe(0)
    })

    test('제안이 없거나(undefined) 비유한값이면 건너뛴다', () => {
        const missing = createFitAddonStub(undefined)
        fitIfMeasurable(TERMINAL, missing.addon, MEASURABLE_CONTAINER)
        expect(missing.state.fitCallCount).toBe(0)

        const notFinite = createFitAddonStub({ cols: Number.NaN, rows: Number.NaN })
        fitIfMeasurable(TERMINAL, notFinite.addon, MEASURABLE_CONTAINER)
        expect(notFinite.state.fitCallCount).toBe(0)
    })

    test('제안이 현재 cols·rows 와 같으면 건너뛴다', () => {
        const { state, addon } = createFitAddonStub({ cols: TERMINAL.cols, rows: TERMINAL.rows })
        fitIfMeasurable(TERMINAL, addon, MEASURABLE_CONTAINER)
        expect(state.fitCallCount).toBe(0)
    })

    test('측정 가능한 컨테이너의 정상 제안이면 fit 한다', () => {
        const { state, addon } = createFitAddonStub({ cols: 120, rows: 40 })
        fitIfMeasurable(TERMINAL, addon, MEASURABLE_CONTAINER)
        expect(state.fitCallCount).toBe(1)
    })
})
