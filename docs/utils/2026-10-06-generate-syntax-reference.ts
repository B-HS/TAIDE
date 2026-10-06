import { readdir } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { createHighlighterCore, type HighlighterCore } from '@shikijs/core'
import { createJavaScriptRegexEngine } from '@shikijs/engine-javascript'
import { shikiToMonaco } from '@shikijs/monaco'
import { INITIAL, type StateStack } from '@shikijs/vscode-textmate'
import type { ResolvedTheme, SyntaxStyle, TokenColorRule } from '@shared/api/bindings'
import { TAIDE_MONACO_THEME_NAME } from '@shared/lib/monaco/theme'
import { buildShikiTheme } from '@shared/lib/shiki/build-shiki-theme'
import { isTaideLanguageId, loadTaideGrammar, loadTaideGrammars, TAIDE_CORE_LANGUAGE_IDS, TAIDE_LANGUAGE_IDS, type TaideLanguageId } from '@shared/lib/shiki/lang-map'

const REPOSITORY_ROOT = resolve(import.meta.dir, '../..')
const BUNDLED_THEMES_DIRECTORY = join(REPOSITORY_ROOT, 'crates/taide-theme/resources/themes')
const FIXTURES_DIRECTORY = join(REPOSITORY_ROOT, 'native/taide-native-syntax/tests/fixtures')
const SAMPLES_DIRECTORY = join(FIXTURES_DIRECTORY, 'samples')
const REFERENCE_DIRECTORY = join(FIXTURES_DIRECTORY, 'reference')
const TOKEN_THEMES_DIRECTORY = join(FIXTURES_DIRECTORY, 'token-themes')
const SYNTHETIC_THEMES_DIRECTORY = join(TOKEN_THEMES_DIRECTORY, 'synthetic')
const TOKEN_THEME_REFERENCE_DIRECTORY = join(TOKEN_THEMES_DIRECTORY, 'reference')
const TOKEN_THEMES_FLAG = '--token-themes'
const THEME_FILE_EXTENSION = '.json'
const BUNDLED_THEME_GROUP = 'bundled'
const SYNTHETIC_THEME_GROUP = 'synthetic'
const MONACO_TOKENIZATION_MODULE = 'monaco-editor/editor/common/languages/supports/tokenization.js'
const REFERENCE_THEME_IDS = ['one-dark-pro', 'github-light']
const TOKENIZE_MAX_LINE_LENGTH = 20000
const TOKENIZE_TIME_LIMIT_MS = 500
const NO_TIME_LIMIT = 0
const PROBE_LANGUAGE_ID = 'json'
const PROBE_LINE = 'probe'
const PALETTE_REFERENCE_PREFIX = '$'
const MONACO_LANGUAGE_ID = 1
const BALANCED_BRACKETS_MASK = 1024
const FONT_STYLE_OFFSET = 11
const FONT_STYLE_VARIANTS = 16
const FOREGROUND_OFFSET = 15
const HEX_RADIX = 16
const HEX_CHANNEL_DIGITS = 2
const JSON_INDENT = 4

type LongLine = { prefix: string; suffix: string; tail: string }

type Sample = ReturnType<typeof readSample>

type MonacoThemeRule = { token: string; foreground?: string; background?: string; fontStyle?: string }

type MonacoThemeData = { colors: Record<string, string>; rules: MonacoThemeRule[] }

type MonacoColor = { rgba: { r: number; g: number; b: number } }

type MonacoTokenTheme = { match: (languageId: number, scope: string) => number; getColorMap: () => (MonacoColor | undefined)[] }

type MonacoTokenizationModule = { TokenTheme: { createFromRawTokenTheme: (rules: MonacoThemeRule[], customTokenColors: string[]) => MonacoTokenTheme } }

type ProviderState = { ruleStack: StateStack }

type ProviderToken = { startIndex: number; scopes: string }

type TokensProvider = {
    getInitialState: () => ProviderState
    tokenize: (line: string, state: ProviderState) => { endState: ProviderState; tokens: ProviderToken[] }
}


const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)

const isStringRecord = (value: unknown): value is Record<string, string> => isRecord(value) && Object.values(value).every((item) => typeof item === 'string')

const isOptionalString = (value: unknown): value is string | null | undefined => value === undefined || value === null || typeof value === 'string'

const isSyntaxStyle = (value: unknown): value is SyntaxStyle => isRecord(value) && typeof value.fg === 'string'

const isSyntaxRecord = (value: unknown): value is Record<string, SyntaxStyle> => isRecord(value) && Object.values(value).every(isSyntaxStyle)

const isTokenColorRule = (value: unknown): value is TokenColorRule =>
    isRecord(value) &&
    Array.isArray(value.scope) &&
    value.scope.every((scope) => typeof scope === 'string') &&
    isRecord(value.settings) &&
    isOptionalString(value.settings.foreground) &&
    isOptionalString(value.settings.background) &&
    isOptionalString(value.settings.fontStyle)

const isLongLine = (value: unknown): value is LongLine =>
    isRecord(value) && typeof value.prefix === 'string' && typeof value.suffix === 'string' && typeof value.tail === 'string'

const isMonacoTokenizationModule = (value: unknown): value is MonacoTokenizationModule =>
    isRecord(value) &&
    typeof value.TokenTheme === 'function' &&
    'createFromRawTokenTheme' in value.TokenTheme &&
    typeof value.TokenTheme.createFromRawTokenTheme === 'function'

const readSample = (value: unknown) => {
    if (!isRecord(value) || typeof value.id !== 'string' || typeof value.file !== 'string' || !isLongLine(value.longLine)) {
        throw new Error(`invalid sample entry: ${JSON.stringify(value)}`)
    }
    const additionalCandidates: unknown[] = Array.isArray(value.additionalLanguageIds) ? value.additionalLanguageIds : []
    const additionalLanguageIds = additionalCandidates.filter((id): id is TaideLanguageId => typeof id === 'string' && isTaideLanguageId(id))
    if (typeof value.languageId !== 'string' || !isTaideLanguageId(value.languageId) || additionalLanguageIds.length !== additionalCandidates.length) {
        throw new Error(`${value.id}: languageId and additionalLanguageIds must be TAIDE language ids`)
    }
    return { id: value.id, languageId: value.languageId, file: value.file, additionalLanguageIds, longLine: value.longLine }
}

const readSampleManifest = async () => {
    const manifest: unknown = await Bun.file(join(SAMPLES_DIRECTORY, 'manifest.json')).json()
    if (!isRecord(manifest) || typeof manifest.longLineFill !== 'string' || manifest.longLineFill.length !== 1 || !Array.isArray(manifest.samples)) {
        throw new Error('sample manifest needs a one-code-unit longLineFill and a samples array')
    }
    return { longLineFill: manifest.longLineFill, samples: manifest.samples.map(readSample) }
}

const readResolvedBundledTheme = async (themeId: string) => {
    const theme: unknown = await Bun.file(join(BUNDLED_THEMES_DIRECTORY, `${themeId}.json`)).json()
    if (
        !isRecord(theme) ||
        typeof theme.id !== 'string' ||
        typeof theme.name !== 'string' ||
        (theme.type !== 'dark' && theme.type !== 'light') ||
        !isStringRecord(theme.palette) ||
        !isStringRecord(theme.colors) ||
        !isSyntaxRecord(theme.syntax) ||
        !isStringRecord(theme.terminal) ||
        !Array.isArray(theme.tokenColors) ||
        !theme.tokenColors.every(isTokenColorRule) ||
        !isOptionalString(theme.author) ||
        !isOptionalString(theme.license) ||
        !isOptionalString(theme.source)
    ) {
        throw new Error(`${themeId}: not a bundled theme with tokenColors`)
    }
    if (theme.extends !== undefined && theme.extends !== null) throw new Error(`${themeId}: reference themes must not extend another theme`)
    const { palette } = theme
    const resolveValue = (value: string) => (value.startsWith(PALETTE_REFERENCE_PREFIX) ? (palette[value.slice(PALETTE_REFERENCE_PREFIX.length)] ?? value) : value)
    const resolveValues = (values: Record<string, string>) => Object.fromEntries(Object.entries(values).map(([key, value]) => [key, resolveValue(value)]))
    return {
        id: theme.id,
        name: theme.name,
        type: theme.type,
        colors: resolveValues(theme.colors),
        syntax: Object.fromEntries(Object.entries(theme.syntax).map(([key, style]) => [key, { ...style, fg: resolveValue(style.fg) }])),
        terminal: resolveValues(theme.terminal),
        tokenColors: theme.tokenColors,
        syntaxOverrides: [],
        warnings: [],
        author: theme.author ?? null,
        license: theme.license ?? null,
        source: theme.source ?? null,
    } satisfies ResolvedTheme
}

const readSyntheticResolvedTheme = async (themeId: string) => {
    const theme: unknown = await Bun.file(join(SYNTHETIC_THEMES_DIRECTORY, `${themeId}${THEME_FILE_EXTENSION}`)).json()
    if (
        !isRecord(theme) ||
        theme.id !== themeId ||
        typeof theme.name !== 'string' ||
        (theme.type !== 'dark' && theme.type !== 'light') ||
        !isStringRecord(theme.colors) ||
        !isSyntaxRecord(theme.syntax) ||
        !isStringRecord(theme.terminal) ||
        !(theme.tokenColors === undefined || (Array.isArray(theme.tokenColors) && theme.tokenColors.every(isTokenColorRule))) ||
        !(theme.syntaxOverrides === undefined || (Array.isArray(theme.syntaxOverrides) && theme.syntaxOverrides.every((token) => typeof token === 'string')))
    ) {
        throw new Error(`${themeId}: not a synthetic resolved theme`)
    }
    return {
        id: theme.id,
        name: theme.name,
        type: theme.type,
        colors: theme.colors,
        syntax: theme.syntax,
        terminal: theme.terminal,
        ...(theme.tokenColors === undefined ? {} : { tokenColors: theme.tokenColors }),
        syntaxOverrides: theme.syntaxOverrides ?? [],
        warnings: [],
        author: null,
        license: null,
        source: null,
    } satisfies ResolvedTheme
}

const listThemeIds = async (directory: string) =>
    (await readdir(directory))
        .filter((name) => name.endsWith(THEME_FILE_EXTENSION))
        .map((name) => name.slice(0, -THEME_FILE_EXTENSION.length))
        .toSorted()

const createMonacoStub = () => {
    const definedThemes = new Map<string, MonacoThemeData>()
    const tokensProviders = new Map<string, TokensProvider>()
    const namespace = {
        editor: {
            defineTheme: (themeId: string, theme: MonacoThemeData) => void definedThemes.set(themeId, theme),
            setTheme: (themeId: string) => void themeId,
            create: () => {
                throw new Error('monaco.editor.create is unavailable while generating reference data')
            },
        },
        languages: {
            getLanguages: () => TAIDE_LANGUAGE_IDS.map((id) => ({ id })),
            setTokensProvider: (languageId: string, provider: TokensProvider) => {
                tokensProviders.set(languageId, provider)
                return { dispose: () => undefined }
            },
        },
    }
    return { namespace, definedThemes, tokensProviders }
}

const createTokenTheme = (tokenization: MonacoTokenizationModule, theme: MonacoThemeData) => {
    const editorForeground = theme.colors['editor.foreground']
    const editorBackground = theme.colors['editor.background']
    const defaultRule = {
        token: '',
        ...(editorForeground ? { foreground: editorForeground } : {}),
        ...(editorBackground ? { background: editorBackground } : {}),
    }
    return tokenization.TokenTheme.createFromRawTokenTheme(editorForeground || editorBackground ? [defaultRule, ...theme.rules] : [...theme.rules], [])
}

const matchMonacoMetadata = (tokenTheme: MonacoTokenTheme, scope: string) => (tokenTheme.match(MONACO_LANGUAGE_ID, scope) | BALANCED_BRACKETS_MASK) >>> 0

const toMonacoBinaryTokens = (tokenTheme: MonacoTokenTheme, tokens: ProviderToken[]) => {
    const result: number[] = []
    let previousStartIndex = 0
    tokens.forEach((token, index) => {
        const metadata = matchMonacoMetadata(tokenTheme, token.scopes)
        if (result.length > 0 && result.at(-1) === metadata) return
        const startIndex = index === 0 ? 0 : Math.max(token.startIndex, previousStartIndex)
        result.push(startIndex, metadata)
        previousStartIndex = startIndex
    })
    return result
}

const toHexChannel = (channel: number) => channel.toString(HEX_RADIX).padStart(HEX_CHANNEL_DIGITS, '0').toUpperCase()

const toHexColor = (color: MonacoColor | undefined) => (color ? `#${toHexChannel(color.rgba.r)}${toHexChannel(color.rgba.g)}${toHexChannel(color.rgba.b)}` : null)

const createThemedHighlighter = async (resolved: ResolvedTheme, requestedLanguageIds: TaideLanguageId[]) => {
    const coreLanguageIds: readonly TaideLanguageId[] = TAIDE_CORE_LANGUAGE_IDS
    const highlighter = await createHighlighterCore({
        themes: [buildShikiTheme(resolved)],
        langs: await loadTaideGrammars(coreLanguageIds),
        engine: createJavaScriptRegexEngine(),
    })
    for (const languageId of requestedLanguageIds.filter((id) => !coreLanguageIds.includes(id))) {
        await highlighter.loadLanguage(...(await loadTaideGrammar(languageId)))
    }
    return highlighter
}

const attachTokensProviders = async (tokenization: MonacoTokenizationModule, highlighter: HighlighterCore, resolved: ResolvedTheme) => {
    await highlighter.loadTheme(buildShikiTheme(resolved))
    const monaco = createMonacoStub()
    shikiToMonaco(highlighter, monaco.namespace as unknown as Parameters<typeof shikiToMonaco>[1], { tokenizeTimeLimit: NO_TIME_LIMIT })
    const monacoTheme = monaco.definedThemes.get(TAIDE_MONACO_THEME_NAME)
    if (!monacoTheme) throw new Error('shikiToMonaco did not define the taide theme')
    return { tokensProviders: monaco.tokensProviders, monacoTheme, tokenTheme: createTokenTheme(tokenization, monacoTheme) }
}

const probeThemeCombinations = async (tokenization: MonacoTokenizationModule, resolved: ResolvedTheme) => {
    const highlighter = await createThemedHighlighter(resolved, [])
    try {
        const { tokensProviders, monacoTheme, tokenTheme } = await attachTokensProviders(tokenization, highlighter, resolved)
        const { theme, colorMap } = highlighter.setTheme(TAIDE_MONACO_THEME_NAME)
        const combinations = [...colorMap.keys()].flatMap((colorIndex) => Array.from({ length: FONT_STYLE_VARIANTS }, (_unused, fontStyle) => ({ colorIndex, fontStyle })))
        highlighter.getLanguage(PROBE_LANGUAGE_ID).tokenizeLine2 = (_lineText, prevState) => ({
            tokens: Uint32Array.from(
                combinations.flatMap(({ colorIndex, fontStyle }, index) => [index, ((colorIndex << FOREGROUND_OFFSET) | (fontStyle << FONT_STYLE_OFFSET)) >>> 0]),
            ),
            ruleStack: prevState ?? INITIAL,
            stoppedEarly: false,
        })
        const provider = tokensProviders.get(PROBE_LANGUAGE_ID)
        if (!provider) throw new Error(`no tokens provider for ${PROBE_LANGUAGE_ID}`)
        const probed = provider.tokenize(PROBE_LINE, provider.getInitialState()).tokens
        if (probed.length !== combinations.length) throw new Error('style probe returned an unexpected token count')
        return {
            monacoTheme,
            settings: theme.settings,
            colorMap: Array.from(colorMap, (color) => color ?? null),
            monacoColorMap: Array.from(tokenTheme.getColorMap(), toHexColor),
            defaultFinalMetadata: matchMonacoMetadata(tokenTheme, ''),
            styles: combinations.map((combination, index) => ({
                ...combination,
                scope: probed[index].scopes,
                finalMetadata: matchMonacoMetadata(tokenTheme, probed[index].scopes),
            })),
        }
    } finally {
        highlighter.dispose()
    }
}

const probeThemeStyles = async (tokenization: MonacoTokenizationModule, resolved: ResolvedTheme) => {
    const { monacoTheme, settings, colorMap, monacoColorMap, defaultFinalMetadata, styles } = await probeThemeCombinations(tokenization, resolved)
    return {
        themeId: resolved.id,
        type: resolved.type,
        editorForeground: monacoTheme.colors['editor.foreground'] ?? null,
        editorBackground: monacoTheme.colors['editor.background'] ?? null,
        settings,
        colorMap,
        monacoColorMap,
        defaultFinalMetadata,
        styleScopes: styles.filter((style) => style.scope !== ''),
    }
}

const probeTokenThemeReference = async (tokenization: MonacoTokenizationModule, resolved: ResolvedTheme) => {
    try {
        const { settings, colorMap, monacoColorMap, defaultFinalMetadata, styles } = await probeThemeCombinations(tokenization, resolved)
        return { themeId: resolved.id, settings, colorMap, monacoColorMap, defaultFinalMetadata, finalMetadataByStyle: styles.map((style) => style.finalMetadata) }
    } catch (error) {
        return { themeId: resolved.id, error: error instanceof Error ? error.message : String(error) }
    }
}

const writeTokenThemeReferences = async (tokenization: MonacoTokenizationModule) => {
    const groups = [
        { group: BUNDLED_THEME_GROUP, directory: BUNDLED_THEMES_DIRECTORY, read: readResolvedBundledTheme },
        { group: SYNTHETIC_THEME_GROUP, directory: SYNTHETIC_THEMES_DIRECTORY, read: readSyntheticResolvedTheme },
    ]
    for (const { group, directory, read } of groups) {
        for (const themeId of await listThemeIds(directory)) {
            const reference = await probeTokenThemeReference(tokenization, await read(themeId))
            await Bun.write(join(TOKEN_THEME_REFERENCE_DIRECTORY, group, `${themeId}${THEME_FILE_EXTENSION}`), `${JSON.stringify(reference)}\n`)
            console.log(
                'error' in reference
                    ? `${group}/${themeId}: error ${reference.error}`
                    : `${group}/${themeId}: ${reference.colorMap.length} colors, ${reference.finalMetadataByStyle.length} styles`,
            )
        }
    }
}

const buildDocumentLines = (sampleText: string, sample: Sample, longLineFill: string) => {
    const authoredLines = (sampleText.endsWith('\n') ? sampleText.slice(0, -1) : sampleText).split('\n')
    if (authoredLines.some((line) => line.includes('\r'))) throw new Error(`${sample.id}: sample must use LF line endings`)
    const { prefix, suffix, tail } = sample.longLine
    const buildLongLine = (length: number) => `${prefix}${longLineFill.repeat(length - prefix.length - suffix.length)}${suffix}`
    return [...authoredLines, buildLongLine(TOKENIZE_MAX_LINE_LENGTH - 1), buildLongLine(TOKENIZE_MAX_LINE_LENGTH), tail]
}

const tokenizeDocument = async (
    tokenization: MonacoTokenizationModule,
    resolved: ResolvedTheme,
    sample: Sample,
    lines: string[],
    requestedLanguageIds: TaideLanguageId[],
) => {
    const highlighter = await createThemedHighlighter(resolved, requestedLanguageIds)
    try {
        const { tokensProviders, tokenTheme } = await attachTokensProviders(tokenization, highlighter, resolved)
        const provider = tokensProviders.get(sample.languageId)
        if (!provider) throw new Error(`${sample.id}: no tokens provider for ${sample.languageId}`)
        const grammar = highlighter.getLanguage(sample.languageId)
        const rawTokens: number[][] = []
        const finalTokens: number[][] = []
        const linesOverTimeLimit: string[] = []
        let state = provider.getInitialState()
        for (const [lineIndex, line] of lines.entries()) {
            const isTokenized = line.length < TOKENIZE_MAX_LINE_LENGTH
            const startedAt = performance.now()
            const raw = isTokenized ? grammar.tokenizeLine2(line, state.ruleStack, NO_TIME_LIMIT) : null
            const elapsedMs = performance.now() - startedAt
            if (elapsedMs > TOKENIZE_TIME_LIMIT_MS) linesOverTimeLimit.push(`${lineIndex}:${Math.round(elapsedMs)}ms`)
            const provided = provider.tokenize(line, state)
            const rawLine = raw ? [...raw.tokens] : []
            const hasSameBoundaries = provided.tokens.every((token, tokenIndex) => token.startIndex === rawLine[tokenIndex * 2])
            if (raw && (provided.tokens.length * 2 !== rawLine.length || !hasSameBoundaries)) {
                throw new Error(`${sample.id}: line ${lineIndex} tokenized differently on the second pass`)
            }
            rawTokens.push(rawLine)
            finalTokens.push(toMonacoBinaryTokens(tokenTheme, provided.tokens))
            state = provided.endState
        }
        return { rawTokens, finalTokens, linesOverTimeLimit }
    } finally {
        highlighter.dispose()
    }
}

const serializeNumberLines = (lines: number[][]) => `[\n${lines.map((line) => `${' '.repeat(JSON_INDENT * 2)}${JSON.stringify(line)}`).join(',\n')}\n${' '.repeat(JSON_INDENT)}]`

const serializeSampleReference = (reference: {
    sampleId: string
    languageId: string
    themeId: string
    requestedLanguageIds: string[]
    loadedScopeNames: string[]
    lineUtf16Lengths: number[]
    rawTokens: number[][]
    finalTokens: number[][]
}) => {
    const { rawTokens, finalTokens, ...header } = reference
    const headerEntries = Object.entries(header).map(([key, value]) => `${' '.repeat(JSON_INDENT)}${JSON.stringify(key)}: ${JSON.stringify(value)}`)
    const tokenEntries = [
        `${' '.repeat(JSON_INDENT)}"rawTokens": ${serializeNumberLines(rawTokens)}`,
        `${' '.repeat(JSON_INDENT)}"finalTokens": ${serializeNumberLines(finalTokens)}`,
    ]
    return `{\n${[...headerEntries, ...tokenEntries].join(',\n')}\n}\n`
}

const reportThemeSwitchBehavior = async (tokenization: MonacoTokenizationModule, themes: ResolvedTheme[], sample: Sample, lines: string[]) => {
    const [firstTheme, secondTheme] = themes
    if (!firstTheme || !secondTheme) return
    const requestedLanguageIds = [...new Set([...TAIDE_CORE_LANGUAGE_IDS, sample.languageId])]
    const tokenizeFinalLines = (provider: TokensProvider, tokenTheme: MonacoTokenTheme) => {
        let state = provider.getInitialState()
        return lines.map((line) => {
            const provided = provider.tokenize(line, state)
            state = provided.endState
            return JSON.stringify(toMonacoBinaryTokens(tokenTheme, provided.tokens))
        })
    }
    const requireProvider = (providers: Map<string, TokensProvider>) => {
        const provider = providers.get(sample.languageId)
        if (!provider) throw new Error(`${sample.id}: no tokens provider for ${sample.languageId}`)
        return provider
    }
    const switchedHighlighter = await createThemedHighlighter(firstTheme, requestedLanguageIds)
    const freshHighlighter = await createThemedHighlighter(secondTheme, requestedLanguageIds)
    try {
        await attachTokensProviders(tokenization, switchedHighlighter, firstTheme)
        const switched = await attachTokensProviders(tokenization, switchedHighlighter, secondTheme)
        const fresh = await attachTokensProviders(tokenization, freshHighlighter, secondTheme)
        const switchedLines = tokenizeFinalLines(requireProvider(switched.tokensProviders), switched.tokenTheme)
        const freshLines = tokenizeFinalLines(requireProvider(fresh.tokensProviders), fresh.tokenTheme)
        const differingLineCount = switchedLines.filter((line, index) => line !== freshLines[index]).length
        const isColorMapStale =
            JSON.stringify(switchedHighlighter.setTheme(TAIDE_MONACO_THEME_NAME).colorMap) !== JSON.stringify(freshHighlighter.setTheme(TAIDE_MONACO_THEME_NAME).colorMap)
        console.log(
            `theme switch ${firstTheme.id} -> ${secondTheme.id} on one highlighter (${sample.id}): ${differingLineCount}/${lines.length} lines differ from a fresh highlighter, textmate color map stale: ${isColorMapStale}`,
        )
    } finally {
        switchedHighlighter.dispose()
        freshHighlighter.dispose()
    }
}

const main = async () => {
    const tokenization: unknown = await import(MONACO_TOKENIZATION_MODULE)
    if (!isMonacoTokenizationModule(tokenization)) throw new Error('monaco tokenization module does not export TokenTheme')
    if (Bun.argv.includes(TOKEN_THEMES_FLAG)) {
        await writeTokenThemeReferences(tokenization)
        return
    }
    const manifest = await readSampleManifest()
    const themes = await Promise.all(REFERENCE_THEME_IDS.map(readResolvedBundledTheme))
    const documents = new Map<string, string[]>()
    for (const sample of manifest.samples) {
        documents.set(sample.id, buildDocumentLines(await Bun.file(join(SAMPLES_DIRECTORY, sample.file)).text(), sample, manifest.longLineFill))
    }

    for (const resolved of themes) {
        const themeReference = await probeThemeStyles(tokenization, resolved)
        await Bun.write(join(REFERENCE_DIRECTORY, resolved.id, 'theme.json'), `${JSON.stringify(themeReference, null, JSON_INDENT)}\n`)
        for (const sample of manifest.samples) {
            const lines = documents.get(sample.id) ?? []
            const requestedLanguageIds = [...new Set([...TAIDE_CORE_LANGUAGE_IDS, sample.languageId, ...sample.additionalLanguageIds])]
            const loadedScopeNames = [...new Set((await loadTaideGrammars(requestedLanguageIds)).map((registration) => registration.scopeName))].toSorted()
            const { rawTokens, finalTokens, linesOverTimeLimit } = await tokenizeDocument(tokenization, resolved, sample, lines, requestedLanguageIds)
            await Bun.write(
                join(REFERENCE_DIRECTORY, resolved.id, `${sample.id}.json`),
                serializeSampleReference({
                    sampleId: sample.id,
                    languageId: sample.languageId,
                    themeId: resolved.id,
                    requestedLanguageIds,
                    loadedScopeNames,
                    lineUtf16Lengths: lines.map((line) => line.length),
                    rawTokens,
                    finalTokens,
                }),
            )
            console.log(
                `${resolved.id}/${sample.id}: ${lines.length} lines, ${rawTokens.reduce((total, line) => total + line.length / 2, 0)} raw tokens, over ${TOKENIZE_TIME_LIMIT_MS}ms: [${linesOverTimeLimit.join(', ')}]`,
            )
        }
    }

    const [firstSample] = manifest.samples
    if (firstSample) await reportThemeSwitchBehavior(tokenization, themes, firstSample, documents.get(firstSample.id) ?? [])
}

await main()
