import { describe, expect, test } from 'bun:test'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'

describe('buildMonospaceFontStack', () => {
    test('폰트를 고르지 않으면 폴백 체인만 반환한다', () => {
        expect(buildMonospaceFontStack(null)).toBe('ui-monospace, SFMono-Regular, Menlo, "Apple SD Gothic Neo", monospace')
    })

    test('공백이 있는 폰트명은 따옴표로 감싸 앞에 붙인다', () => {
        expect(buildMonospaceFontStack('Fira Code')).toBe('"Fira Code", ui-monospace, SFMono-Regular, Menlo, "Apple SD Gothic Neo", monospace')
    })

    test('공백이 없는 폰트명은 따옴표 없이 앞에 붙인다', () => {
        expect(buildMonospaceFontStack('Menlo')).toBe('Menlo, ui-monospace, SFMono-Regular, Menlo, "Apple SD Gothic Neo", monospace')
    })

    test('폴백 체인은 한글 글리프 폰트를 generic monospace 앞에 포함한다', () => {
        const stack = buildMonospaceFontStack(null).split(', ')
        expect(stack.indexOf('"Apple SD Gothic Neo"')).toBeGreaterThan(stack.indexOf('Menlo'))
        expect(stack.indexOf('"Apple SD Gothic Neo"')).toBeLessThan(stack.indexOf('monospace'))
    })
})
