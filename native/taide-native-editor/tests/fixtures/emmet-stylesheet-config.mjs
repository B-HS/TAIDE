const helper = await Bun.file(new URL('../../../../node_modules/emmet-monaco-es/src/emmetHelper.ts', import.meta.url)).text()
export const FUZZY_SCORE_TEXT = helper.match(/'stylesheet\.fuzzySearchMinScore':\s*(0(?:\.\d+)?)/)?.[1]
if (!FUZZY_SCORE_TEXT) throw new Error('Missing project Emmet fuzzy threshold')
export const FUZZY_MIN_SCORE = Number(FUZZY_SCORE_TEXT)

export const createStylesheetOptions = () => ({
    'output.field': (index, placeholder) => `\${${index}${placeholder ? ':' + placeholder : ''}}`,
    'stylesheet.shortHex': true,
    'stylesheet.between': ': ',
    'stylesheet.after': ';',
    'stylesheet.intUnit': 'px',
    'stylesheet.floatUnit': 'em',
    'stylesheet.unitAliases': { e: 'em', p: '%', x: 'ex', r: 'rem' },
    'stylesheet.fuzzySearchMinScore': FUZZY_MIN_SCORE,
    'output.format': true,
    'output.selfClosingStyle': 'html',
})
