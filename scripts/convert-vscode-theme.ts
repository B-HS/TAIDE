import { format, resolveConfig } from 'prettier'
import { isHexColor } from '@shared/lib/color'
import { convertVscodeTheme } from '@shared/lib/theme-convert/convert'
import { parseJsonc } from '@shared/lib/theme-convert/jsonc'
import type { ThemeTypeArg } from '@shared/lib/theme-convert/types'

const KEBAB_ID_PATTERN = /^[a-z0-9]+(-[a-z0-9]+)*$/
const MAX_INCLUDE_CHAIN_DEPTH = 5

type CliArgs = {
    input: string
    id: string
    name: string
    type: ThemeTypeArg
    sourceUrl: string
    author: string
    license: string
    out: string
    includeDir?: string
}

const parseArgs = (argv: string[]): CliArgs => {
    const flags = new Map<string, string>()
    for (let index = 0; index < argv.length; index += 1) {
        const token = argv[index]
        if (!token.startsWith('--')) continue
        flags.set(token.slice(2), argv[index + 1])
        index += 1
    }

    const required = ['input', 'id', 'name', 'type', 'source-url', 'author', 'license']
    const missing = required.filter((key) => !flags.get(key))
    if (missing.length > 0) {
        console.error(`Missing required args: ${missing.map((key) => `--${key}`).join(', ')}`)
        process.exit(1)
    }

    const type = flags.get('type')
    if (type !== 'dark' && type !== 'light') {
        console.error(`--type must be 'dark' or 'light', got: ${type}`)
        process.exit(1)
    }

    const id = flags.get('id') ?? ''
    if (!KEBAB_ID_PATTERN.test(id)) {
        console.error(`--id must be kebab-case (lowercase letters, digits, hyphens), got: ${id}`)
        process.exit(1)
    }

    return {
        input: flags.get('input') ?? '',
        id,
        name: flags.get('name') ?? '',
        type,
        sourceUrl: flags.get('source-url') ?? '',
        author: flags.get('author') ?? '',
        license: flags.get('license') ?? '',
        out: flags.get('out') ?? 'src-tauri/resources/themes/',
        includeDir: flags.get('include-dir'),
    }
}

/**
 * VS Code's built-in theme-defaults extension ships include chains that
 * reference sibling files by their original repo filename (e.g.
 * "./dark_vs.json"). This repo's local mirror of those sources uses
 * different filenames (base-dark-vs.json, vscode-dark-plus.json, ...), so
 * an include path must be mapped explicitly to the file that actually
 * holds that role rather than resolved literally against --include-dir.
 */
const INCLUDE_ROLE_FILENAME_MAP: Record<string, string> = {
    'dark_vs.json': 'base-dark-vs.json',
    'light_vs.json': 'base-light-vs.json',
    'dark_plus.json': 'vscode-dark-plus.json',
    'light_plus.json': 'vscode-light-plus.json',
}

const resolveIncludeFilename = (includePath: string): string => {
    const normalized = includePath.replace(/^\.\//, '')
    return INCLUDE_ROLE_FILENAME_MAP[normalized] ?? normalized
}

const loadRawThemeChain = async (inputPath: string, includeDir: string | undefined, depth = 0): Promise<Record<string, unknown>[]> => {
    if (depth > MAX_INCLUDE_CHAIN_DEPTH) {
        console.error(`convert-vscode-theme: include chain deeper than ${MAX_INCLUDE_CHAIN_DEPTH} levels while resolving '${inputPath}'`)
        process.exit(1)
    }

    const source = await Bun.file(inputPath).text()
    const raw = parseJsonc(source)
    const includePath = typeof raw.include === 'string' ? raw.include : undefined
    if (!includePath) return [raw]

    if (!includeDir) {
        console.error(`convert-vscode-theme: '${inputPath}' declares "include": "${includePath}" but --include-dir was not provided`)
        process.exit(1)
    }

    const basePath = `${includeDir.replace(/\/$/, '')}/${resolveIncludeFilename(includePath)}`
    const baseChain = await loadRawThemeChain(basePath, includeDir, depth + 1)
    return [...baseChain, raw]
}

const main = async () => {
    const args = parseArgs(process.argv.slice(2))

    const rawChain = await loadRawThemeChain(args.input, args.includeDir)
    const result = convertVscodeTheme(rawChain, args.type)

    if (result.status === 'incomplete') {
        const allMissing = [...new Set([...result.missingColors, ...result.missingSyntax, ...result.missingTerminal])]
        console.error(`convert-vscode-theme: incomplete output for '${args.id}', missing tokens:`)
        for (const key of allMissing) console.error(`  - ${key}`)
        process.exit(1)
    }

    if (result.safeDefaultNotices.length > 0) {
        console.warn(`convert-vscode-theme: '${args.id}' used ${args.type} safe-default fallback for tokens with no matching source color:`)
        for (const notice of result.safeDefaultNotices) console.warn(`  - ${notice}`)
    }

    if (result.ansiFallbackTokens.length > 0) {
        console.warn(
            `convert-vscode-theme: '${args.id}' has no terminal.ansi* colors in source — used VS Code's official default ${args.type} ANSI palette (terminalColorRegistry) for: ${result.ansiFallbackTokens.join(', ')}`,
        )
    }

    if (result.repairs.length > 0) {
        console.warn(`convert-vscode-theme: '${args.id}' substituted a low-contrast token with a same-family alternative:`)
        for (const repair of result.repairs) console.warn(`  - ${repair}`)
    }

    if (result.outputColorErrors.length > 0) {
        console.error(`convert-vscode-theme: output color validation failed for '${args.id}':`)
        for (const error of result.outputColorErrors) console.error(`  - ${error}`)
        process.exit(1)
    }

    if (result.tokenColors.length === 0) {
        console.warn(`convert-vscode-theme: '${args.id}' produced 0 tokenColors rules — source may lack usable scope-based syntax rules`)
    }

    const nonHexForegroundScopes = result.tokenColors
        .filter((rule) => typeof rule.settings.foreground === 'string' && !isHexColor(rule.settings.foreground))
        .map((rule) => rule.scope.join(', '))
    if (nonHexForegroundScopes.length > 0) {
        console.warn(`convert-vscode-theme: '${args.id}' has tokenColors rules with a non-hex foreground:`)
        for (const scope of nonHexForegroundScopes) console.warn(`  - ${scope}`)
    }

    const output = {
        version: 1,
        id: args.id,
        name: args.name,
        type: args.type,
        palette: {},
        colors: result.colors,
        syntax: result.syntax,
        tokenColors: result.tokenColors,
        terminal: result.terminal,
        author: args.author,
        license: args.license,
        source: args.sourceUrl,
    }

    const outputPath = `${args.out.replace(/\/$/, '')}/${args.id}.json`
    const prettierConfig = await resolveConfig(outputPath)
    await Bun.write(outputPath, await format(JSON.stringify(output), { ...prettierConfig, filepath: outputPath }))
    console.log(
        `convert-vscode-theme: wrote ${args.out.replace(/\/$/, '')}/${args.id}.json (${Object.keys(result.colors).length} colors, ${Object.keys(result.syntax).length} syntax, ${Object.keys(result.terminal).length} terminal, ${result.tokenColors.length} tokenColors)`,
    )
}

await main()
