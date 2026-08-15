import type { SearchQuery } from '@shared/api/bindings'

export type SearchOptions = Required<Omit<SearchQuery, 'text'>>

export const DEFAULT_SEARCH_OPTIONS: SearchOptions = {
    caseSensitive: false,
    wholeWord: false,
    regex: false,
    includeGlob: null,
    excludeGlob: null,
    contextLines: 0,
    respectGitignore: true,
}
