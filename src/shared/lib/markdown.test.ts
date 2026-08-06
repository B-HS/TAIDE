import { describe, expect, test } from 'bun:test'
import { parseMarkdownToHtml } from '@shared/lib/markdown'

describe('parseMarkdownToHtml', () => {
    test('제목을 heading 태그로 변환한다', () => {
        expect(parseMarkdownToHtml('# Hello')).toContain('<h1>Hello</h1>')
    })

    test('굵게 표시를 strong 태그로 변환한다', () => {
        expect(parseMarkdownToHtml('**bold**')).toContain('<strong>bold</strong>')
    })

    test('목록을 ul/li 태그로 변환한다', () => {
        const html = parseMarkdownToHtml('- one\n- two')
        expect(html).toContain('<ul>')
        expect(html).toContain('<li>one</li>')
        expect(html).toContain('<li>two</li>')
    })

    test('링크를 anchor 태그로 변환한다', () => {
        expect(parseMarkdownToHtml('[docs](https://example.com)')).toContain('<a href="https://example.com">docs</a>')
    })

    test('코드 블록을 pre/code 태그로 변환한다', () => {
        const html = parseMarkdownToHtml('```\nconst a = 1\n```')
        expect(html).toContain('<pre>')
        expect(html).toContain('<code>')
    })

    test('원본 HTML 태그는 이스케이프하지 않고 그대로 통과시킨다 (sanitize 는 별도 단계)', () => {
        expect(parseMarkdownToHtml('<script>alert(1)</script>')).toContain('<script>alert(1)</script>')
    })
})
