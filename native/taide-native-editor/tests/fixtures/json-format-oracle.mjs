import { format as rawFormat } from '../../../../node_modules/monaco-editor/esm/external/jsonc-parser/lib/esm/impl/format.js'
import { format as serviceFormat } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/utils/format.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'
import { writeBoundedScanner } from './json-source-boundaries.mjs'

const FORMAT_URL = new URL('../../../../node_modules/monaco-editor/esm/external/jsonc-parser/lib/esm/impl/format.js', import.meta.url)
const SCANNER_URL = new URL('./scanner.js', FORMAT_URL)
const SCANNER_PATCH_URL = new URL('file:///private/tmp/taide-json-format-scanner-oracle.mjs')
const FORMAT_PATCH_URL = new URL('file:///private/tmp/taide-json-format-source-oracle.mjs')
const SCANNER_IMPORT = "import { createScanner } from './scanner.js';"
const source = await Bun.file(FORMAT_URL).text()
if (source.split(SCANNER_IMPORT).length !== 2) throw new Error('Unexpected original format scanner import')
await writeBoundedScanner(SCANNER_URL, SCANNER_PATCH_URL)
const patched = source
    .replace(SCANNER_IMPORT, `import { createScanner } from ${JSON.stringify(SCANNER_PATCH_URL.href)};`)
    .replace(/(from\s+)'(\.[^']+)'/g, (_, prefix, relative) => `${prefix}${JSON.stringify(new URL(relative, FORMAT_URL).href)}`)
await Bun.write(FORMAT_PATCH_URL, patched)
const correctedProvider = await import(FORMAT_PATCH_URL.href)
const INDENTATIONS = [
    { tabSize: 0, insertSpaces: true },
    { tabSize: 1, insertSpaces: true },
    { tabSize: 2, insertSpaces: true },
    { tabSize: 4, insertSpaces: true },
    { tabSize: 8, insertSpaces: true },
    { tabSize: 4, insertSpaces: false },
]
const inputSet = new Set(['', ' ', '\t', '\n', '\r\n', 'null', 'true', '1e', 'unknown', '{}', '[]', '{', '[', '/*', '/*x\r\n', '//x', '"𐐀'])
const structured = [
    '{"a":1,"b":[true,false,null,{"c":"text"}]}',
    '  { "a" : 1 , "b" : [ 0 , 2 ] }  ',
    '{\n\n "a":1,\n\n "b":{\n"c":[]\n}\n}\n\n',
    '\n\n  {"a":1}\n\n',
    '\t\t{ "a":1, "b":2 }',
    '{"a":/*a*/1,//line\n"b":2}',
    '{\n//x\n"a":1\n/*x\ny*/\n}',
    '[0,/*x*/1, //x\n 2]',
    '{"𐐀":"漢字\\uD800", "b":"\\q"}',
    '{"a":1e,"b":1.,"c":true false}',
    '{"a"\n"b":2}',
    '{"a":1,/*',
    '{"a":1,/*x\r\n',
    '[1,,2,3 4,]',
    '}]\n[{',
]
for (const input of structured) {
    inputSet.add(input)
    inputSet.add(input.replaceAll('\n', '\r'))
    inputSet.add(input.replaceAll('\n', '\r\n'))
}
const hexUnits = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const canonical = (edits) => edits.map((edit) => `${edit.offset}..${edit.offset + edit.length}:${hexUnits(edit.content)};`).join('')
const apply = (input, edits) => {
    let result = input
    for (const edit of edits.toReversed()) result = result.slice(0, edit.offset) + edit.content + result.slice(edit.offset + edit.length)
    return result
}
let corrections = 0
const cases = Array.from(inputSet).flatMap((input) => {
    const document = TextDocument.create('file:///format.json', 'json', 1, input)
    const cursorUnits = [0]
    let unit = 0
    for (const character of input) {
        unit += character.length
        cursorUnits.push(unit)
    }
    const middle = cursorUnits[Math.floor(cursorUnits.length / 2)]
    const ranges = [null, { offset: 0, length: input.length }, { offset: middle, length: 0 }, { offset: middle, length: input.length - middle }]
    return INDENTATIONS.flatMap((indentation) =>
        [false, true].flatMap((keepLines) =>
            [false, true].flatMap((insertFinalNewline) =>
                [false, true].flatMap((isService) =>
                    ranges.map((range) => {
                        const options = { ...indentation, keepLines, insertFinalNewline, eol: isService ? '\n' : '\u2028' }
                        const rawEdits = isService
                            ? serviceFormat(
                                  document,
                                  options,
                                  range == null
                                      ? undefined
                                      : { start: document.positionAt(range.offset), end: document.positionAt(range.offset + range.length) },
                              ).map((edit) => ({
                                  offset: document.offsetAt(edit.range.start),
                                  length: document.offsetAt(edit.range.end) - document.offsetAt(edit.range.start),
                                  content: edit.newText,
                              }))
                            : rawFormat(input, range, options)
                        const expectedEdits = correctedProvider.format(input, range, options)
                        const raw = canonical(rawEdits)
                        const expected = canonical(expectedEdits)
                        if (raw !== expected) corrections += 1
                        const rustRange =
                            range == null
                                ? 'None'
                                : `Some(${new TextEncoder().encode(input.slice(0, range.offset)).length}..${new TextEncoder().encode(input.slice(0, range.offset + range.length)).length})`
                        const applied = apply(input, expectedEdits)
                        return `    (${rustLiteral(input)}, ${rustRange}, ${indentation.tabSize}, ${indentation.insertSpaces}, ${rustLiteral(options.eol)}, ${keepLines}, ${insertFinalNewline}, ${rustLiteral(raw)}, ${rustLiteral(expected)}, ${rustLiteral(applied)}),`
                    }),
                ),
            ),
        ),
    )
})
await Bun.write(
    new URL('./json-format-oracle.rs', import.meta.url),
    `const CASES: &[(&str, Option<std::ops::Range<usize>>, u32, bool, &str, bool, bool, &str, &str, &str)] = &[\n${cases.join('\n')}\n];\n`,
)
process.stdout.write(`${JSON.stringify({ inputs: inputSet.size, cases: cases.length, corrections })}\n`)
