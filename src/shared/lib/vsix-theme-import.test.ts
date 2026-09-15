import { describe, expect, test } from 'bun:test'
import type { VsixThemeExtractionResult } from '@shared/api/bindings'
import { convertVscodeTheme } from '@shared/lib/theme-convert/convert'
import { buildVsixThemeCandidates } from '@shared/lib/vsix-theme-import'

const MINIMAL_THEME_JSON = JSON.stringify({
    colors: { 'editor.background': '#1e1e1e', 'editor.foreground': '#d4d4d4', foreground: '#d4d4d4' },
})

const EDITOR_BACKGROUND = '#1e1e1e'
const BODY_TEXT_COLOR = '#d4d4d4'

/**
 * A terminal surface deliberately painted in the theme's own body text color. The distinctness
 * repair derives a replacement by mixing the container toward `app.foreground`, so a container that
 * already is that foreground leaves the repair nothing to move toward and the violation survives
 * into `stateDistinctnessErrors` instead of being fixed away.
 */
const UNREPAIRABLE_TERMINAL_BACKGROUND = BODY_TEXT_COLOR

const terminalStateThemeJson = (selection: string, cursor: string) =>
    JSON.stringify({
        colors: {
            'editor.background': EDITOR_BACKGROUND,
            'editor.foreground': BODY_TEXT_COLOR,
            foreground: BODY_TEXT_COLOR,
            'terminal.background': UNREPAIRABLE_TERMINAL_BACKGROUND,
            'terminal.selectionBackground': selection,
            'terminalCursor.foreground': cursor,
        },
    })

const extensionResult = (overrides: Partial<VsixThemeExtractionResult> = {}): VsixThemeExtractionResult => ({
    extension: { name: 'my-cool-theme', displayName: 'My Cool Theme', publisher: 'someone', version: '1.0.0' },
    themes: [{ label: 'My Cool Theme', uiTheme: 'vs-dark', rawJson: MINIMAL_THEME_JSON, includeChain: [] }],
    ...overrides,
})

describe('buildVsixThemeCandidates', () => {
    test('단일 테마는 publisher-name 기반 slug id 를 받는다', () => {
        const [candidate] = buildVsixThemeCandidates(extensionResult(), [])
        expect(candidate.id).toBe('someone-my-cool-theme')
        expect(candidate.themeType).toBe('dark')
        expect(candidate.failureReason).toBeNull()
        expect(candidate.theme?.id).toBe('someone-my-cool-theme')
    })

    test('기존 테마 목록에 같은 id 가 있으면 idCollides 를 true 로 표시한다', () => {
        const [candidate] = buildVsixThemeCandidates(extensionResult(), ['someone-my-cool-theme'])
        expect(candidate.idCollides).toBe(true)
    })

    test('name 이 NLS 플레이스홀더로 남은 displayName 과 달라도 안정적인 id 를 만든다', () => {
        const result = extensionResult({
            extension: { name: 'github-theme', displayName: '%displayName%', publisher: 'github', version: '1.0.0' },
        })

        const [candidate] = buildVsixThemeCandidates(result, [])

        expect(candidate.id).toBe('github-github-theme')
    })

    test('테마가 여러 개면 각 테마 라벨을 id 에 덧붙여 구분한다', () => {
        const result = extensionResult({
            themes: [
                { label: 'My Cool Theme', uiTheme: 'vs-dark', rawJson: MINIMAL_THEME_JSON, includeChain: [] },
                { label: 'My Cool Theme Light', uiTheme: 'vs', rawJson: MINIMAL_THEME_JSON, includeChain: [] },
            ],
        })

        const candidates = buildVsixThemeCandidates(result, [])

        expect(candidates.map((c) => c.id)).toEqual(['someone-my-cool-theme-my-cool-theme', 'someone-my-cool-theme-my-cool-theme-light'])
        expect(candidates[1].themeType).toBe('light')
    })

    test('원본 JSON 파싱이 실패하면 failureReason 을 parse 로 표시하고 theme 는 null 이다', () => {
        const result = extensionResult({
            themes: [{ label: 'Broken', uiTheme: 'vs-dark', rawJson: '{ not json', includeChain: [] }],
        })

        const [candidate] = buildVsixThemeCandidates(result, [])

        expect(candidate.failureReason).toBe('parse')
        expect(candidate.theme).toBeNull()
    })

    test('tokenColors 변환 결과가 Theme 에 실린다', () => {
        const themeJson = JSON.stringify({
            colors: { 'editor.background': '#1e1e1e', 'editor.foreground': '#d4d4d4', foreground: '#d4d4d4' },
            tokenColors: [{ scope: 'comment', settings: { foreground: '#6a9955', fontStyle: 'italic' } }],
        })
        const result = extensionResult({ themes: [{ label: 'My Cool Theme', uiTheme: 'vs-dark', rawJson: themeJson, includeChain: [] }] })

        const [candidate] = buildVsixThemeCandidates(result, [])

        expect(candidate.theme?.tokenColors).toContainEqual({ scope: ['comment'], settings: { foreground: '#6a9955', fontStyle: 'italic' } })
    })

    test('수리 뒤에도 남은 상태색 구별성 위반을 경고 수에 반영한다', () => {
        const candidateFor = (selection: string, cursor: string) =>
            buildVsixThemeCandidates(
                extensionResult({
                    themes: [{ label: 'Terminal', uiTheme: 'vs-dark', rawJson: terminalStateThemeJson(selection, cursor), includeChain: [] }],
                }),
                [],
            )[0]
        const residualErrorsFor = (selection: string, cursor: string) =>
            convertVscodeTheme([JSON.parse(terminalStateThemeJson(selection, cursor))], 'dark').stateDistinctnessErrors.length

        const distinct = candidateFor(EDITOR_BACKGROUND, EDITOR_BACKGROUND)
        const collapsed = candidateFor(UNREPAIRABLE_TERMINAL_BACKGROUND, UNREPAIRABLE_TERMINAL_BACKGROUND)
        const addedErrors =
            residualErrorsFor(UNREPAIRABLE_TERMINAL_BACKGROUND, UNREPAIRABLE_TERMINAL_BACKGROUND) -
            residualErrorsFor(EDITOR_BACKGROUND, EDITOR_BACKGROUND)

        expect(addedErrors).toBeGreaterThan(0)
        expect(distinct.failureReason).toBeNull()
        expect(collapsed.failureReason).toBeNull()
        expect(collapsed.warningCount - distinct.warningCount).toBe(addedErrors)
    })

    test('includeChain 은 base 를 먼저 병합하고 테마 본문이 마지막에 덮어쓴다', () => {
        const base = JSON.stringify({ colors: { 'editor.background': '#000000', 'editor.foreground': '#d4d4d4', foreground: '#d4d4d4' } })
        const leaf = JSON.stringify({ colors: { 'editor.background': '#222222' }, include: './base.json' })
        const result = extensionResult({
            themes: [{ label: 'Derived', uiTheme: 'vs-dark', rawJson: leaf, includeChain: [{ path: 'base.json', rawJson: base }] }],
        })

        const [candidate] = buildVsixThemeCandidates(result, [])

        expect(candidate.theme?.colors?.['editor.background']).toBe('#222222')
    })
})
