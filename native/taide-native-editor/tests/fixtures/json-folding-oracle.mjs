import { getFoldingRanges } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonFolding.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SOURCE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonFolding.js',
    import.meta.url,
)
const PATCHED_URL = new URL('file:///private/tmp/taide-json-folding-source-oracle.mjs')
const REGION_PATTERN = String.raw`/^\/\/\s*#(region\b)|(endregion\b)/`
const FIXED_REGION_PATTERN = String.raw`/^\/\/\s*#(?:(region\b)|(endregion\b))/`
const LEVEL_GUARD = 'if (level < 30)'
const FIXED_LEVEL_GUARD = 'if (level >= 0)'
const source = await Bun.file(SOURCE_URL).text()
for (const marker of [REGION_PATTERN, LEVEL_GUARD]) {
    if (source.split(marker).length !== 2) throw new Error(`Unexpected original folding source marker: ${marker}`)
}
const patched = source
    .replace(REGION_PATTERN, FIXED_REGION_PATTERN)
    .replace(LEVEL_GUARD, FIXED_LEVEL_GUARD)
    .replace(/(from\s+|import\s+)'(\.[^']+)'/g, (_, prefix, relative) => `${prefix}${JSON.stringify(new URL(relative, SOURCE_URL).href)}`)
await Bun.write(PATCHED_URL, patched)
const correctedProvider = await import(PATCHED_URL.href)
const RESULT_LIMITS = [null, 0, 1, 2, 4, 8, 32]
const DEPTHS = [1, 2, 4, 29, 30, 31, 34]
const inputs = new Set(['', '{}', '[]', '[\n]', '{\n}', '/*', '/*\n', '// #endregion', '// #region'])
const shapes = [
    '{\n "a":1\n}',
    '[\n 0\n]',
    '{\n "a":[\n 1\n ],\n "b":{\n "c":2\n }\n}',
    '{[\n 0\n]}',
    '{\n [\n 0\n }\n]',
    '{\n "text":"{[\\n]}"\n}',
    '/* comment\n second\n last */',
    '/*\n[\n0\n]\n',
    '/*\n/*\n[\n0\n]\n',
    '// #region x\n[\n0\n]\n// #endregion',
    '// #region\n// #region inner\n0\n// #endregion\n// #endregion',
    '// #region\n{\n0\n// #endregion\n}',
    '// #region\n// ordinary endregion text\n0\n// #endregion',
    '// #region\n// notanendregion suffix\n0\n// #endregion',
    '// #region\n// #endregionX\n0\n// #endregion',
    '// #region\n// #endregion_\n0\n// #endregion',
    '// #region\n// ENDREGION\n0\n// #endregion',
    '// #regioné\n0\n// #endregioné',
    '//\ufeff#region\n0\n//\u00a0#endregion',
    '//\u0085#region\n0\n// #endregion',
    '//\u2028#region\n0\n//\u2029#endregion',
    '{\n/* a\n b */\n[\n0\n]\n}',
    '[\n0\n]\n[\n1\n]\n{\n2\n}',
]
for (const shape of shapes) {
    for (const newline of ['\n', '\r', '\r\n']) inputs.add(shape.replaceAll('\n', newline))
}
for (const depth of DEPTHS) {
    inputs.add('[\n'.repeat(depth) + '0\n' + ']\n'.repeat(depth))
    inputs.add('{\n'.repeat(depth) + '[\n0\n]\n[\n1\n]\n')
    inputs.add('// #region\n'.repeat(depth) + '0\n' + '// #endregion\n'.repeat(depth))
}
const canonical = (ranges, exceeded) => ranges.map((range) => `${range.startLine}..${range.endLine}:${range.kind};`).join('') + `|${exceeded}`
let corrections = 0
const cases = Array.from(inputs).flatMap((input) =>
    RESULT_LIMITS.map((rangeLimit) => {
        const document = TextDocument.create('file:///folding.json', 'json', 1, input)
        let rawExceeded = false
        let correctedExceeded = false
        const rawContext = {
            onRangeLimitExceeded: () => {
                rawExceeded = true
            },
        }
        const correctedContext = {
            onRangeLimitExceeded: () => {
                correctedExceeded = true
            },
        }
        if (rangeLimit != null) {
            rawContext.rangeLimit = rangeLimit
            correctedContext.rangeLimit = rangeLimit
        }
        const raw = canonical(getFoldingRanges(document, rawContext), rawExceeded)
        const expected = canonical(correctedProvider.getFoldingRanges(document, correctedContext), correctedExceeded)
        if (raw !== expected) corrections += 1
        const limit = rangeLimit == null ? 'None' : `Some(${rangeLimit})`
        return `    (${rustLiteral(input)}, ${limit}, ${rustLiteral(raw)}, ${rustLiteral(expected)}),`
    }),
)
await Bun.write(
    new URL('./json-folding-oracle.rs', import.meta.url),
    `const CASES: &[(&str, Option<usize>, &str, &str)] = &[\n${cases.join('\n')}\n];\n`,
)
process.stdout.write(
    `${JSON.stringify({ inputs: inputs.size, cases: cases.length, corrections, sourceCorrections: [REGION_PATTERN, LEVEL_GUARD] })}\n`,
)
