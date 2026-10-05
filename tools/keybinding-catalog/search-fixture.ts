import { resolve } from 'node:path'
import type { AppCommand } from '@shared/lib/command-registry'
import { fuzzyFilter, fuzzyMatch } from '@shared/lib/fuzzy-match'
import { APP_KEYMAP } from '@shared/lib/keymap/keymap'
import { buildKeybindingRows, sortKeybindingRows } from '@shared/lib/keymap/keybinding-catalog'

const ROOT = resolve(import.meta.dir, '../..')
const TARGET = 'native/taide-native-app/tests/fixtures/keybinding-search.json'
const parsed: unknown = await Bun.file(resolve(ROOT, 'native/taide-native-ui/src/keybinding-commands.json')).json()
if (!Array.isArray(parsed)) throw new Error('Invalid command metadata')
const commands = parsed.map((command: unknown): AppCommand => {
    if (typeof command !== 'object' || command === null || !('id' in command) || typeof command.id !== 'string') throw new Error('Invalid command id')
    if (!('titleKey' in command) || typeof command.titleKey !== 'string') throw new Error('Invalid command title')
    const keymapId = 'keymapId' in command ? APP_KEYMAP.find((entry) => entry.id === command.keymapId)?.id : undefined
    if ('keymapId' in command && !keymapId) throw new Error('Invalid command keymap')
    return {
        id: command.id,
        titleKey: command.titleKey,
        keymapId,
        categoryKey: 'categoryKey' in command && typeof command.categoryKey === 'string' ? command.categoryKey : undefined,
        titleDefaultValue: 'titleDefaultValue' in command && typeof command.titleDefaultValue === 'string' ? command.titleDefaultValue : undefined,
        run: () => {},
    }
})
const rows = buildKeybindingRows(commands, [])
const raw = [
    ['abc', 'AxbYC'],
    ['abc', 'ABC'],
    ['', ''],
    ['z', 'abc'],
    ['İ', 'xİ'],
    ['i', 'İ'],
    ['\u{1d518}', 'A\u{1d518}B'],
    ['Σ', 'xΣ'],
    ['SS', 'ß'],
    ['한', '\u1112\u1161\u11ab'],
].map(([query, target]) => ({ query, target, matched: fuzzyMatch(query, target) }))

const labels = [
    'src/foo.ts',
    'foo/src.ts',
    'foo.ts',
    'foo.ts',
    'FOO.ts',
    'fOO.ts',
    'src/panel/search.ts',
    'search/panel.ts',
    'Search Panel',
    '\u1112\u1161\u11ab글',
    '한글',
    'é.ts',
    'e\u0301.ts',
    '\u{1d518}b',
    'a b c d e f g h',
    'x\u0085y',
    'X Y',
    '日本語',
]
const queries = ['', '  ', 'foo', 'panel search', '한', 'é', '\u{1d518}', 'a b c d e f g h missing', 'x\u0085y', 'x\ufeffy', '日本']
const fuzzy = queries.map((query) => ({ query, ranked: fuzzyFilter(query, labels, (label) => label) }))
const catalogs = await Promise.all(
    ['en', 'ko', 'ja'].map(async (language) => {
        const pack: unknown = await Bun.file(resolve(ROOT, `crates/taide-locale/resources/locales/${language}.json`)).json()
        if (typeof pack !== 'object' || pack === null || Array.isArray(pack)) throw new Error('Invalid locale pack')
        const messages = new Map(
            Object.entries(pack).map(([key, value]) => {
                if (typeof value !== 'string') throw new Error('Invalid message')
                return [key, value] as const
            }),
        )
        const label = (row: (typeof rows)[number]) => {
            const title = messages.get(row.titleKey) ?? row.titleDefaultValue ?? row.titleKey
            return row.categoryKey ? `${messages.get(row.categoryKey) ?? row.categoryKey}: ${title}` : title
        }
        const sorted = sortKeybindingRows(rows, label)
        const queries = ['', 'save', 'editor monaco', 'terminal', '키맵', 'エディター', 'Ctrl']
        return {
            language,
            sorted: sorted.map((row) => row.id),
            labels: rows.map((row) => ({ id: row.id, key: row.key, label: label(row) })),
            queries: queries.map((query) => ({
                query,
                ids: fuzzyFilter(query, sorted, (row) => `${label(row)} ${row.id}`).map((result) => result.item.id),
            })),
        }
    }),
)
const fixture = { locale: new Intl.Collator().resolvedOptions().locale, raw, labels, fuzzy, catalogs }
const serialized = `${JSON.stringify(fixture)}\n`
if (process.argv.includes('--check')) {
    const stored = await Bun.file(resolve(ROOT, TARGET)).text()
    if (stored !== serialized) throw new Error('Search fixture differs from original TS functions')
    process.stdout.write(`PASS: original fuzzy functions and ${catalogs.length} complete localized catalogs\n`)
} else {
    process.stdout.write(serialized)
}
