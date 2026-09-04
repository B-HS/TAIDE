import { describe, expect, mock, test } from 'bun:test'
import type { ResolvedTheme, Theme } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import type { ThemeDraft } from '@shared/lib/theme-draft'
import { act, createTestQueryClient, renderHookWithProviders } from '@shared/testing/render'

/**
 * `theme.ipc.ts` is a Tauri command binding that cannot load under `bun:test`, so it is stubbed
 * before `theme.query` is pulled in through a *dynamic* `import()`; `mock.module` is process-global
 * and last-registration-wins (`docs/memory/test-conventions.md` §3), so this fake carries the whole
 * export surface.
 *
 * Beyond the query factories, the live-preview store is what these tests are really for: it must
 * stay *out* of `QUERY_KEY.THEME.CURRENT` (a draft is client state, not what `theme_get_current`
 * resolved — contract F1#11), it must collapse a drag's worth of pushes into one publish per frame,
 * and `clearPreview` must cancel a frame that has already been scheduled.
 */
const capturedGetCurrentThemeCalls: string[] = []
const capturedGetThemeCalls: string[] = []
const capturedSaveCalls: Theme[] = []
const capturedDeleteCalls: string[] = []

mock.module('@entities/theme/theme.ipc', () => ({
    listThemes: () => Promise.resolve([]),
    getTheme: (themeId: string) => {
        capturedGetThemeCalls.push(themeId)
        return Promise.resolve(null)
    },
    getCurrentTheme: (systemTheme: string) => {
        capturedGetCurrentThemeCalls.push(systemTheme)
        return Promise.resolve(null)
    },
    saveTheme: (theme: Theme) => {
        capturedSaveCalls.push(theme)
        return Promise.resolve(undefined)
    },
    deleteTheme: (themeId: string) => {
        capturedDeleteCalls.push(themeId)
        return Promise.resolve(undefined)
    },
}))

const importThemeQuery = () => import('@entities/theme/theme.query')

const EMPTY_VALUES = { colors: {}, syntax: {}, terminal: {} }

const buildDraft = (overrides: Partial<ThemeDraft> = {}): ThemeDraft => ({
    id: 'draft-theme',
    name: 'Draft Theme',
    themeType: 'dark',
    extendsId: 'base-theme',
    base: EMPTY_VALUES,
    current: EMPTY_VALUES,
    metadata: { tokenColors: null, author: null, license: null, source: null },
    ...overrides,
})

const BASE_THEME = {
    id: 'base-theme',
    name: 'Base',
    type: 'dark',
    colors: {},
    syntax: {},
    terminal: {},
    tokenColors: [{ scope: 'keyword', settings: { foreground: '#ff0000' } }],
    syntaxOverrides: [],
    warnings: [],
    author: null,
    license: null,
    source: null,
} as unknown as ResolvedTheme

const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))

const seedQuery = async (queryClient: ReturnType<typeof createTestQueryClient>, queryKey: readonly unknown[]) => {
    await queryClient.fetchQuery({ queryKey, queryFn: () => Promise.resolve('seed'), gcTime: Infinity })
}

describe('theme queryOptions 팩토리', () => {
    test('목록·상세·현재 테마가 각자의 THEME 키를 쓴다', async () => {
        const { currentThemeQueryOptions, themeListQueryOptions, themeQueryOptions } = await importThemeQuery()

        expect([...themeListQueryOptions().queryKey]).toEqual([...QUERY_KEY.THEME.LIST])
        expect([...currentThemeQueryOptions().queryKey]).toEqual([...QUERY_KEY.THEME.CURRENT])
        expect([...themeQueryOptions('one-dark').queryKey]).toEqual([...QUERY_KEY.THEME.DETAIL('one-dark')])
    })

    test('현재 테마는 OS 외형을 함께 넘기고 staleTime 이 무한이다 (변경은 mutation·이벤트가 무효화한다)', async () => {
        const { currentThemeQueryOptions } = await importThemeQuery()
        const options = currentThemeQueryOptions()

        await (options.queryFn as () => Promise<unknown>)()

        expect(options.staleTime).toBe(Infinity)
        expect(['dark', 'light'].includes(capturedGetCurrentThemeCalls.at(-1) ?? '')).toBe(true)
    })

    test('상세 조회는 themeId 를 그대로 IPC 에 넘긴다', async () => {
        const { themeQueryOptions } = await importThemeQuery()

        await (themeQueryOptions('one-dark').queryFn as () => Promise<unknown>)()

        expect(capturedGetThemeCalls.at(-1)).toBe('one-dark')
    })
})

describe('useSaveTheme · useDeleteTheme', () => {
    test('저장 성공 시 THEME 계열 전체를 무효화한다 (목록·현재·상세가 같이 바뀐다)', async () => {
        const { useSaveTheme } = await importThemeQuery()
        const queryClient = createTestQueryClient()
        await seedQuery(queryClient, QUERY_KEY.THEME.LIST)
        await seedQuery(queryClient, QUERY_KEY.THEME.CURRENT)
        await seedQuery(queryClient, QUERY_KEY.SETTINGS.CURRENT)

        const { result } = renderHookWithProviders(() => useSaveTheme(), { queryClient })
        await result.current.mutateAsync({ id: 'one-dark' } as unknown as Theme)

        expect(capturedSaveCalls.at(-1)).toEqual({ id: 'one-dark' } as unknown as Theme)
        expect(queryClient.getQueryState(QUERY_KEY.THEME.LIST)?.isInvalidated).toBe(true)
        expect(queryClient.getQueryState(QUERY_KEY.THEME.CURRENT)?.isInvalidated).toBe(true)
        expect(queryClient.getQueryState(QUERY_KEY.SETTINGS.CURRENT)?.isInvalidated).toBe(false)
    })

    test('삭제 성공 시에도 THEME 계열을 무효화한다', async () => {
        const { useDeleteTheme } = await importThemeQuery()
        const queryClient = createTestQueryClient()
        await seedQuery(queryClient, QUERY_KEY.THEME.LIST)

        const { result } = renderHookWithProviders(() => useDeleteTheme(), { queryClient })
        await result.current.mutateAsync('one-dark')

        expect(capturedDeleteCalls.at(-1)).toBe('one-dark')
        expect(queryClient.getQueryState(QUERY_KEY.THEME.LIST)?.isInvalidated).toBe(true)
    })
})

describe('useThemePreview', () => {
    test('push 한 draft 는 다음 프레임에 미리보기 스토어로 나오고 THEME.CURRENT 캐시는 건드리지 않는다', async () => {
        const { useThemePreview, useThemePreviewValue } = await importThemeQuery()
        const queryClient = createTestQueryClient()

        const { result } = renderHookWithProviders(() => ({ preview: useThemePreview(), value: useThemePreviewValue() }), { queryClient })
        act(() => result.current.preview.setPreview(buildDraft({ name: 'Live' })))
        await act(async () => await nextFrame())

        expect(result.current.value?.name).toBe('Live')
        expect(queryClient.getQueryData(QUERY_KEY.THEME.CURRENT)).toBeUndefined()

        act(() => result.current.preview.clearPreview())
    })

    test('한 프레임 안의 연속 push 는 마지막 draft 하나로 합쳐진다 (컬러 피커 드래그)', async () => {
        const { useThemePreview, useThemePreviewValue } = await importThemeQuery()
        const queryClient = createTestQueryClient()
        const publishedNames: (string | undefined)[] = []

        const { result } = renderHookWithProviders(
            () => {
                const value = useThemePreviewValue()
                publishedNames.push(value?.name)
                return { preview: useThemePreview(), value }
            },
            { queryClient },
        )
        act(() => {
            result.current.preview.setPreview(buildDraft({ name: 'first' }))
            result.current.preview.setPreview(buildDraft({ name: 'second' }))
            result.current.preview.setPreview(buildDraft({ name: 'third' }))
        })
        await act(async () => await nextFrame())

        expect(result.current.value?.name).toBe('third')
        expect(publishedNames.filter((name) => name === 'first' || name === 'second')).toEqual([])

        act(() => result.current.preview.clearPreview())
    })

    test('clearPreview 는 아직 flush 되지 않은 프레임까지 취소해 스토어를 되살리지 않는다', async () => {
        const { useThemePreview, useThemePreviewValue } = await importThemeQuery()
        const queryClient = createTestQueryClient()

        const { result } = renderHookWithProviders(() => ({ preview: useThemePreview(), value: useThemePreviewValue() }), { queryClient })
        act(() => {
            result.current.preview.setPreview(buildDraft({ name: 'stale' }))
            result.current.preview.clearPreview()
        })
        await act(async () => await nextFrame())

        expect(result.current.value).toBeNull()
    })

    test('draft 에 tokenColors 가 없으면 base 테마의 규칙을 그대로 쓴다 (vsix 임포트 미리보기)', async () => {
        const { useThemePreview, useThemePreviewValue } = await importThemeQuery()
        const queryClient = createTestQueryClient()
        queryClient.setQueryData(QUERY_KEY.THEME.DETAIL('base-theme'), BASE_THEME)

        const { result } = renderHookWithProviders(() => ({ preview: useThemePreview(), value: useThemePreviewValue() }), { queryClient })
        act(() => result.current.preview.setPreview(buildDraft()))
        await act(async () => await nextFrame())

        expect(result.current.value?.tokenColors).toEqual(BASE_THEME.tokenColors)

        act(() => result.current.preview.clearPreview())
    })

    test('draft 자신의 tokenColors 가 base 보다 우선한다', async () => {
        const { useThemePreview, useThemePreviewValue } = await importThemeQuery()
        const queryClient = createTestQueryClient()
        queryClient.setQueryData(QUERY_KEY.THEME.DETAIL('base-theme'), BASE_THEME)
        const ownRules = [{ scope: 'string', settings: { foreground: '#00ff00' } }] as unknown as ResolvedTheme['tokenColors']

        const { result } = renderHookWithProviders(() => ({ preview: useThemePreview(), value: useThemePreviewValue() }), { queryClient })
        act(() => result.current.preview.setPreview(buildDraft({ metadata: { tokenColors: ownRules, author: null, license: null, source: null } })))
        await act(async () => await nextFrame())

        expect(result.current.value?.tokenColors).toEqual(ownRules)

        act(() => result.current.preview.clearPreview())
    })
})
