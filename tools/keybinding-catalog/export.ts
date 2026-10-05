import { resolve } from 'node:path'
import ts from 'typescript'
import type { AppCommand } from '@shared/lib/command-registry'
import { APP_KEYMAP, parseKeymapOverrides, keymapEntryToEvent } from '@shared/lib/keymap/keymap'
import {
    buildKeybindingRows,
    buildKeybindingConflictIndex,
    findConflictingRowInIndex,
    filterKeybindingRowsByCapturedKey,
    isKeybindingRowUnassigned,
} from '@shared/lib/keymap/keybinding-catalog'

const ROOT = resolve(import.meta.dir, '../..')
const TARGET = 'native/taide-native-ui/src/keybinding-commands.json'
const META_KEYS = new Set(['id', 'titleKey', 'titleDefaultValue', 'categoryKey', 'keymapId', 'actionId', 'defaultLabel', 'defaultBindingLabel'])
const COMMAND_SOURCES = [
    ['src/shared/lib/command-catalog.ts', 'DEFAULT_COMMANDS'],
    ['src/entities/sync/sync.commands.ts', 'SYNC_COMMANDS'],
    ['src/shared/lib/monaco/monaco-actions.ts', 'MONACO_ACTIONS'],
    ['src/entities/agent/agent.commands.ts', 'AGENT_CLI_COMMANDS'],
    ['src/entities/ai/ai.commands.ts', 'AI_COMMANDS'],
    ['src/entities/git/git.commands.ts', 'GIT_COMMANDS'],
    ['src/entities/terminal/terminal.commands.ts', 'TERMINAL_COMMANDS'],
    ['src/entities/task/task.commands.ts', 'TASK_COMMANDS'],
] as const

type CommandMetadata = Pick<AppCommand, 'id' | 'titleKey' | 'titleDefaultValue' | 'categoryKey'> & {
    keymapId?: string
    defaultBindingLabel?: string | null
    platform?: 'mac'
}

const parseSource = async (path: string) => ts.createSourceFile(path, await Bun.file(resolve(ROOT, path)).text(), ts.ScriptTarget.Latest, true)

const registrationOrder = (await parseSource('src/app/bootstrap-commands.ts')).statements.flatMap((statement) => {
    if (!ts.isExpressionStatement(statement) || !ts.isCallExpression(statement.expression)) return []
    const call = statement.expression
    if (!ts.isIdentifier(call.expression) || call.expression.text !== 'registerCommands') return []
    const argument = call.arguments[0]
    if (call.arguments.length !== 1 || !argument || !ts.isIdentifier(argument)) throw new Error('Unsupported command registration')
    return [argument.text]
})
const expectedOrder = COMMAND_SOURCES.map(([, name]) => (name === 'MONACO_ACTIONS' ? 'MONACO_ACTION_COMMANDS' : name))
if (JSON.stringify(registrationOrder) !== JSON.stringify(expectedOrder)) throw new Error('Original bootstrap registration order changed')

const declarations = (source: ts.SourceFile) =>
    new Map(
        source.statements.flatMap((statement) => {
            if (!ts.isVariableStatement(statement)) return []
            return statement.declarationList.declarations.flatMap((declaration) => {
                if (!ts.isIdentifier(declaration.name) || !declaration.initializer) return []
                return [[declaration.name.text, declaration.initializer] as const]
            })
        }),
    )

const categories = declarations(await parseSource('src/shared/lib/keymap/keymap-category.ts')).get('KEYMAP_CATEGORY')
if (!categories || !ts.isAsExpression(categories) || !ts.isObjectLiteralExpression(categories.expression)) throw new Error('Missing categories')
const categoryValues = new Map<string, string>(
    categories.expression.properties.map((property) => {
        if (!ts.isPropertyAssignment(property) || !ts.isIdentifier(property.name) || !ts.isStringLiteral(property.initializer)) {
            throw new Error('Nonliteral category')
        }
        return [property.name.text, property.initializer.text]
    }),
)

const commands = (
    await Promise.all(
        COMMAND_SOURCES.map(async ([path, name]) => {
            const definitions = declarations(await parseSource(path))
            const declaration = definitions.get(name)
            if (!declaration) throw new Error(`Missing ${name}`)
            if (
                ts.isConditionalExpression(declaration) &&
                (!ts.isIdentifier(declaration.condition) ||
                    declaration.condition.text !== 'IS_MAC' ||
                    !ts.isArrayLiteralExpression(declaration.whenFalse) ||
                    declaration.whenFalse.elements.length !== 0)
            ) {
                throw new Error('Unsupported platform catalog condition')
            }
            const array = ts.isConditionalExpression(declaration) ? declaration.whenTrue : declaration
            if (!ts.isArrayLiteralExpression(array)) throw new Error(`Nonliteral catalog ${name}`)

            const readRows = (rows: ts.ArrayLiteralExpression): CommandMetadata[] =>
                rows.elements.flatMap((element) => {
                    if (ts.isSpreadElement(element) && ts.isIdentifier(element.expression)) {
                        const spread = definitions.get(element.expression.text)
                        if (!spread || !ts.isArrayLiteralExpression(spread)) throw new Error('Nonliteral spread')
                        return readRows(spread)
                    }
                    if (!ts.isObjectLiteralExpression(element)) throw new Error('Nonliteral row')
                    const metadata = new Map<string, string | null>(
                        element.properties.flatMap<[string, string | null]>((property) => {
                            if (!ts.isPropertyAssignment(property) || !ts.isIdentifier(property.name)) throw new Error('Unsupported property')
                            if (!META_KEYS.has(property.name.text)) return []
                            const initializer = property.initializer
                            if (ts.isStringLiteral(initializer)) return [[property.name.text, initializer.text]]
                            if (initializer.kind === ts.SyntaxKind.NullKeyword) return [[property.name.text, null]]
                            if (ts.isIdentifier(initializer)) {
                                const literal = definitions.get(initializer.text)
                                if (literal && ts.isStringLiteral(literal)) return [[property.name.text, literal.text]]
                            }
                            if (
                                ts.isPropertyAccessExpression(initializer) &&
                                ts.isIdentifier(initializer.expression) &&
                                initializer.expression.text === 'KEYMAP_CATEGORY'
                            ) {
                                const category = categoryValues.get(initializer.name.text)
                                if (category) return [[property.name.text, category]]
                            }
                            throw new Error(`Unsupported metadata ${property.name.text}`)
                        }),
                    )
                    if (name === 'MONACO_ACTIONS') {
                        const actionId = metadata.get('actionId')
                        const defaultLabel = metadata.get('defaultLabel')
                        if (typeof actionId !== 'string' || typeof defaultLabel !== 'string') throw new Error('Invalid Monaco metadata')
                        return [
                            {
                                id: `monaco.${actionId}`,
                                titleKey: `keymap.monaco.${actionId}`,
                                titleDefaultValue: defaultLabel,
                                categoryKey: metadata.get('categoryKey') ?? undefined,
                                defaultBindingLabel: metadata.get('defaultBindingLabel'),
                            },
                        ]
                    }
                    const id = metadata.get('id')
                    const titleKey = metadata.get('titleKey')
                    if (typeof id !== 'string' || typeof titleKey !== 'string') throw new Error('Invalid command metadata')
                    return [
                        {
                            id,
                            titleKey,
                            ...(metadata.has('titleDefaultValue') ? { titleDefaultValue: metadata.get('titleDefaultValue') ?? undefined } : {}),
                            ...(metadata.has('categoryKey') ? { categoryKey: metadata.get('categoryKey') ?? undefined } : {}),
                            ...(metadata.has('keymapId') ? { keymapId: metadata.get('keymapId') ?? undefined } : {}),
                            ...(name === 'AGENT_CLI_COMMANDS' ? { platform: 'mac' as const } : {}),
                        },
                    ]
                })

            return readRows(array)
        }),
    )
).flat()

if (new Set(commands.map((command) => command.id)).size !== commands.length) throw new Error('Duplicate command id')
const serialized = `${JSON.stringify(commands, null, 4)}\n`
if (process.argv.includes('--fixture') || process.argv.includes('--check-fixture')) {
    const scenarios = [
        { name: 'default-mac', mac: true, json: null },
        { name: 'default-non-mac', mac: false, json: null },
        {
            name: 'legacy-first-invalid-chord-and-future-fields',
            mac: true,
            json: '[{"actionId":"keybindings.open","key":"j","mods":["mod"],"chord":{"key":"k","mods":null},"future":42},{"actionId":"open-keybindings-editor","key":"u","mods":["mod"]},{"actionId":"unknown.future","key":"f","mods":["ctrl"]}]',
        },
        {
            name: 'scope-and-shared-prefix',
            mac: true,
            json: '[{"actionId":"open-keybindings-editor","key":"k","mods":["mod"],"chord":{"key":"x","mods":[]}},{"actionId":"toggle-zen-mode","key":"k","mods":["mod"],"chord":{"key":"y","mods":[]}},{"actionId":"new-terminal","key":"k","mods":["mod"]},{"actionId":"terminal-jump-to-previous-command","key":"ArrowUp","mods":["mod"]}]',
        },
        {
            name: 'monaco-unbind-and-command-chord',
            mac: true,
            json: '[{"actionId":"monaco.editor.action.addSelectionToNextFindMatch","key":"","mods":[]},{"actionId":"monaco.editor.action.commentLine","key":"k","mods":["mod"],"chord":{"key":"s","mods":["mod"]}},{"actionId":"sync.uploadNow","key":"p","mods":["mod"]},{"actionId":"sync.downloadNow","key":"p","mods":["mod"],"chord":{"key":"s","mods":["mod"]}}]',
        },
        {
            name: 'non-mac-control-space-and-chord-gate-removal',
            mac: false,
            json: '[{"actionId":"open-keybindings-editor","key":"j","mods":["ctrl"]},{"actionId":"toggle-zen-mode","key":"j","mods":["mod"],"chord":{"key":"z","mods":[]}},{"actionId":"sync.uploadNow","key":" ","mods":["ctrl"]},{"actionId":"sync.downloadNow","key":"space","mods":["mod"]}]',
        },
    ]
    const fixture = scenarios.map((scenario) => {
        const originalCommands = commands
            .filter((command) => scenario.mac || command.platform !== 'mac')
            .map((command): AppCommand => {
                const keymapId = APP_KEYMAP.find((entry) => entry.id === command.keymapId)?.id
                if (command.keymapId && !keymapId) throw new Error('Unknown command keymap')
                return { ...command, keymapId, run: () => {} }
            })
        const overrides = parseKeymapOverrides(scenario.json)
        const rows = buildKeybindingRows(originalCommands, overrides)
        const index = buildKeybindingConflictIndex(rows, scenario.mac)
        const filters = [
            { key: 'k', mods: ['mod'] },
            { key: 'j', mods: ['ctrl'] },
            { key: ' ', mods: ['mod'] },
            { key: 'ArrowUp', mods: ['mod'] },
        ] satisfies Parameters<typeof keymapEntryToEvent>[0][]
        return {
            ...scenario,
            rows,
            conflicts: rows.map((row) => findConflictingRowInIndex(index, row)?.id ?? null),
            unassigned: rows.filter(isKeybindingRowUnassigned).map((row) => row.id),
            filters: filters.map((filter) => ({
                ...filter,
                ids: filterKeybindingRowsByCapturedKey(rows, filter.key, filter.mods, scenario.mac).map((row) => row.id),
            })),
            overrides,
        }
    })
    const fixtureJson = `${JSON.stringify(fixture)}\n`
    if (process.argv.includes('--check-fixture')) {
        const stored = await Bun.file(resolve(ROOT, 'native/taide-native-app/tests/fixtures/keybinding-catalog.json')).text()
        if (stored !== fixtureJson) throw new Error('Native fixture differs from original catalog functions')
        process.stdout.write(`PASS: ${fixture.length} complete original catalog scenarios\n`)
    } else {
        process.stdout.write(fixtureJson)
    }
} else if (process.argv.includes('--check')) {
    const stored = await Bun.file(resolve(ROOT, TARGET)).text()
    if (stored !== serialized) throw new Error('Native command metadata differs from original registration order or metadata')
    process.stdout.write(`PASS: ${commands.length} command metadata entries, complete registration order\n`)
} else {
    process.stdout.write(serialized)
}
