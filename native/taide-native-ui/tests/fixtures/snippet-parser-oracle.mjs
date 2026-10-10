import {
    Choice,
    FormatString,
    Placeholder,
    SnippetParser,
    Text,
    TextmateSnippet,
    Transform,
    Variable,
} from '../../../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetParser.js'
import { normalizeIndentation } from '../../../../node_modules/monaco-editor/esm/vs/editor/common/core/misc/indentation.js'

const inputs = new WeakMap()
const NESTED_CASE_DEPTH = 32
const LARGE_INDEX_DIGITS = 400
const NativeRegExp = globalThis.RegExp
const originalTransformResolve = Transform.prototype.resolve
const originalTransformClone = Transform.prototype.clone
let attempts = []
let evaluations = []

class RecordingRegExp extends NativeRegExp {
    constructor(pattern, options) {
        try {
            super(pattern, options)
        } catch (error) {
            attempts = [...attempts, { pattern, options: options ?? '', accepted: false }]
            throw error
        }
        inputs.set(this, { pattern, options: options ?? '' })
        attempts = [
            ...attempts,
            { pattern, options: options ?? '', accepted: true, source: this.source, ignoreCase: this.ignoreCase, global: this.global },
        ]
    }
}

const formatPart = (part) => {
    if (part instanceof Text) return { kind: 'text', text: part.value }
    if (!(part instanceof FormatString)) throw new TypeError('Unexpected original format marker')
    return {
        kind: 'capture',
        index: part.index,
        shorthand: part.shorthandName ?? null,
        ifValue: part.ifValue ?? null,
        elseValue: part.elseValue ?? null,
    }
}

const transform = (value) => {
    if (!value) return null
    return { ...inputs.get(value.regexp), format: value.children.map(formatPart) }
}

class RecordingTransform extends Transform {
    clone() {
        const result = originalTransformClone.call(this)
        const options = [
            ['i', this.regexp.ignoreCase],
            ['g', this.regexp.global],
            ['d', this.regexp.hasIndices],
            ['m', this.regexp.multiline],
            ['s', this.regexp.dotAll],
            ['u', this.regexp.unicode],
            ['v', this.regexp.unicodeSets],
            ['y', this.regexp.sticky],
        ]
            .filter(([, enabled]) => enabled)
            .map(([flag]) => flag)
            .join('')
        result.regexp = new RecordingRegExp(this.regexp.source, options)
        return result
    }
    resolve(value) {
        const result = originalTransformResolve.call(this, value)
        evaluations = [...evaluations, { transform: transform(this), value, result }]
        return result
    }
}

const marker = (value) => {
    if (value instanceof Text) return { kind: 'text', text: value.value }
    if (value instanceof Placeholder) {
        return {
            kind: 'placeholder',
            index: value.index,
            children: value.children.filter((child) => !(child instanceof Choice)).map(marker),
            choices: value.choice?.options.map((option) => option.value) ?? null,
            transform: transform(value.transform),
        }
    }
    if (!(value instanceof Variable)) throw new TypeError('Unexpected original syntax marker')
    return { kind: 'variable', name: value.name, children: value.children.map(marker), transform: transform(value.transform) }
}

const templates = [
    '',
    '한글😀 text',
    String.raw`\$1 \} \\ \x`,
    '$1${2} $0 $001',
    '$name $_x9$é${_FOO_1}',
    '${1:foo ${2:한글}} $1',
    '${1|a,b,c|}',
    '$' + String.raw`{1|a\,b,c\|d,e\\f|}`,
    '${0|a,b|}',
    '${1|,b|}',
    '${1|a,|}',
    '${1:foo $2',
    '${NAME:${1:x}}${name}',
    '${_x',
    '${1/abc/$1/}',
    '${1/(.*)/$1-${1:/upcase}-${2:+yes}-${3:-no}-${4:?if:else}-${5:other}/gi}',
    '$' + String.raw`{TM_FILENAME/(.*)\..+$/$1/}`,
    '$' + String.raw`{1/ab\/cd/slash\/\\/g}`,
    '${1/[/$1/}',
    '${1/a/$1/gg}',
    '${1/a//}',
    '${1/(.)/${1:+}/}',
    '${1/(.)/${1:?yes:}/}',
    '$' + String.raw`{1/(.)/` + '$' + String.raw`{1:+a\}b}-` + '$' + String.raw`{2:-c\\d}-` + '$' + String.raw`{3:?x\$:y}/}`,
    '$' + String.raw`{1/(.)/` + '$' + String.raw`{1:+a\x}/}`,
    '${1/(.)/${1:/unknownName}/}',
    '${1/(.)/${x}/}',
    '${1/(.)/${1+bad}/}',
    '${1/(.)/$1',
    '${foo/((a))/${1:?if:else}/g}',
    '${00:a} ${2:} } { $',
    '${4294967296:a} $9007199254740993',
    `$${'9'.repeat(LARGE_INDEX_DIGITS)}`,
    Array.from({ length: NESTED_CASE_DEPTH }).reduce((text) => '${1:' + text + '}', 'nested'),
]

const completeTemplates = [
    ...templates.slice(0, -1),
    '$1 ${1:later}',
    '${1:first} ${1:second}',
    '${1:a $2} ${2:b} $1 $2',
    '${1:${1:inner}} $1',
    '${1:${2:$1}} $2',
    '${1:${2:two}} $1 $2',
    '${1:$foo} $1',
    '${1:${VAR:$2}} ${2:text} $1',
    '${1|a,b|} $1 ${1:other}',
    '$0 ${0:value}',
    '${1/(.*)/${1:/upcase}/g} ${1:body}',
    '한${1:😀} $1${0:끝}',
    '${1:${2:x} ${2:y}}$1$2',
    '${1:${2/(.)/$1/m}} $1',
    '${1:${2//a/s}} $1',
    '$' + String.raw`{1:` + '$' + String.raw`{2/ab\/cd/x/giu}} ` + '$1',
    '${1:${2:${3:$1}}} $2 $3',
    '${1:${VAR:${2:x}}} $1 $2',
    '$1 ${1:$2} ${2:$3} ${3:end}',
    Array.from({ length: NESTED_CASE_DEPTH }).reduce((text, index) => '${' + (index + 1) + ':' + text + '}', 'nested'),
]

const finalOptions = [
    { insert: false, enforce: false },
    { insert: true, enforce: false },
    { insert: false, enforce: true },
    { insert: true, enforce: true },
]

const variableTemplates = [
    '$TM_SELECTED_TEXT',
    '${TM_SELECTED_TEXT:${1:default}} $1',
    '${UNKNOWN:${1:default}} $1',
    '$UNKNOWN ${UNKNOWN} ${UNKNOWN:default}',
    '$__proto__ ${__proto__:default}',
    '$TM_CURRENT_LINE ${TM_CURRENT_LINE:default}',
    '${UNKNOWN:$TM_SELECTED_TEXT ${TM_CURRENT_LINE:${1:body}}}',
    '${TM_SELECTED_TEXT:$TM_CURRENT_LINE ${1:body}} $1',
    '${1:$TM_SELECTED_TEXT} $1 ${2:next}',
    '${1:$UNKNOWN} $1',
    '$' + String.raw`{TM_FILENAME/(.*)\..+$/$1/}`,
    '${UNKNOWN/(.*)/${1:+yes}${1:-missing}/}',
    '${UNKNOWN/(.+)/${1:?yes:no}/}',
    '${TM_CURRENT_LINE/(.*)/${1:/upcase}/}',
    '${TM_SELECTED_TEXT/(.*)/${1:/capitalize}/g}',
    '${1/(.*)/${1:/upcase}/} ${1:$TM_SELECTED_TEXT}',
    '${1:${TM_FILENAME/(.*)/${1:/upcase}/gi}} $1',
    '  text\n\t$TM_SELECTED_TEXT\r\n  ${TM_CURRENT_LINE}\r$UNKNOWN',
    '${1|  literal,two|}$TM_SELECTED_TEXT ${TM_CURRENT_LINE}',
    '${1:$TM_CURRENT_LINE} $TM_SELECTED_TEXT',
    '${UNKNOWN:${2:$TM_CURRENT_LINE}} $TM_SELECTED_TEXT',
    '$TM_SELECTED_TEXT${TM_FILENAME}$TM_CURRENT_LINE',
    '漢${1:${TM_SELECTED_TEXT}}😀${0:끝}',
]

const variableValues = [
    { TM_SELECTED_TEXT: 'one\n  two😀', TM_CURRENT_LINE: 'alpha beta', TM_FILENAME: 'example-123.456-TEST.js' },
    { TM_SELECTED_TEXT: null, TM_CURRENT_LINE: '', TM_FILENAME: '.dotfile' },
    { TM_SELECTED_TEXT: '$1 $UNKNOWN\r\n\t文字', TM_CURRENT_LINE: '  ßΣ\r\n  last', TM_FILENAME: '한글.rs' },
]

const whitespaceTemplates = [
    '',
    '\t one\n\t two',
    'one\r\ntwo\rthree\nfour\n',
    '${1:\tfoo\n\tbar} $1',
    '\n${1:\tfoo}\n${2:  bar}',
    '${1:foo\n}${2:\tbar}',
    '${1:foo\n}${2:}${3:\tbar}',
    '${1|foo\n,two|}\tbar',
    '${1|\tfoo\r\n,two|}\tbar',
    '${UNKNOWN:\tfoo\r\n\tbar}\r\n\t$TM_SELECTED_TEXT',
    '${1:${2:\tfoo\n}\tbar}\n\tend',
    '漢😀\n\t${1:文字}\n\t${0:끝}',
    '${1/(.*)/${1:/upcase}/} \n${1:\ttext}',
    '  \u00a0x\n\u2003  y\n\tend',
    '${1:}\tfirst${2:\r\n}\tlast',
]

const whitespaceContexts = [
    { line: '\t  owner', byteColumn: 3, indentSize: 4, insertSpaces: true, eol: '\n', adjust: true },
    { line: ' \t  owner', byteColumn: 4, indentSize: 4, insertSpaces: false, eol: '\r\n', adjust: true },
    { line: '\t\towner', byteColumn: 1, indentSize: 2, insertSpaces: true, eol: '\r\n', adjust: false },
    { line: '  owner', byteColumn: 1, indentSize: 2, insertSpaces: false, eol: '\n', adjust: true },
    { line: '漢😀 \towner', byteColumn: Buffer.byteLength('漢😀 '), indentSize: 8, insertSpaces: true, eol: '\n', adjust: true },
    { line: 'owner', byteColumn: 0, indentSize: 1, insertSpaces: false, eol: '\r\n', adjust: true },
]

const whitespaceApi =
    process.argv[2] === 'whitespace'
        ? await import('../../../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetSession.js')
        : null

const markerPath = (value) => {
    let path = []
    let current = value
    while (current.parent) {
        path = [current.parent.children.indexOf(current), ...path]
        current = current.parent
    }
    return path
}

globalThis.RegExp = RecordingRegExp
Transform.prototype.resolve = RecordingTransform.prototype.resolve
try {
    let configurations = templates.map((input) => ({ input, options: null, variables: null }))
    if (process.argv[2] === 'complete') {
        configurations = completeTemplates.flatMap((input) => finalOptions.map((options) => ({ input, options, variables: null })))
    }
    if (process.argv[2] === 'variables') {
        configurations = variableTemplates.flatMap((input) =>
            variableValues.map((variables) => ({ input, options: { insert: true, enforce: false }, variables })),
        )
    }
    if (whitespaceApi) {
        configurations = whitespaceTemplates.flatMap((input) =>
            whitespaceContexts.map((whitespace) => ({ input, options: { insert: true, enforce: false }, variables: null, whitespace })),
        )
    }
    const cases = configurations.map(({ input, options, variables, whitespace }) => {
        attempts = []
        evaluations = []
        const parser = new SnippetParser()
        let snippet = new TextmateSnippet()
        if (options) {
            snippet = parser.parse(input, options.insert, options.enforce)
        } else {
            parser._scanner.text(input)
            parser._token = parser._scanner.next()
            while (parser._parse(snippet)) {}
        }
        let resolutions = []
        let leading = null
        if (whitespace) {
            const { line, byteColumn, indentSize, insertSpaces, eol, adjust } = whitespace
            const column = Buffer.from(line).subarray(0, byteColumn).toString().length + 1
            leading = whitespaceApi.SnippetSession.adjustWhitespace(
                {
                    getLineContent: () => line,
                    normalizeIndentation: (value) => normalizeIndentation(value, indentSize, insertSpaces),
                    getEOL: () => eol,
                },
                { lineNumber: 1, column },
                adjust,
                snippet,
            )
        }
        if (variables) {
            const values = new Map(Object.entries(variables))
            snippet.resolveVariables({
                resolve: (variable) => {
                    let precedingTextLine = null
                    snippet.walk((candidate) => {
                        if (candidate === variable) return false
                        if (candidate instanceof Text) precedingTextLine = candidate.value.split(/\r\n|\r|\n/).at(-1)
                        return true
                    })
                    resolutions = [...resolutions, { name: variable.name, precedingTextLine }]
                    return values.get(variable.name) ?? undefined
                },
            })
        }
        const text = snippet.toString()
        const spans = snippet.placeholders.map((placeholder) => ({
            index: placeholder.index,
            start: Buffer.byteLength(text.slice(0, snippet.offset(placeholder))),
            end: Buffer.byteLength(text.slice(0, snippet.offset(placeholder) + snippet.fullLen(placeholder))),
            choices: placeholder.choice?.options.map((option) => option.value) ?? null,
            ...(variables || whitespace
                ? {
                      markerPath: markerPath(placeholder),
                      enclosing: snippet.enclosingPlaceholders(placeholder).map((parent) => snippet.placeholders.indexOf(parent)),
                  }
                : {}),
        }))
        const expected = snippet.children.map(marker)
        let correctedExpected = expected
        if (process.argv[2] === 'complete') {
            Transform.prototype.clone = RecordingTransform.prototype.clone
            try {
                correctedExpected = new SnippetParser().parse(input, options.insert, options.enforce).children.map(marker)
            } finally {
                Transform.prototype.clone = originalTransformClone
            }
        }
        return {
            input,
            options,
            variables,
            whitespace,
            leading,
            expected,
            correctedExpected,
            attempts,
            text,
            spans,
            evaluations,
            resolutions,
        }
    })
    await new Promise((resolve, reject) => {
        process.stdout.write(JSON.stringify(cases), (error) => {
            if (error) {
                reject(error)
                return
            }
            resolve()
        })
    })
} finally {
    globalThis.RegExp = NativeRegExp
    Transform.prototype.resolve = originalTransformResolve
    Transform.prototype.clone = originalTransformClone
}
if (whitespaceApi) process.exit(0)
