const SOURCE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js',
    import.meta.url,
)
const source = await Bun.file(SOURCE_URL).text()
const replacements = [
    ['path = decodeURIComponent(path);', ''],
    [
        "part = part.replace(/~1/g, '/').replace(/~0/g, '~');",
        "if (/~(?![01])/.test(part)) { current = undefined; return true; } part = part.replace(/~1/g, '/').replace(/~0/g, '~');",
    ],
    [
        'current = current[part];',
        "if (Array.isArray(current) && !/^(0|[1-9][0-9]*)$/.test(part)) { current = undefined; return true; } current = current !== null && typeof current === 'object' && Object.prototype.hasOwnProperty.call(current, part) ? current[part] : undefined;",
    ],
    ['return !current;', 'return current === undefined;'],
    ['const merge = (target, section) => {', 'const merge = (target, section) => { if (section === false) { target.not = {}; return; }'],
    ['section.hasOwnProperty(key)', 'Object.prototype.hasOwnProperty.call(section, key)'],
    [
        'let section;',
        `let section;
            try { if (refSegment !== undefined) refSegment = decodeURIComponent(refSegment); }
            catch { resolveErrors.push(t("$ref '{0}' in '{1}' can not be resolved.", refSegment || '', sourceHandle.uri)); return; }`,
    ],
    ['if (section) {', 'if (section !== undefined) {'],
    [
        'const ref = next.$ref;',
        `if (typeof next.$ref !== 'string') { delete next.$ref; resolveErrors.push('Invalid $ref: a string is required.'); break; } const ref = next.$ref;`,
    ],
]
export const writeReferenceSource = async (destinationUrl) => {
    let patched = source
    for (const [before, after] of replacements) {
        if (patched.split(before).length !== 2) throw new Error(`Unexpected original schema refs marker: ${before}`)
        patched = patched.replace(before, after)
    }
    patched = patched.replace(
        /(from\s+|import\s+)'(\.[^']+)'/g,
        (_, prefix, relative) => `${prefix}${JSON.stringify(new URL(relative, SOURCE_URL).href)}`,
    )
    await Bun.write(destinationUrl, patched)
    return await import(destinationUrl.href)
}

export const REFERENCE_SOURCE_CORRECTIONS = replacements.length
