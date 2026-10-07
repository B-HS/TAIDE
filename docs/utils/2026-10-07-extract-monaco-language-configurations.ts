import { dirname, join, relative, resolve } from 'node:path'
import { TAIDE_LANGUAGE_IDS } from '../../src/shared/lib/shiki/lang-map'

const REPOSITORY_ROOT = resolve(import.meta.dir, '../..')
const MONACO_PACKAGE_DIRECTORY = join(REPOSITORY_ROOT, 'node_modules/monaco-editor')
const MONACO_SOURCE_DIRECTORY = join(MONACO_PACKAGE_DIRECTORY, 'esm/vs')
const DEFINITIONS_DIRECTORY = join(MONACO_SOURCE_DIRECTORY, 'languages/definitions')
const JSON_MODE_PATH = join(MONACO_SOURCE_DIRECTORY, 'languages/features/json/jsonMode.js')
const JSON_REGISTER_PATH = join(MONACO_SOURCE_DIRECTORY, 'languages/features/json/register.js')
const SAMPLES_DIRECTORY = join(REPOSITORY_ROOT, 'native/taide-native-syntax/tests/fixtures/samples')
const CONFIGURATION_OUTPUT_PATH = join(REPOSITORY_ROOT, 'native/taide-native-syntax/language-configurations/monaco.json')
const REFERENCE_OUTPUT_PATH = join(REPOSITORY_ROOT, 'native/taide-native-syntax/tests/fixtures/language-configurations/reference.json')
const EXPECTED_MONACO_VERSION = '0.56.0'
const PLAINTEXT_LANGUAGE_ID = 'plaintext'
const JSON_LANGUAGE_ID = 'json'
const JSON_CONFIGURATION_DECLARATION = 'const richEditConfiguration = '
const EDITOR_API_SPECIFIER_SUFFIX = 'editor/editor.api.js'
const FULL_AUTO_INDENT = 4
const OUTPUT_INDENT = 4
const MAX_SPLITS_PER_LINE = 8
const SPLIT_CHARACTERS = '{}[]()<>*/"\'`'
const IMPORT_PATTERN = /^import \{([^}]*)\} from '([^']+)';$/gm
const EXPORT_PATTERN = /^export \{([^}]*)\};$/m
const REGISTRATION_PATTERN = /registerLanguage\(\{\s*id: "([^"]+)"[\s\S]*?loader: \(\) => (?:\{\s*return )?import\('\.\/([\w.-]+)'\)/g
const CONFIGURATION_KEYS = [
    'comments',
    'brackets',
    'wordPattern',
    'indentationRules',
    'onEnterRules',
    'autoClosingPairs',
    'surroundingPairs',
    'autoCloseBefore',
    'folding',
    'colorizedBracketPairs',
    '__electricCharacterSupport',
]
const SHARED_PROBES = [
    '',
    ' ',
    '\t',
    '{',
    '}',
    '    {',
    '    }',
    'if (value) {',
    'call(',
    'items = [',
    ']',
    ')',
    '});',
    '} else {',
    'function name(first, second) {',
    'const text = "{";',
    "const text = '(';",
    '/**',
    '/** summary',
    '/** summary */',
    ' * continued',
    '  * continued',
    '\t * continued',
    ' */',
    '\t */',
    '   */  ',
    '/*',
    ' * /',
    '// region',
    '//#region name',
    '// #endregion',
    '#region name',
    '#endregion',
    '#pragma region name',
    '#pragma endregion',
    '<!-- #region -->',
    '<!-- #endregion -->',
    '<div>',
    '</div>',
    '<div class="box">',
    '<br>',
    '<br/>',
    '<input type="text">',
    '<ul><li>',
    '<!-- note',
    '-->',
    'def name(first):',
    'class Name:',
    'if value:',
    'else:',
    'return value',
    'pass',
    'begin',
    'end',
    'do',
    'items.each do |item|',
    'if value then',
    'function name()',
    'repeat',
    'until done',
    'case value',
    'when 1',
    'key:',
    '- item',
    'key: [',
    'key: {',
    'word-with-dash another_word',
    'café {',
    '한글 (',
    ' {',
    'BEGIN',
    'End',
    'endif',
    'doing',
]

type MonacoRegExp = { source: string; flags: string }
type Bindings = Record<string, unknown>

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)

const repositoryPath = (path: string) => relative(REPOSITORY_ROOT, path)

const evaluateModule = async (path: string, editorApi: Bindings, evaluated: Map<string, Bindings>): Promise<Bindings> => {
    const known = evaluated.get(path)
    if (known) return known
    const source = await Bun.file(path).text()
    const names: string[] = []
    const values: unknown[] = []
    for (const [, clause, specifier] of source.matchAll(IMPORT_PATTERN)) {
        const imported = specifier.endsWith(EDITOR_API_SPECIFIER_SUFFIX) ? editorApi : await evaluateModule(resolve(dirname(path), specifier), editorApi, evaluated)
        for (const binding of clause.split(',')) {
            const [exportedName, localName = exportedName] = binding.trim().split(' as ')
            if (!(exportedName in imported)) throw new Error(`${repositoryPath(path)}: ${specifier} has no export ${exportedName}`)
            names.push(localName)
            values.push(imported[exportedName])
        }
    }
    const withoutImports = source.replace(IMPORT_PATTERN, '')
    if (/^import /m.test(withoutImports)) throw new Error(`${repositoryPath(path)}: unsupported import form`)
    const exported = EXPORT_PATTERN.exec(withoutImports)
    if (!exported) throw new Error(`${repositoryPath(path)}: expected one export list`)
    const body = withoutImports.replace(EXPORT_PATTERN, `return { ${exported[1]} };`)
    const bindings: unknown = new Function(...names, body)(...values)
    if (!isRecord(bindings)) throw new Error(`${repositoryPath(path)}: module did not evaluate to bindings`)
    evaluated.set(path, bindings)
    return bindings
}

const readObjectLiteral = (source: string, declaration: string, path: string) => {
    const declarationIndex = source.indexOf(declaration)
    if (declarationIndex === -1 || source.indexOf(declaration, declarationIndex + 1) !== -1) {
        throw new Error(`${repositoryPath(path)}: expected exactly one "${declaration}"`)
    }
    const start = declarationIndex + declaration.length
    const end = source.indexOf('\n};', start)
    if (source[start] !== '{' || end === -1) throw new Error(`${repositoryPath(path)}: "${declaration}" is not an object literal`)
    return source.slice(start, end + 2)
}

const readDefinitionModules = async () => {
    const moduleByLanguageId = new Map<string, string>()
    for await (const registerPath of new Bun.Glob('*/register.js').scan({ cwd: DEFINITIONS_DIRECTORY, absolute: true })) {
        const source = await Bun.file(registerPath).text()
        for (const [, languageId, moduleFile] of source.matchAll(REGISTRATION_PATTERN)) {
            if (moduleByLanguageId.has(languageId)) throw new Error(`${languageId}: registered by more than one Monaco definition`)
            moduleByLanguageId.set(languageId, join(dirname(registerPath), moduleFile))
        }
    }
    return moduleByLanguageId
}

const serialize = (value: unknown, path: string): unknown => {
    if (value instanceof RegExp) return { source: value.source, flags: value.flags } satisfies MonacoRegExp
    if (Array.isArray(value)) return value.map((item, index) => serialize(item, `${path}[${index}]`))
    if (isRecord(value)) return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, serialize(item, `${path}.${key}`)]))
    if (value === undefined || typeof value === 'function') throw new Error(`${path}: value cannot be stored`)
    return value
}

const serializeConfiguration = (configuration: unknown, indentActionNames: Map<unknown, string>, languageId: string) => {
    if (!isRecord(configuration)) throw new Error(`${languageId}: language configuration is not an object`)
    const unknownKeys = Object.keys(configuration).filter((key) => !CONFIGURATION_KEYS.includes(key))
    if (unknownKeys.length > 0) throw new Error(`${languageId}: unexpected language configuration keys ${unknownKeys.join(', ')}`)
    const onEnterRules: unknown[] = Array.isArray(configuration.onEnterRules) ? configuration.onEnterRules : []
    const namedRules = onEnterRules.map((rule, index) => {
        if (!isRecord(rule) || !isRecord(rule.action)) throw new Error(`${languageId}: onEnterRules[${index}] has no action`)
        const indentAction = indentActionNames.get(rule.action.indentAction)
        if (indentAction === undefined) throw new Error(`${languageId}: onEnterRules[${index}] has an unknown indentAction`)
        return { ...rule, action: { ...rule.action, indentAction } }
    })
    return serialize(configuration.onEnterRules === undefined ? configuration : { ...configuration, onEnterRules: namedRules }, languageId)
}

const collectRegExps = (value: unknown, path: string): { path: string; regExp: RegExp }[] => {
    if (value instanceof RegExp) return [{ path, regExp: value }]
    if (Array.isArray(value)) return value.flatMap((item, index) => collectRegExps(item, `${path}[${index}]`))
    if (isRecord(value)) return Object.entries(value).flatMap(([key, item]) => collectRegExps(item, path === '' ? key : `${path}.${key}`))
    return []
}

const testFromStart = (regExp: RegExp, text: string) => {
    regExp.lastIndex = 0
    return regExp.test(text)
}

const readCorpus = async (languageId: string) => {
    const sample = Bun.file(join(SAMPLES_DIRECTORY, `${languageId}.sample`))
    const sampleLines = (await sample.exists()) ? (await sample.text()).split(/\r\n|\r|\n/) : []
    return [...new Set([...sampleLines, ...SHARED_PROBES])]
}

const splitOffsets = (line: string) => {
    const offsets = new Set([0, line.length])
    for (let index = 0; index < line.length && offsets.size < MAX_SPLITS_PER_LINE; index += 1) {
        if (!SPLIT_CHARACTERS.includes(line[index])) continue
        offsets.add(index)
        offsets.add(index + 1)
    }
    return [...offsets].toSorted((first, second) => first - second)
}

const main = async () => {
    const packageJson: unknown = await Bun.file(join(MONACO_PACKAGE_DIRECTORY, 'package.json')).json()
    if (!isRecord(packageJson) || packageJson.version !== EXPECTED_MONACO_VERSION) throw new Error(`monaco-editor must be ${EXPECTED_MONACO_VERSION}`)

    const { IndentAction } = await import(join(MONACO_SOURCE_DIRECTORY, 'editor/common/languages/languageConfiguration.js'))
    const { OnEnterSupport } = await import(join(MONACO_SOURCE_DIRECTORY, 'editor/common/languages/supports/onEnter.js'))
    const { IndentRulesSupport } = await import(join(MONACO_SOURCE_DIRECTORY, 'editor/common/languages/supports/indentRules.js'))
    const { LanguageBracketsConfiguration } = await import(join(MONACO_SOURCE_DIRECTORY, 'editor/common/languages/supports/languageBracketsConfiguration.js'))
    const { LanguageConfigurationRegistry } = await import(join(MONACO_SOURCE_DIRECTORY, 'editor/common/languages/languageConfigurationRegistry.js'))
    const { RichEditBrackets, BracketsUtils } = await import(join(MONACO_SOURCE_DIRECTORY, 'editor/common/languages/supports/richEditBrackets.js'))
    const { BracketTokens } = await import(join(MONACO_SOURCE_DIRECTORY, 'editor/common/model/bracketPairsTextModelPart/bracketPairsTree/brackets.js'))
    const { DenseKeyProvider } = await import(join(MONACO_SOURCE_DIRECTORY, 'editor/common/model/bracketPairsTextModelPart/bracketPairsTree/smallImmutableSet.js'))
    const indentActionNames = new Map<unknown, string>(Object.entries(IndentAction).flatMap(([name, value]) => (typeof value === 'number' ? [[value, name]] : [])))
    const editorApi = { languages: { IndentAction }, editor: {} }

    const definitionModules = await readDefinitionModules()
    const evaluated = new Map<string, Bindings>()
    const jsonRegisterSource = await Bun.file(JSON_REGISTER_PATH).text()
    if (!jsonRegisterSource.includes(`languages.onLanguage("${JSON_LANGUAGE_ID}"`) || definitionModules.has(JSON_LANGUAGE_ID)) {
        throw new Error('json: expected the configuration to come from the JSON language feature only')
    }

    const combinedPlaintext: unknown = new LanguageConfigurationRegistry().getLanguageConfiguration(PLAINTEXT_LANGUAGE_ID)?.underlyingConfig
    if (!isRecord(combinedPlaintext)) throw new Error('plaintext: Monaco registered no language configuration')
    const plaintextConfiguration = Object.fromEntries(Object.entries(combinedPlaintext).filter(([, value]) => value !== undefined))
    const sources: { languageId: string; monacoModule: string | null; configuration: unknown }[] = [
        { languageId: PLAINTEXT_LANGUAGE_ID, monacoModule: 'editor/common/languages/languageConfigurationRegistry.js', configuration: plaintextConfiguration },
    ]
    for (const languageId of TAIDE_LANGUAGE_IDS) {
        const modulePath = definitionModules.get(languageId)
        if (languageId === JSON_LANGUAGE_ID) {
            const literal = readObjectLiteral(await Bun.file(JSON_MODE_PATH).text(), JSON_CONFIGURATION_DECLARATION, JSON_MODE_PATH)
            const configuration: unknown = new Function(`return (${literal});`)()
            sources.push({ languageId, monacoModule: relative(MONACO_SOURCE_DIRECTORY, JSON_MODE_PATH), configuration })
        } else if (modulePath) {
            const bindings = await evaluateModule(modulePath, editorApi, evaluated)
            sources.push({ languageId, monacoModule: relative(MONACO_SOURCE_DIRECTORY, modulePath), configuration: bindings.conf })
        } else {
            sources.push({ languageId, monacoModule: null, configuration: null })
        }
    }

    const configurations = {
        package: 'monaco-editor',
        version: EXPECTED_MONACO_VERSION,
        languages: sources.map(({ languageId, monacoModule, configuration }) => ({
            languageId,
            monacoModule,
            configuration: configuration === null ? null : serializeConfiguration(configuration, indentActionNames, languageId),
        })),
    }
    await Bun.write(CONFIGURATION_OUTPUT_PATH, `${JSON.stringify(configurations, null, OUTPUT_INDENT)}\n`)

    const references = []
    for (const { languageId, configuration } of sources) {
        if (!isRecord(configuration)) continue
        const corpus = await readCorpus(languageId)
        const hasEnterSupport = Boolean(configuration.brackets || configuration.indentationRules || configuration.onEnterRules)
        const enterSupport = hasEnterSupport ? new OnEnterSupport({ ...configuration }) : null
        const indentRules = configuration.indentationRules ? new IndentRulesSupport(configuration.indentationRules) : null
        const bracketsConfiguration = new LanguageBracketsConfiguration(languageId, configuration)
        const hasBrackets = bracketsConfiguration.openingBrackets.length + bracketsConfiguration.closingBrackets.length > 0
        const bracketRegExp: RegExp | null = hasBrackets ? bracketsConfiguration.getBracketRegExp({ global: true }) : null
        const richEditBrackets = configuration.brackets ? new RichEditBrackets(languageId, configuration.brackets) : null
        const treeBracketSource: string | null = BracketTokens.createFromLanguage({ bracketsNew: bracketsConfiguration }, new DenseKeyProvider()).getRegExpStr()
        const treeBracketRegExp = treeBracketSource === null ? null : new RegExp(treeBracketSource, 'gi')
        const markers = isRecord(configuration.folding) && isRecord(configuration.folding.markers) ? configuration.folding.markers : null
        const enterActions: unknown[] = []
        if (enterSupport) {
            corpus.forEach((line, index) => {
                const previousIndex = index === 0 ? null : index - 1
                for (const offset of splitOffsets(line)) {
                    const action = enterSupport.onEnter(FULL_AUTO_INDENT, previousIndex === null ? '' : corpus[previousIndex], line.slice(0, offset), line.slice(offset))
                    if (!action) continue
                    enterActions.push([
                        previousIndex,
                        index,
                        offset,
                        indentActionNames.get(action.indentAction),
                        action.appendText ?? null,
                        action.removeText ?? null,
                    ])
                }
            })
        }
        references.push({
            languageId,
            corpus,
            regExps: collectRegExps(configuration, '').map(({ path, regExp }) => ({
                path,
                source: regExp.source,
                flags: regExp.flags,
                matches: corpus.map((line) => (testFromStart(regExp, line) ? '1' : '0')).join(''),
            })),
            enterBrackets: enterSupport
                ? enterSupport._brackets.map((bracket: { open: string; close: string; openRegExp: RegExp; closeRegExp: RegExp }) => ({
                      open: bracket.open,
                      close: bracket.close,
                      openSource: bracket.openRegExp.source,
                      closeSource: bracket.closeRegExp.source,
                  }))
                : null,
            enterSplits: enterSupport ? corpus.map(splitOffsets) : null,
            enterActions,
            indentMetadata: indentRules ? corpus.map((line) => indentRules.getIndentMetadata(line)) : null,
            bracketRegExp: bracketRegExp ? { source: bracketRegExp.source, flags: bracketRegExp.flags } : null,
            withoutBrackets: bracketRegExp ? corpus.map((line) => line.replace(bracketRegExp, '')) : null,
            reversedBracketSource: richEditBrackets ? richEditBrackets.reversedRegex.source : null,
            lastBrackets: richEditBrackets
                ? corpus.map((line) => {
                      const found = BracketsUtils.findPrevBracketInRange(richEditBrackets.reversedRegex, 1, line, 0, line.length)
                      return found ? [found.startColumn - 1, found.endColumn - 1] : null
                  })
                : null,
            treeBracketSource,
            treeBrackets: treeBracketRegExp ? corpus.map((line) => [...line.matchAll(treeBracketRegExp)].map((match) => [match.index, match.index + match[0].length])) : null,
            foldMarkers: markers
                ? corpus
                      .map((line) => {
                          const isStart = markers.start instanceof RegExp && testFromStart(markers.start, line)
                          const isEnd = markers.end instanceof RegExp && testFromStart(markers.end, line)
                          return Number(isStart) + 2 * Number(isEnd)
                      })
                      .join('')
                : null,
        })
    }
    await Bun.write(REFERENCE_OUTPUT_PATH, `${JSON.stringify({ package: 'monaco-editor', version: EXPECTED_MONACO_VERSION, languages: references })}\n`)

    console.table(
        sources.map(({ languageId, monacoModule, configuration }) => {
            const present = isRecord(configuration) ? configuration : {}
            const folding = isRecord(present.folding) ? present.folding : {}
            return {
                languageId,
                module: monacoModule ?? '(not registered by Monaco)',
                keys: Object.keys(present).join(' '),
                onEnterRules: Array.isArray(present.onEnterRules) ? present.onEnterRules.length : 0,
                indentationRules: isRecord(present.indentationRules) ? Object.keys(present.indentationRules).join(' ') : '',
                folding: Object.keys(folding).join(' '),
            }
        }),
    )
    console.log(`regular expressions: ${references.reduce((total, reference) => total + reference.regExps.length, 0)}`)
}

await main()
process.exit(0)
