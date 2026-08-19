import { useMutation } from '@tanstack/react-query'
import { replaceSearch } from '@entities/search/search.ipc'

/**
 * Wraps `replaceSearch`'s request/response call in `useMutation` instead of a widget hand-rolling
 * its own `.then`/`.catch`/`.finally` chain and `isReplacing` state (contract F4#5) — `isPending`
 * already tracks that, so the caller no longer needs a parallel `useState` for it.
 */
export const useReplaceSearch = () => useMutation({ mutationFn: replaceSearch })
