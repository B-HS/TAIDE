import { QueryObserver } from '@tanstack/react-query'
import type { SnippetFile } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { monaco } from '@shared/lib/monaco/setup'
import { registerSnippetCompletions } from '@shared/lib/snippet-completion'
import { snippetListQueryOptions } from '@entities/snippet/snippet.query'
import { queryClient } from '@app/query-client'

/**
 * Keeps the `QUERY_KEY.SNIPPET.LIST` cache warm for the entire app session, not only while
 * Settings > Snippets (the only other mount point for `useSnippetList`, contract §3.3) happens to
 * be open. A bare `prefetchQuery` only *fetches once* — it creates no observer, and TanStack Query
 * garbage-collects a query with zero observers `gcTime` (`app/query-client.ts`'s 10-minute default)
 * after its fetch settles, silently emptying this cache for the rest of the session (`getSnippetFiles`
 * below would fall back to `[]` forever, with no error and no retry path). Subscribing a
 * `QueryObserver` for the app's lifetime is what `useSnippetList` would otherwise provide only while
 * mounted — it keeps the query permanently active (no GC), and gives `save`/`delete`'s
 * `invalidateQueries` (`entities/snippet/snippet.query.ts`, default `refetchType: 'active'`) an
 * observer to actually refetch against, so `registerSnippetCompletions`'s `getSnippetFiles` getter
 * always reads a live cache instead of a query that was quietly garbage-collected out from under it.
 */
const snippetListObserver = new QueryObserver(queryClient, snippetListQueryOptions())
snippetListObserver.subscribe(() => {})

registerSnippetCompletions(monaco, {
    getSnippetFiles: () => queryClient.getQueryData<SnippetFile[]>(QUERY_KEY.SNIPPET.LIST) ?? [],
})
