import { describe, expect, test } from 'bun:test'
import { stripCodeFence } from '@shared/lib/inline-edit-fence'

describe('stripCodeFence', () => {
    test('언어 태그가 있는 펜스를 벗긴다', () => {
        expect(stripCodeFence('```typescript\nconst a = 1\n```')).toBe('const a = 1')
    })

    test('언어 태그가 없는 펜스를 벗긴다', () => {
        expect(stripCodeFence('```\nconst a = 1\n```')).toBe('const a = 1')
    })

    test('중첩된 펜스는 최외곽만 벗긴다', () => {
        const input = '```markdown\nExample:\n```js\nconsole.log(1)\n```\n```'
        expect(stripCodeFence(input)).toBe('Example:\n```js\nconsole.log(1)\n```')
    })

    test('펜스가 없으면 트림된 원문을 그대로 반환한다', () => {
        expect(stripCodeFence('  const a = 1  \n')).toBe('const a = 1')
    })

    test('닫는 펜스가 없는 미종결 블록은 원문을 그대로 반환한다', () => {
        const input = '```typescript\nconst a = 1'
        expect(stripCodeFence(input)).toBe(input)
    })

    test('빈 문자열은 빈 문자열을 반환한다', () => {
        expect(stripCodeFence('')).toBe('')
    })

    test('펜스 앞뒤 공백은 트림하고 내부 내용은 보존한다', () => {
        expect(stripCodeFence('\n\n```\n  indented line\n```\n\n')).toBe('  indented line')
    })

    test('여러 줄 코드 블록을 그대로 보존한다', () => {
        const input = '```typescript\nfunction add(a: number, b: number) {\n    return a + b\n}\n```'
        expect(stripCodeFence(input)).toBe('function add(a: number, b: number) {\n    return a + b\n}')
    })

    test('닫는 펜스 뒤에 붙은 설명문을 버린다', () => {
        const input = '```typescript\nconst y = 1\n```\n\nI renamed x to y.'
        expect(stripCodeFence(input)).toBe('const y = 1')
    })

    test('여는 펜스 앞에 붙은 설명문을 버린다', () => {
        const input = 'Sure! Here is the code:\n```ts\nconst y = 1\n```'
        expect(stripCodeFence(input)).toBe('const y = 1')
    })

    test('언어 태그 없이 한 줄로 열고 닫는 펜스도 벗긴다', () => {
        expect(stripCodeFence('```feat: add login```')).toBe('feat: add login')
    })

    test('CRLF 개행이 섞인 펜스도 벗기고 내부 CRLF를 LF로 정규화한다', () => {
        const input = '```ts\r\nconst a = 1\r\n```'
        expect(stripCodeFence(input)).toBe('const a = 1')
    })
})
