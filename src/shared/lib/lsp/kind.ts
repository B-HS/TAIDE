const KIND_SEPARATOR = '.'

/**
 * Tests LSP `CodeActionKind` hierarchy containment: `parent` contains `child` when they are
 * equal, `parent` is the empty kind (`''`, the hierarchy root), or `child` is a dot-separated
 * descendant of `parent` (e.g. `source.organizeImports` contains `source.organizeImports.ts`).
 * Mirrors monaco's own `HierarchicalKind.contains` so client-side kind filtering (on-save,
 * `only` post-filtering) agrees with how monaco itself matches `providedCodeActionKinds`.
 */
export const kindContains = (parent: string, child: string) => parent === child || parent === '' || child.startsWith(`${parent}${KIND_SEPARATOR}`)

/**
 * True when `kind` matches at least one entry in `parents` per {@link kindContains}. Used to
 * post-filter a code action response against the requested `only` kind(s) — servers are not
 * required to honor `only` strictly (LSP spec: it is a hint the client may also filter by).
 */
export const kindMatchesAny = (parents: readonly string[], kind: string) => parents.some((parent) => kindContains(parent, kind))
