import { describe, expect, test } from 'bun:test'
import { parseJsonc, stripJsonComments } from '@shared/lib/theme-convert/jsonc'

describe('stripJsonComments', () => {
    test('한 줄 주석을 제거한다', () => {
        expect(stripJsonComments('{ "a": 1 // comment\n}')).toBe('{ "a": 1 \n}')
    })

    test('블록 주석을 제거한다', () => {
        expect(stripJsonComments('{ /* comment */ "a": 1 }')).toBe('{  "a": 1 }')
    })

    test('문자열 내부의 // 나 /* 는 주석으로 취급하지 않는다', () => {
        expect(stripJsonComments('{ "a": "https://example.com" }')).toBe('{ "a": "https://example.com" }')
    })

    test('trailing comma 를 제거한다', () => {
        expect(stripJsonComments('{ "a": 1, }')).toBe('{ "a": 1 }')
        expect(stripJsonComments('[1, 2, ]')).toBe('[1, 2 ]')
    })
})

describe('parseJsonc', () => {
    test('주석과 trailing comma 가 섞인 VS Code 테마 스타일 JSON 을 파싱한다', () => {
        const source = `{
            // theme colors
            "colors": {
                "editor.background": "#1e1e1e", // dark bg
            },
        }`
        expect(parseJsonc(source)).toEqual({ colors: { 'editor.background': '#1e1e1e' } })
    })
})
