import { describe, expect, test } from 'bun:test'
import {
    EDITOR_RULER_COLUMN_MAX,
    EDITOR_RULERS_MAX,
    formatEditorRulers,
    normalizeEditorRulersText,
    parseEditorRulers,
} from '@shared/lib/editor-rulers'

describe('parseEditorRulers', () => {
    test('콤마로 구분된 열 번호를 숫자 목록으로 읽는다', () => {
        expect(parseEditorRulers('80, 120')).toEqual([80, 120])
    })

    test('공백만 있거나 빈 입력은 빈 목록이다', () => {
        expect(parseEditorRulers('')).toEqual([])
        expect(parseEditorRulers('  ,  , ')).toEqual([])
    })

    test('정렬하고 중복을 제거한다 — 백엔드 sanitize_editor_rulers 와 같은 정규화', () => {
        expect(parseEditorRulers('120, 80, 80')).toEqual([80, 120])
    })

    test('0·음수·범위 초과·정수가 아닌 열은 버린다', () => {
        expect(parseEditorRulers(`0, -4, 4.5, ${EDITOR_RULER_COLUMN_MAX + 1}, 80`)).toEqual([80])
    })

    test('숫자가 아닌 조각은 버린다', () => {
        expect(parseEditorRulers('80, abc, 120')).toEqual([80, 120])
    })

    test('최대 개수를 넘기면 앞에서부터 잘린다', () => {
        const columns = Array.from({ length: EDITOR_RULERS_MAX + 5 }, (_, index) => index + 1)
        expect(parseEditorRulers(columns.join(','))).toHaveLength(EDITOR_RULERS_MAX)
    })
})

describe('formatEditorRulers', () => {
    test('저장된 목록을 입력 필드 표기로 되돌린다', () => {
        expect(formatEditorRulers([80, 120])).toBe('80, 120')
        expect(formatEditorRulers([])).toBe('')
    })

    test('parse 와 format 은 정규화된 값에서 왕복한다', () => {
        expect(parseEditorRulers(formatEditorRulers(parseEditorRulers('120, 80, 80')))).toEqual([80, 120])
    })
})

describe('normalizeEditorRulersText (d-53 검토 — 저장값과 화면 텍스트 일치)', () => {
    test('버려진 열은 화면 텍스트에서도 사라진다 — 저장값이 그대로여도 마찬가지', () => {
        expect(normalizeEditorRulersText(`80, ${EDITOR_RULER_COLUMN_MAX + 1}`)).toBe('80')
        expect(normalizeEditorRulersText('abc')).toBe('')
    })

    test('정렬·중복 제거 결과를 그대로 되쓴다', () => {
        expect(normalizeEditorRulersText('120, 80, 80')).toBe('80, 120')
    })

    test('이미 정규화된 텍스트는 그대로 둔다(멱등)', () => {
        expect(normalizeEditorRulersText('80, 120')).toBe('80, 120')
        expect(normalizeEditorRulersText(normalizeEditorRulersText('120, 80, 80'))).toBe('80, 120')
    })
})
