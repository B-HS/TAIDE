import type { FC, PropsWithChildren, ReactElement } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import type { RenderHookOptions, RenderOptions } from '@testing-library/react'
import { render, renderHook } from '@testing-library/react'
import { i18next } from '@shared/i18n/i18n'

const TEST_QUERY_GC_TIME_MS = 0

/**
 * A `QueryClient` scoped to one test: no retries (a failing `queryFn` must surface as an error, not
 * as a hang), no cache lifetime past the last observer, and `networkMode: 'always'` because every
 * query in this app talks to Tauri IPC rather than the network — the same reason `app/query-client.ts`
 * sets it in production. Created per render so nothing survives into the next test.
 */
export const createTestQueryClient = () =>
    new QueryClient({
        defaultOptions: {
            queries: { retry: 0, gcTime: TEST_QUERY_GC_TIME_MS, staleTime: 0, networkMode: 'always', refetchOnWindowFocus: false },
            mutations: { retry: 0, networkMode: 'always' },
        },
    })

const createTestProviders =
    (queryClient: QueryClient): FC<PropsWithChildren> =>
    ({ children }) => (
        <QueryClientProvider client={queryClient}>
            <I18nextProvider i18n={i18next}>{children}</I18nextProvider>
        </QueryClientProvider>
    )

type WithQueryClient = { queryClient?: QueryClient }

/**
 * Mounts `ui` under the two providers a component may not run without: TanStack Query and i18n.
 *
 * Deliberately smaller than `AppProviders` — no toaster, no tooltip provider — so a test only pays
 * for what it asserts; a component needing more composes it into the `ui` it passes in. The i18n
 * instance is the app's own singleton with no resource bundles loaded, so `t('a.b')` renders the key
 * itself and assertions can match on keys instead of translated copy.
 *
 * Returns the created `queryClient` alongside the render result so a test can seed or inspect the
 * cache (`setQueryData`, `getQueryState`) without having to build the client itself.
 */
export const renderWithProviders = (
    ui: ReactElement,
    { queryClient = createTestQueryClient(), ...options }: Omit<RenderOptions, 'wrapper'> & WithQueryClient = {},
) => ({
    queryClient,
    ...render(ui, { wrapper: createTestProviders(queryClient), ...options }),
})

/** {@link renderWithProviders} for a hook — same providers, same returned `queryClient`. */
export const renderHookWithProviders = <TResult, TProps>(
    hook: (props: TProps) => TResult,
    { queryClient = createTestQueryClient(), ...options }: Omit<RenderHookOptions<TProps>, 'wrapper'> & WithQueryClient = {},
) => ({
    queryClient,
    ...renderHook(hook, { wrapper: createTestProviders(queryClient), ...options }),
})

export { act, cleanup, fireEvent, render, renderHook, screen, waitFor, within } from '@testing-library/react'
