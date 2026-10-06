import { createHash } from 'node:crypto'
import { join, resolve } from 'node:path'
import { loadTaideGrammar, TAIDE_LANGUAGE_IDS } from '../../src/shared/lib/shiki/lang-map'

const REPOSITORY_ROOT = resolve(import.meta.dir, '../..')
const SHIKI_LANGS_PACKAGE_DIRECTORY = join(REPOSITORY_ROOT, 'node_modules/@shikijs/langs')
const LANG_MAP_PATH = join(REPOSITORY_ROOT, 'src/shared/lib/shiki/lang-map.ts')
const OUTPUT_DIRECTORY = join(REPOSITORY_ROOT, 'native/taide-native-syntax/grammars')
const EXPECTED_SHIKI_LANGS_VERSION = '4.4.3'
const GPL_GRAMMAR_IDS = ['ada', 'gnuplot', 'nginx', 'org', 'racket']
const LOADER_PATTERN = /^ {4}(\w+): async \(\) => (?:renameMainGrammar\()?\(await import\('@shikijs\/langs\/([a-z0-9-]+)'\)\)\.default/gm
const STATIC_IMPORT_PATTERN = /^import [\w$]+ from '\.\/([a-z0-9-]+)\.mjs'$/gm
const GRAMMAR_LITERAL_PREFIX = 'Object.freeze(JSON.parse('
const GRAMMAR_LITERAL_SUFFIX = '"))'
const MANIFEST_INDENT = 4
const STRUCTURAL_GRAMMAR_KEYS = ['patterns', 'repository', 'injections']

type ExtractedGrammar = Awaited<ReturnType<typeof extractGrammar>>

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)

const readStringList = (value: unknown) => {
    const items: unknown[] = Array.isArray(value) ? value : []
    return items.filter((item): item is string => typeof item === 'string')
}

const readGrammarIdByLanguageId = async () => {
    const source = await Bun.file(LANG_MAP_PATH).text()
    const grammarIdByLanguageId = new Map([...source.matchAll(LOADER_PATTERN)].map((match) => [match[1], match[2]]))
    const missing = TAIDE_LANGUAGE_IDS.filter((languageId) => !grammarIdByLanguageId.has(languageId))
    if (missing.length > 0 || grammarIdByLanguageId.size !== TAIDE_LANGUAGE_IDS.length) {
        throw new Error(`lang-map.ts loaders do not match TAIDE_LANGUAGE_IDS, missing: ${missing.join(', ')}`)
    }
    return grammarIdByLanguageId
}

const extractGrammarLiteral = (moduleSource: string, grammarId: string) => {
    const prefixIndex = moduleSource.indexOf(GRAMMAR_LITERAL_PREFIX)
    if (prefixIndex === -1 || moduleSource.indexOf(GRAMMAR_LITERAL_PREFIX, prefixIndex + 1) !== -1) {
        throw new Error(`${grammarId}: expected exactly one ${GRAMMAR_LITERAL_PREFIX} literal`)
    }
    const start = prefixIndex + GRAMMAR_LITERAL_PREFIX.length
    if (moduleSource[start] !== '"') throw new Error(`${grammarId}: grammar literal is not a double-quoted string`)
    let end = start + 1
    while (end < moduleSource.length && moduleSource[end] !== '"') end += moduleSource[end] === '\\' ? 2 : 1
    if (moduleSource.slice(end, end + GRAMMAR_LITERAL_SUFFIX.length) !== GRAMMAR_LITERAL_SUFFIX) {
        throw new Error(`${grammarId}: grammar literal is not terminated by ${GRAMMAR_LITERAL_SUFFIX}`)
    }
    return moduleSource.slice(start, end + 1)
}

const decodeGrammarText = (literal: string, grammarId: string) => {
    const decoded: unknown = JSON.parse(literal)
    if (typeof decoded !== 'string') throw new Error(`${grammarId}: grammar literal did not decode to a string`)
    if (new TextDecoder().decode(new TextEncoder().encode(decoded)) !== decoded) {
        throw new Error(`${grammarId}: grammar text is not well-formed UTF-16 and cannot be stored as UTF-8 unchanged`)
    }
    return decoded
}

const extractGrammar = async (grammarId: string) => {
    const moduleSource = await Bun.file(join(SHIKI_LANGS_PACKAGE_DIRECTORY, 'dist', `${grammarId}.mjs`)).text()
    const text = decodeGrammarText(extractGrammarLiteral(moduleSource, grammarId), grammarId)
    const grammar: unknown = JSON.parse(text)
    if (!isRecord(grammar) || typeof grammar.scopeName !== 'string') throw new Error(`${grammarId}: grammar JSON has no scopeName`)
    return {
        id: grammarId,
        scopeName: grammar.scopeName,
        displayName: typeof grammar.displayName === 'string' ? grammar.displayName : null,
        embeddedLangs: readStringList(grammar.embeddedLangs),
        embeddedLangsLazy: readStringList(grammar.embeddedLangsLazy),
        aliases: readStringList(grammar.aliases),
        injectTo: readStringList(grammar.injectTo),
        imports: [...moduleSource.matchAll(STATIC_IMPORT_PATTERN)].map((match) => match[1]),
        bytes: new TextEncoder().encode(text).length,
        sha256: createHash('sha256').update(text).digest('hex'),
        text,
        topLevelKeys: Object.keys(grammar).filter((key) => !STRUCTURAL_GRAMMAR_KEYS.includes(key)),
    }
}

const extractGrammarClosure = async (rootGrammarIds: string[]) => {
    const grammars = new Map<string, ExtractedGrammar>()
    const pending = [...rootGrammarIds]
    while (pending.length > 0) {
        const grammarId = pending.shift()
        if (grammarId === undefined || grammars.has(grammarId)) continue
        const grammar = await extractGrammar(grammarId)
        grammars.set(grammarId, grammar)
        pending.push(...grammar.imports)
    }
    return grammars
}

const assertLoaderMatchesExtraction = async (grammarIdByLanguageId: Map<string, string>, grammars: Map<string, ExtractedGrammar>) => {
    const grammarByScopeName = new Map([...grammars.values()].map((grammar) => [grammar.scopeName, grammar]))
    for (const languageId of TAIDE_LANGUAGE_IDS) {
        const registrations = await loadTaideGrammar(languageId)
        const mainRegistration = registrations.at(-1)
        const mainGrammar = grammars.get(grammarIdByLanguageId.get(languageId) ?? '')
        if (!mainRegistration || !mainGrammar || mainRegistration.scopeName !== mainGrammar.scopeName || mainRegistration.name !== languageId) {
            throw new Error(`${languageId}: loader main grammar does not match the extracted module`)
        }
        for (const registration of registrations) {
            const extracted = grammarByScopeName.get(registration.scopeName)
            if (!extracted) throw new Error(`${languageId}: loader registers ${registration.scopeName}, which was not extracted`)
            const parsed: unknown = JSON.parse(extracted.text)
            if (!isRecord(parsed) || JSON.stringify({ ...registration, name: parsed.name }) !== JSON.stringify(parsed)) {
                throw new Error(`${languageId}: extracted ${extracted.id} differs from the registration the TS loader returns`)
            }
        }
    }
}

const writeGrammars = async (grammars: ExtractedGrammar[]) => {
    for (const grammar of grammars) {
        const outputPath = join(OUTPUT_DIRECTORY, `${grammar.id}.tmLanguage.json`)
        await Bun.write(outputPath, grammar.text)
        if ((await Bun.file(outputPath).text()) !== grammar.text) throw new Error(`${grammar.id}: written file differs from the extracted text`)
    }
}

const main = async () => {
    const packageJson: unknown = await Bun.file(join(SHIKI_LANGS_PACKAGE_DIRECTORY, 'package.json')).json()
    if (!isRecord(packageJson) || packageJson.version !== EXPECTED_SHIKI_LANGS_VERSION) {
        throw new Error(`@shikijs/langs must be ${EXPECTED_SHIKI_LANGS_VERSION}`)
    }

    const grammarIdByLanguageId = await readGrammarIdByLanguageId()
    const grammarsById = await extractGrammarClosure([...new Set(grammarIdByLanguageId.values())])
    const grammars = [...grammarsById.values()].toSorted((first, second) => first.id.localeCompare(second.id))

    const gplGrammarIds = grammars.filter((grammar) => GPL_GRAMMAR_IDS.includes(grammar.id)).map((grammar) => grammar.id)
    if (gplGrammarIds.length > 0) throw new Error(`GPL grammars must not be bundled: ${gplGrammarIds.join(', ')}`)

    await assertLoaderMatchesExtraction(grammarIdByLanguageId, grammarsById)
    await writeGrammars(grammars)

    const manifest = {
        package: '@shikijs/langs',
        version: EXPECTED_SHIKI_LANGS_VERSION,
        languages: TAIDE_LANGUAGE_IDS.map((languageId) => ({ languageId, grammarId: grammarIdByLanguageId.get(languageId) })),
        grammars: grammars.map(({ text: _text, topLevelKeys: _topLevelKeys, ...metadata }) => metadata),
    }
    await Bun.write(join(OUTPUT_DIRECTORY, 'manifest.json'), `${JSON.stringify(manifest, null, MANIFEST_INDENT)}\n`)

    console.table(
        grammars.map((grammar) => ({
            id: grammar.id,
            scopeName: grammar.scopeName,
            bytes: grammar.bytes,
            imports: grammar.imports.join(' '),
            lazy: grammar.embeddedLangsLazy.length,
            injectTo: grammar.injectTo.join(' '),
            keys: grammar.topLevelKeys.join(' '),
        })),
    )
    console.log(`grammars: ${grammars.length}, bytes: ${grammars.reduce((total, grammar) => total + grammar.bytes, 0)}`)
}

await main()
