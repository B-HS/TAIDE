import { describe, expect, test } from 'bun:test'
import { BUILTIN_THEME_ID } from '@entities/theme/theme-tokens'
import { builtinThemeIdForType, resolveThemeIdAfterDelete } from '@entities/theme/theme-selection'

describe('resolveThemeIdAfterDelete — 활성 테마 삭제 시 builtin fallback (audit §4-B B5)', () => {
    test('활성 테마를 지우면 같은 타입의 builtin 으로 전환한다(재현: 재시작 후 테마 로드 실패)', () => {
        expect(resolveThemeIdAfterDelete({ deletedThemeId: 'my-dark', deletedThemeType: 'dark', activeThemeId: 'my-dark' })).toBe(
            BUILTIN_THEME_ID.DARK,
        )
        expect(resolveThemeIdAfterDelete({ deletedThemeId: 'my-light', deletedThemeType: 'light', activeThemeId: 'my-light' })).toBe(
            BUILTIN_THEME_ID.LIGHT,
        )
    })

    test('활성이 아닌 테마를 지우면 전환하지 않는다', () => {
        expect(resolveThemeIdAfterDelete({ deletedThemeId: 'other', deletedThemeType: 'dark', activeThemeId: 'my-dark' })).toBeNull()
    })

    test('활성 테마 아이디가 아직 없으면 전환하지 않는다', () => {
        expect(resolveThemeIdAfterDelete({ deletedThemeId: 'my-dark', deletedThemeType: 'dark', activeThemeId: null })).toBeNull()
        expect(resolveThemeIdAfterDelete({ deletedThemeId: 'my-dark', deletedThemeType: 'dark', activeThemeId: undefined })).toBeNull()
    })
})

describe('builtinThemeIdForType', () => {
    test('타입별 builtin 아이디를 돌려준다', () => {
        expect(builtinThemeIdForType('dark')).toBe(BUILTIN_THEME_ID.DARK)
        expect(builtinThemeIdForType('light')).toBe(BUILTIN_THEME_ID.LIGHT)
    })
})
