import { describe, expect, mock, test } from 'bun:test'
import type { LanguageRegistration } from '@shikijs/core'
import type { LoadedPlugin } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { createTestQueryClient, renderHookWithProviders } from '@shared/testing/render'

/**
 * `plugin.query.ts` reaches three seams that need a real browser: the entity's Tauri bindings, the
 * monaco language registry, and the shiki highlighter. All three are stubbed before `plugin.query`
 * is pulled in through a *dynamic* `import()`, each covering its module's whole export surface
 * because `mock.module` is process-global and last-registration-wins
 * (`docs/memory/test-conventions.md` §3). `plugin-grammar.ts` is left real — it is pure, and it is
 * what turns a plugin list into the shiki registrations these tests assert on.
 *
 * The behaviour under test is the three-step `applyPluginList` order (cache → monaco languages →
 * shiki rebuild) and, above all, which mutations have to re-fetch the list: `plugin_install`
 * answers with the *one* new plugin, so applying its response directly would drop every other
 * installed plugin's languages out of the editor.
 */
const capturedRegisterCalls: LoadedPlugin[][] = []
const capturedReinitCalls: LanguageRegistration[][] = []
const capturedGrammarReads: { pluginId: string; languageId: string }[] = []
const capturedInstallCalls: string[] = []
const capturedUninstallCalls: string[] = []

const listPluginsResult = { current: [] as LoadedPlugin[] }
const uninstallResult = { current: [] as LoadedPlugin[] }
const reloadResult = { current: [] as LoadedPlugin[] }
let listPluginsCallCount = 0

mock.module('@entities/plugin/plugin.ipc', () => ({
    listPlugins: () => {
        listPluginsCallCount += 1
        return Promise.resolve(listPluginsResult.current)
    },
    reloadPlugins: () => Promise.resolve(reloadResult.current),
    installPlugin: (sourcePath: string) => {
        capturedInstallCalls.push(sourcePath)
        return Promise.resolve(listPluginsResult.current[0] ?? null)
    },
    uninstallPlugin: (pluginId: string) => {
        capturedUninstallCalls.push(pluginId)
        return Promise.resolve(uninstallResult.current)
    },
    readPluginGrammar: (pluginId: string, languageId: string) => {
        capturedGrammarReads.push({ pluginId, languageId })
        return Promise.resolve(JSON.stringify({ scopeName: `source.${languageId}`, patterns: [], repository: {} }))
    },
}))

mock.module('@shared/lib/monaco/register-plugin-languages', () => ({
    registerPluginLanguages: (plugins: LoadedPlugin[]) => capturedRegisterCalls.push(plugins),
}))

mock.module('@shared/lib/shiki/shiki-monaco', () => ({
    ensureShikiLanguage: () => Promise.resolve(undefined),
    initShiki: () => Promise.resolve(undefined),
    applyShikiTheme: () => undefined,
    reinitShiki: (grammars: LanguageRegistration[]) => {
        capturedReinitCalls.push(grammars)
        return Promise.resolve(undefined)
    },
}))

const importPluginQuery = () => import('@entities/plugin/plugin.query')

const buildPlugin = (id: string, { enabled = true, languageId }: { enabled?: boolean; languageId?: string } = {}): LoadedPlugin => ({
    manifest: {
        manifestVersion: 1,
        id,
        name: id,
        version: '1.0.0',
        contributes: languageId ? { languages: [{ id: languageId, extensions: [`.${languageId}`], grammar: 'grammar.json' }] } : undefined,
    },
    root: `/plugins/${id}`,
    enabled,
})

describe('pluginListQueryOptions', () => {
    test('PLUGIN.LIST 키로 목록을 조회한다', async () => {
        const { pluginListQueryOptions } = await importPluginQuery()
        listPluginsResult.current = [buildPlugin('alpha')]

        const options = pluginListQueryOptions()

        expect([...options.queryKey]).toEqual([...QUERY_KEY.PLUGIN.LIST])
        expect(await (options.queryFn as () => Promise<unknown>)()).toEqual(listPluginsResult.current)
    })
})

describe('useReloadPlugins', () => {
    test('응답 목록을 캐시에 쓰고 monaco 언어 등록과 shiki 재초기화를 이어서 수행한다', async () => {
        const { useReloadPlugins } = await importPluginQuery()
        const plugins = [buildPlugin('alpha', { languageId: 'alpha-lang' })]
        reloadResult.current = plugins
        const queryClient = createTestQueryClient()

        const { result } = renderHookWithProviders(() => useReloadPlugins(), { queryClient })
        await result.current.mutateAsync()

        expect(queryClient.getQueryData<LoadedPlugin[]>(QUERY_KEY.PLUGIN.LIST)).toEqual(plugins)
        expect(capturedRegisterCalls.at(-1)).toEqual(plugins)
        expect(capturedReinitCalls.at(-1)?.map((registration) => registration.name)).toEqual(['alpha-lang'])
    })

    test('비활성 플러그인의 문법은 shiki 에 넘기지 않는다', async () => {
        const { useReloadPlugins } = await importPluginQuery()
        reloadResult.current = [buildPlugin('alpha', { languageId: 'alpha-lang', enabled: false })]
        const queryClient = createTestQueryClient()

        const { result } = renderHookWithProviders(() => useReloadPlugins(), { queryClient })
        await result.current.mutateAsync()

        expect(capturedReinitCalls.at(-1)).toEqual([])
    })
})

describe('useInstallPlugin', () => {
    test('설치 응답(새 플러그인 1개)을 그대로 쓰지 않고 목록을 다시 받아 적용한다', async () => {
        const { useInstallPlugin } = await importPluginQuery()
        const full = [buildPlugin('alpha', { languageId: 'alpha-lang' }), buildPlugin('beta', { languageId: 'beta-lang' })]
        listPluginsResult.current = full
        const queryClient = createTestQueryClient()
        const before = listPluginsCallCount

        const { result } = renderHookWithProviders(() => useInstallPlugin(), { queryClient })
        await result.current.mutateAsync('/downloads/beta.vsix')

        expect(capturedInstallCalls.at(-1)).toBe('/downloads/beta.vsix')
        expect(listPluginsCallCount).toBe(before + 1)
        expect(queryClient.getQueryData<LoadedPlugin[]>(QUERY_KEY.PLUGIN.LIST)).toEqual(full)
        expect(capturedReinitCalls.at(-1)?.map((registration) => registration.name)).toEqual(['alpha-lang', 'beta-lang'])
    })
})

describe('useUninstallPlugin', () => {
    test('제거 응답이 이미 갱신된 목록이므로 다시 조회하지 않는다', async () => {
        const { useUninstallPlugin } = await importPluginQuery()
        const remaining = [buildPlugin('alpha', { languageId: 'alpha-lang' })]
        uninstallResult.current = remaining
        const queryClient = createTestQueryClient()
        const before = listPluginsCallCount

        const { result } = renderHookWithProviders(() => useUninstallPlugin(), { queryClient })
        await result.current.mutateAsync('beta')

        expect(capturedUninstallCalls.at(-1)).toBe('beta')
        expect(listPluginsCallCount).toBe(before)
        expect(queryClient.getQueryData<LoadedPlugin[]>(QUERY_KEY.PLUGIN.LIST)).toEqual(remaining)
    })
})

describe('refetchAndApplyPluginList', () => {
    test('목록을 다시 걷어 캐시·monaco·shiki 를 한 번에 맞춘다', async () => {
        const { refetchAndApplyPluginList } = await importPluginQuery()
        const plugins = [buildPlugin('alpha', { languageId: 'alpha-lang' })]
        listPluginsResult.current = plugins
        const queryClient = createTestQueryClient()

        await refetchAndApplyPluginList(queryClient)

        expect(queryClient.getQueryData<LoadedPlugin[]>(QUERY_KEY.PLUGIN.LIST)).toEqual(plugins)
        expect(capturedGrammarReads.at(-1)).toEqual({ pluginId: 'alpha', languageId: 'alpha-lang' })
    })
})
