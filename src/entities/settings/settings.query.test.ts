import { describe, expect, mock, test } from 'bun:test'
import type { Settings, SettingsPatch } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { createTestQueryClient, renderHookWithProviders, waitFor } from '@shared/testing/render'

/**
 * `settings.ipc.ts` is a Tauri command binding that cannot load under `bun:test`, so it is stubbed
 * before `settings.query` is pulled in through a *dynamic* `import()`; `mock.module` is
 * process-global and last-registration-wins (`docs/memory/test-conventions.md` §3), so this fake
 * carries the module's whole export surface — including `emptySettingsPatch`, which callers outside
 * the query module import from here.
 *
 * The two mutations write the returned `Settings` straight into the cache and then invalidate only
 * the derived caches the patch can actually have changed (contract F1#3/F5#10). That selectivity is
 * the point of these tests: an unrelated field must not force a theme/locale/remote refetch, and a
 * field that *is* in one of the maps must.
 */
const SETTINGS_FROM_BACKEND = { themeId: 'dark-plus', language: 'ko' } as unknown as Settings

const capturedUpdateCalls: SettingsPatch[] = []
const capturedSetThemeIdCalls: string[] = []
const updateImpl = { current: () => Promise.resolve(SETTINGS_FROM_BACKEND) }

const EMPTY_PATCH_FIELDS = ['themeId', 'followSystemTheme', 'language', 'remoteAccessEnabled', 'editorFontSize'] as const

const emptyPatch = () => Object.fromEntries(EMPTY_PATCH_FIELDS.map((field) => [field, null])) as unknown as SettingsPatch

mock.module('@entities/settings/settings.ipc', () => ({
    emptySettingsPatch: emptyPatch,
    getSettings: () => Promise.resolve(SETTINGS_FROM_BACKEND),
    updateSettings: (patch: SettingsPatch) => {
        capturedUpdateCalls.push(patch)
        return updateImpl.current()
    },
    setThemeId: (themeId: string) => {
        capturedSetThemeIdCalls.push(themeId)
        return Promise.resolve(SETTINGS_FROM_BACKEND)
    },
}))

const importSettingsQuery = () => import('@entities/settings/settings.query')

/**
 * Seeds a derived cache entry with no observer so `isInvalidated` reports the mutation's decision
 * rather than a refetch. `gcTime` is overridden because the harness client collects observerless
 * queries immediately, and a test that awaits anything would then find the entry gone rather than
 * un-invalidated.
 */
const seedQuery = async (queryClient: ReturnType<typeof createTestQueryClient>, queryKey: readonly unknown[]) => {
    await queryClient.fetchQuery({ queryKey, queryFn: () => Promise.resolve('seed'), gcTime: Infinity })
}

const setupDerivedCaches = async () => {
    const queryClient = createTestQueryClient()
    await seedQuery(queryClient, QUERY_KEY.THEME.CURRENT)
    await seedQuery(queryClient, QUERY_KEY.LOCALE.CURRENT)
    await seedQuery(queryClient, QUERY_KEY.REMOTE.STATUS)
    return queryClient
}

const invalidationSnapshot = (queryClient: ReturnType<typeof createTestQueryClient>) => ({
    theme: queryClient.getQueryState(QUERY_KEY.THEME.CURRENT)?.isInvalidated,
    locale: queryClient.getQueryState(QUERY_KEY.LOCALE.CURRENT)?.isInvalidated,
    remote: queryClient.getQueryState(QUERY_KEY.REMOTE.STATUS)?.isInvalidated,
})

describe('settingsQueryOptions', () => {
    test('SETTINGS.CURRENT 키와 무한 staleTime 을 쓴다 (설정은 mutation 이 직접 캐시를 갱신한다)', async () => {
        const { settingsQueryOptions } = await importSettingsQuery()
        const options = settingsQueryOptions()

        expect([...options.queryKey]).toEqual([...QUERY_KEY.SETTINGS.CURRENT])
        expect(options.staleTime).toBe(Infinity)
        expect(await (options.queryFn as () => Promise<unknown>)()).toBe(SETTINGS_FROM_BACKEND)
    })
})

describe('useUpdateSettings', () => {
    test('성공 응답을 SETTINGS.CURRENT 캐시에 직접 써 넣는다 (재조회 없음)', async () => {
        const { useUpdateSettings } = await importSettingsQuery()
        const queryClient = await setupDerivedCaches()

        const { result } = renderHookWithProviders(() => useUpdateSettings(), { queryClient })
        await result.current.mutateAsync({ ...emptyPatch(), editorFontSize: 15 })

        expect(queryClient.getQueryData<Settings>(QUERY_KEY.SETTINGS.CURRENT)).toBe(SETTINGS_FROM_BACKEND)
        expect(capturedUpdateCalls.at(-1)?.editorFontSize).toBe(15)
    })

    test('테마와 무관한 필드만 바뀌면 파생 캐시를 하나도 무효화하지 않는다', async () => {
        const { useUpdateSettings } = await importSettingsQuery()
        const queryClient = await setupDerivedCaches()

        const { result } = renderHookWithProviders(() => useUpdateSettings(), { queryClient })
        await result.current.mutateAsync({ ...emptyPatch(), editorFontSize: 15 })

        expect(invalidationSnapshot(queryClient)).toEqual({ theme: false, locale: false, remote: false })
    })

    test('themeId 를 건드리면 THEME 만 무효화한다', async () => {
        const { useUpdateSettings } = await importSettingsQuery()
        const queryClient = await setupDerivedCaches()

        const { result } = renderHookWithProviders(() => useUpdateSettings(), { queryClient })
        await result.current.mutateAsync({ ...emptyPatch(), themeId: 'one-dark' })

        expect(invalidationSnapshot(queryClient)).toEqual({ theme: true, locale: false, remote: false })
    })

    test('followSystemTheme 도 테마 해석을 바꾸므로 THEME 을 무효화한다', async () => {
        const { useUpdateSettings } = await importSettingsQuery()
        const queryClient = await setupDerivedCaches()

        const { result } = renderHookWithProviders(() => useUpdateSettings(), { queryClient })
        await result.current.mutateAsync({ ...emptyPatch(), followSystemTheme: true })

        expect(invalidationSnapshot(queryClient)).toEqual({ theme: true, locale: false, remote: false })
    })

    test('language 는 LOCALE 만, remoteAccessEnabled 는 REMOTE.STATUS 만 무효화한다', async () => {
        const { useUpdateSettings } = await importSettingsQuery()
        const localeClient = await setupDerivedCaches()
        const remoteClient = await setupDerivedCaches()

        const locale = renderHookWithProviders(() => useUpdateSettings(), { queryClient: localeClient })
        await locale.result.current.mutateAsync({ ...emptyPatch(), language: 'ja' })
        const remote = renderHookWithProviders(() => useUpdateSettings(), { queryClient: remoteClient })
        await remote.result.current.mutateAsync({ ...emptyPatch(), remoteAccessEnabled: true })

        expect(invalidationSnapshot(localeClient)).toEqual({ theme: false, locale: true, remote: false })
        expect(invalidationSnapshot(remoteClient)).toEqual({ theme: false, locale: false, remote: true })
    })

    test('여러 축을 한 패치에 담으면 해당 파생 캐시를 모두 무효화한다', async () => {
        const { useUpdateSettings } = await importSettingsQuery()
        const queryClient = await setupDerivedCaches()

        const { result } = renderHookWithProviders(() => useUpdateSettings(), { queryClient })
        await result.current.mutateAsync({ ...emptyPatch(), themeId: 'one-dark', language: 'en', remoteAccessEnabled: false })

        expect(invalidationSnapshot(queryClient)).toEqual({ theme: true, locale: true, remote: true })
    })

    test('쓰기가 실패하면 캐시를 건드리지 않고 실패 상태로 남는다 (컨트롤이 옛 값으로 되돌아가는 이유를 토스트가 설명한다)', async () => {
        const { useUpdateSettings } = await importSettingsQuery()
        const queryClient = await setupDerivedCaches()
        const failure = new Error('settings.json write bounced')
        updateImpl.current = () => Promise.reject(failure) as ReturnType<typeof updateImpl.current>

        const { result } = renderHookWithProviders(() => useUpdateSettings(), { queryClient })
        result.current.mutate({ ...emptyPatch(), themeId: 'one-dark' })
        await waitFor(() => expect(result.current.isError).toBe(true))

        expect(queryClient.getQueryData(QUERY_KEY.SETTINGS.CURRENT)).toBeUndefined()
        expect(invalidationSnapshot(queryClient)).toEqual({ theme: false, locale: false, remote: false })

        updateImpl.current = () => Promise.resolve(SETTINGS_FROM_BACKEND)
    })
})

describe('useSetThemeId', () => {
    test('테마 id 를 그대로 넘기고, 응답을 캐시에 쓴 뒤 THEME 을 무효화한다', async () => {
        const { useSetThemeId } = await importSettingsQuery()
        const queryClient = await setupDerivedCaches()

        const { result } = renderHookWithProviders(() => useSetThemeId(), { queryClient })
        await result.current.mutateAsync('one-dark')

        expect(capturedSetThemeIdCalls.at(-1)).toBe('one-dark')
        expect(queryClient.getQueryData<Settings>(QUERY_KEY.SETTINGS.CURRENT)).toBe(SETTINGS_FROM_BACKEND)
        expect(invalidationSnapshot(queryClient)).toEqual({ theme: true, locale: false, remote: false })
    })
})
