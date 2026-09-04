import type { SearchFileMatches } from '@shared/api/bindings'
import { appendSearchFileMatches, createSearchResultAccumulator } from '@entities/search/search-result'
import type { AppCommand } from '@shared/lib/command-registry'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import { buildKeybindingConflictIndex, buildKeybindingRows, findConflictingRowInIndex } from '@shared/lib/keymap/keybinding-catalog'
import type { KeymapModifier, KeymapOverrideEntry } from '@shared/lib/keymap/keymap'
import { APP_KEYMAP, findMatchingKeymapEntry, keymapEntryToEvent } from '@shared/lib/keymap/keymap'

/**
 * Wall-clock benchmarks for the front end's hot pure functions. **Run by hand**
 * (`bun run bench`) — never from `bun test` or CI, where a shared runner's timing noise turns a
 * threshold into a flaky failure. The regression guards that *do* run in CI count operations
 * instead (`src/shared/lib/perf-budget.test.ts`).
 *
 * Procedure and how to read the numbers: `docs/quality-assurance/2026-09-04-perf-baseline.md` §4.
 */
const WARMUP_ROUNDS = 3
const MEASURED_ROUNDS = 10
const MS_PER_SECOND = 1000
const REPORT_DECIMALS = 3

const FUZZY_CORPUS_SIZES = [5_000, 20_000, 50_000]
const FUZZY_QUERY = 'srcpnl'
const SEARCH_BATCH_COUNT = 2_000
const SEARCH_MATCHES_PER_BATCH = 5
const KEYBINDING_COMMAND_COUNT = 200
const BENCH_BINDING_KEYS = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'F1', 'F2', 'Enter', 'Escape']
const BENCH_BINDING_MODS: KeymapModifier[][] = [['mod'], ['mod', 'shift'], ['alt'], ['ctrl', 'alt']]

const PATH_DIRECTORIES = ['src', 'widgets', 'features', 'entities', 'shared', 'lib', 'ui', 'hooks', 'panel', 'view', 'model', 'store', 'util', 'test']
const PATH_STEMS = ['editor', 'terminal', 'explorer', 'search', 'git', 'settings', 'layout', 'project', 'theme', 'command', 'palette', 'tree']
const PATH_EXTENSIONS = ['.ts', '.tsx', '.rs', '.json', '.md', '.css']

const LCG_MULTIPLIER = 1_664_525
const LCG_INCREMENT = 1_013_904_223
const LCG_MODULUS = 2 ** 32
const CORPUS_SEED = 20260904

/** Deterministic corpus generator — the same seed must produce the same paths across runs, or two measurements are not comparable. */
const createPseudoRandom = (seed: number) => {
    let state = seed
    return () => {
        state = (state * LCG_MULTIPLIER + LCG_INCREMENT) % LCG_MODULUS
        return state / LCG_MODULUS
    }
}

const buildPathCorpus = (size: number) => {
    const random = createPseudoRandom(CORPUS_SEED)
    const pick = <T>(pool: T[]) => pool[Math.floor(random() * pool.length)]
    return Array.from({ length: size }, (_, index) => {
        const depth = 2 + Math.floor(random() * 3)
        const directories = Array.from({ length: depth }, () => pick(PATH_DIRECTORIES))
        return `${directories.join('/')}/${pick(PATH_STEMS)}-${index}${pick(PATH_EXTENSIONS)}`
    })
}

const buildSearchBatches = (batchCount: number, matchesPerBatch: number): SearchFileMatches[] =>
    Array.from({ length: batchCount }, (_, batchIndex) => ({
        path: `/repo/src/module-${batchIndex}/file-${batchIndex}.ts`,
        matches: Array.from({ length: matchesPerBatch }, (_, matchIndex) => ({
            line: matchIndex + 1,
            column: 1,
            preview: `const value${matchIndex} = createSearchResultAccumulator()`,
            matchStart: 6,
            matchEnd: 11,
            before: [],
            after: [],
        })),
    }))

type BenchResult = { name: string; rounds: number; msPerRound: number; opsPerSecond: number }

const runBenchCase = (name: string, operation: () => unknown): BenchResult => {
    for (let round = 0; round < WARMUP_ROUNDS; round += 1) operation()

    const startedAt = performance.now()
    for (let round = 0; round < MEASURED_ROUNDS; round += 1) operation()
    const elapsedMs = performance.now() - startedAt

    const msPerRound = elapsedMs / MEASURED_ROUNDS
    return {
        name,
        rounds: MEASURED_ROUNDS,
        msPerRound: Number(msPerRound.toFixed(REPORT_DECIMALS)),
        opsPerSecond: Math.round(MS_PER_SECOND / msPerRound),
    }
}

const fuzzyCases = FUZZY_CORPUS_SIZES.map((size) => {
    const corpus = buildPathCorpus(size)
    return { name: `fuzzyFilter · ${size.toLocaleString('en-US')} paths`, operation: () => fuzzyFilter(FUZZY_QUERY, corpus, (path) => path) }
})

const keymapEvent = keymapEntryToEvent(APP_KEYMAP[APP_KEYMAP.length - 1])
const keymapCase = {
    name: `findMatchingKeymapEntry · ${APP_KEYMAP.length} entries (worst case)`,
    operation: () => findMatchingKeymapEntry(APP_KEYMAP, keymapEvent),
}

const searchBatches = buildSearchBatches(SEARCH_BATCH_COUNT, SEARCH_MATCHES_PER_BATCH)
const searchCase = {
    name: `appendSearchFileMatches · ${SEARCH_BATCH_COUNT.toLocaleString('en-US')} batches × ${SEARCH_MATCHES_PER_BATCH}`,
    operation: () => appendSearchFileMatches(createSearchResultAccumulator(), searchBatches),
}

/**
 * Approximates the keybindings editor's real catalog size (app commands + monaco mirrors, ~200
 * rows) with bound rows: a command carries no key of its own, so each synthetic command is given
 * one through an override, drawn from a pool small enough that several rows share a binding — the
 * collision buckets are what `findConflictingRowInIndex` actually walks.
 */
const buildKeybindingCorpus = (size: number) => {
    const commands: AppCommand[] = Array.from({ length: size }, (_, index) => ({
        id: `bench.command-${index}`,
        titleKey: 'bench.command',
        run: () => {},
    }))
    const overrides: KeymapOverrideEntry[] = commands.map((command, index) => ({
        actionId: command.id,
        key: BENCH_BINDING_KEYS[index % BENCH_BINDING_KEYS.length],
        mods: BENCH_BINDING_MODS[index % BENCH_BINDING_MODS.length],
    }))
    return buildKeybindingRows(commands, overrides)
}

const keybindingRows = buildKeybindingCorpus(KEYBINDING_COMMAND_COUNT)
const keybindingCase = {
    name: `keybinding conflicts · index + ${keybindingRows.length} lookups`,
    operation: () => {
        const index = buildKeybindingConflictIndex(keybindingRows)
        return keybindingRows.map((row) => findConflictingRowInIndex(index, row))
    },
}

const results = [...fuzzyCases, keymapCase, searchCase, keybindingCase].map(({ name, operation }) => runBenchCase(name, operation))

console.info(`bench-frontend · warmup ${WARMUP_ROUNDS} · measured ${MEASURED_ROUNDS} rounds · bun ${Bun.version}`)
console.table(results)
