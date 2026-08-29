import { describe, expect, test } from 'bun:test'
import {
    buildThemeFromDraft,
    countChangedTokens,
    createThemeDraft,
    diffThemeValues,
    generateUniqueThemeId,
    isColorTokenChanged,
    isSyntaxTokenChanged,
    isThemeDraftValid,
    resetColorToken,
    resolveThemeDraftMetadata,
    setColorToken,
    setSyntaxToken,
    slugifyThemeId,
    type ThemeValues,
} from '@shared/lib/theme-draft'

const baseValues: ThemeValues = {
    colors: { 'app.background': '#1e1e2e', 'app.foreground': '#cdd6f4' },
    syntax: { keyword: { fg: '#cba6f7', bold: false, italic: false } },
    terminal: { black: '#45475a' },
}

describe('slugifyThemeId', () => {
    test('공백과 특수문자를 하이픈으로 바꾼다', () => {
        expect(slugifyThemeId('My Dark Theme!')).toBe('my-dark-theme')
    })

    test('앞뒤 하이픈을 제거한다', () => {
        expect(slugifyThemeId('  ***Neon***  ')).toBe('neon')
    })
})

describe('generateUniqueThemeId', () => {
    test('충돌이 없으면 슬러그를 그대로 사용한다', () => {
        expect(generateUniqueThemeId('Ocean', ['taide-dark'])).toBe('ocean')
    })

    test('충돌하면 접미사를 붙인다', () => {
        const id = generateUniqueThemeId('Ocean', ['ocean'])
        expect(id).not.toBe('ocean')
        expect(id.startsWith('ocean-')).toBe(true)
    })
})

describe('createThemeDraft', () => {
    test('initial 이 없으면 base 를 복제해서 시작한다', () => {
        const draft = createThemeDraft({ id: 'custom', name: 'Custom', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        expect(draft.current).toEqual(baseValues)
        expect(draft.current).not.toBe(baseValues)
    })
})

describe('setColorToken / isColorTokenChanged', () => {
    test('토큰을 바꾸면 변경으로 감지된다', () => {
        const draft = createThemeDraft({ id: 'custom', name: 'Custom', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const changed = setColorToken(draft, 'app.background', '#000000')
        expect(isColorTokenChanged(changed, 'app.background')).toBe(true)
        expect(isColorTokenChanged(changed, 'app.foreground')).toBe(false)
    })

    test('base 와 같은 값으로 되돌리면 변경이 아니다', () => {
        const draft = createThemeDraft({ id: 'custom', name: 'Custom', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const changed = setColorToken(draft, 'app.background', '#000000')
        const reset = resetColorToken(changed, 'app.background')
        expect(isColorTokenChanged(reset, 'app.background')).toBe(false)
    })
})

describe('setSyntaxToken / isSyntaxTokenChanged', () => {
    test('fg 만 바꿔도 변경으로 감지된다', () => {
        const draft = createThemeDraft({ id: 'custom', name: 'Custom', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const changed = setSyntaxToken(draft, 'keyword', { fg: '#ffffff' })
        expect(isSyntaxTokenChanged(changed, 'keyword')).toBe(true)
        expect(changed.current.syntax.keyword).toEqual({ fg: '#ffffff', bold: false, italic: false })
    })

    test('bold 만 토글해도 변경으로 감지된다', () => {
        const draft = createThemeDraft({ id: 'custom', name: 'Custom', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const changed = setSyntaxToken(draft, 'keyword', { bold: true })
        expect(isSyntaxTokenChanged(changed, 'keyword')).toBe(true)
    })
})

describe('diffThemeValues', () => {
    test('바뀐 토큰만 남긴다', () => {
        const draft = createThemeDraft({ id: 'custom', name: 'Custom', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const changed = setColorToken(draft, 'app.background', '#000000')
        const diff = diffThemeValues(changed.base, changed.current)
        expect(diff.colors).toEqual({ 'app.background': '#000000' })
        expect(diff.syntax).toEqual({})
        expect(diff.terminal).toEqual({})
    })
})

describe('countChangedTokens', () => {
    test('변경된 토큰 개수를 그룹 합산으로 센다', () => {
        const draft = createThemeDraft({ id: 'custom', name: 'Custom', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const changed = setSyntaxToken(setColorToken(draft, 'app.background', '#000000'), 'keyword', { bold: true })
        expect(countChangedTokens(changed)).toBe(2)
    })
})

describe('buildThemeFromDraft', () => {
    test('바뀐 토큰만 담은 Theme 을 만들고 extends 로 base 를 참조한다', () => {
        const draft = createThemeDraft({ id: 'ocean', name: 'Ocean', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const changed = setColorToken(draft, 'app.background', '#001122')
        const theme = buildThemeFromDraft(changed)

        expect(theme.id).toBe('ocean')
        expect(theme.extends).toBe('taide-dark')
        expect(theme.colors).toEqual({ 'app.background': '#001122' })
        expect(theme.syntax).toEqual({})
        expect(theme.terminal).toEqual({})
    })

    test('아무것도 바꾸지 않으면 빈 diff 를 만든다', () => {
        const draft = createThemeDraft({ id: 'ocean', name: 'Ocean', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const theme = buildThemeFromDraft(draft)
        expect(theme.colors).toEqual({})
        expect(theme.syntax).toEqual({})
        expect(theme.terminal).toEqual({})
    })
})

describe('isThemeDraftValid', () => {
    test('이름이 비어있으면 무효하다', () => {
        const draft = createThemeDraft({ id: 'ocean', name: '  ', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        expect(isThemeDraftValid(draft)).toBe(false)
    })

    test('색상 값이 유효하지 않으면 무효하다', () => {
        const draft = createThemeDraft({ id: 'ocean', name: 'Ocean', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        const invalid = setColorToken(draft, 'app.background', 'not-a-color')
        expect(isThemeDraftValid(invalid)).toBe(false)
    })

    test('모두 유효하면 참이다', () => {
        const draft = createThemeDraft({ id: 'ocean', name: 'Ocean', themeType: 'dark', extendsId: 'taide-dark', base: baseValues })
        expect(isThemeDraftValid(draft)).toBe(true)
    })
})

const importedThemeTokenColors = [{ scope: ['keyword.control'], settings: { foreground: '#ff0000', fontStyle: 'bold' } }]

describe('resolveThemeDraftMetadata / buildThemeFromDraft — vsix 임포트 메타데이터 보존 (audit §4-B B6)', () => {
    test('base 에 없는 tokenColors·author·source 는 저장 왕복에서 보존된다(재현: 무변경 저장만으로 영구 소실)', () => {
        const metadata = resolveThemeDraftMetadata(
            { tokenColors: importedThemeTokenColors, author: 'publisher', license: null, source: 'Some Theme 1.2.3' },
            { tokenColors: null },
        )
        const draft = createThemeDraft({
            id: 'imported',
            name: 'Imported',
            themeType: 'dark',
            extendsId: 'taide-dark',
            base: baseValues,
            metadata,
        })
        const theme = buildThemeFromDraft(draft)

        expect(theme.tokenColors).toEqual(importedThemeTokenColors)
        expect(theme.author).toBe('publisher')
        expect(theme.source).toBe('Some Theme 1.2.3')
    })

    test('base 와 동일한 tokenColors 는 싣지 않는다(번들 테마 복제는 extends 로 계속 상속)', () => {
        const metadata = resolveThemeDraftMetadata(
            { tokenColors: importedThemeTokenColors, author: null, license: null, source: null },
            { tokenColors: importedThemeTokenColors },
        )
        expect(metadata.tokenColors).toBeNull()
        expect(
            buildThemeFromDraft(createThemeDraft({ id: 'copy', name: 'Copy', themeType: 'dark', extendsId: 'bundled', base: baseValues, metadata }))
                .tokenColors,
        ).toBeNull()
    })

    test('메타데이터를 주지 않은 드래프트는 전부 null 로 저장된다(기존 커스텀 테마 동작 불변)', () => {
        const theme = buildThemeFromDraft(
            createThemeDraft({ id: 'plain', name: 'Plain', themeType: 'dark', extendsId: 'taide-dark', base: baseValues }),
        )
        expect(theme.tokenColors).toBeNull()
        expect(theme.author).toBeNull()
        expect(theme.license).toBeNull()
        expect(theme.source).toBeNull()
    })
})
