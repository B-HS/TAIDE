const part = (value) => (value == null ? '-' : `${new TextEncoder().encode(value).length}:${value}`)
const values = (value) =>
    value == null
        ? '-'
        : `[${value.map((token) => (typeof token === 'string' ? `s${part(token)}` : `f${token.index}:${part(token.name)}`)).join('')}]`
const node = (value) => {
    const attributes =
        value.attributes == null
            ? '-'
            : `[${value.attributes.map((attribute) => `${part(attribute.name)}${values(attribute.value)}t${attribute.valueType}b${Number(attribute.boolean ?? false)}i${Number(attribute.implied ?? false)}m${Number(attribute.multiple ?? false)}`).join('')}]`
    const repeat = value.repeat ? `${value.repeat.count}:${value.repeat.value}:${Number(value.repeat.implicit)}` : '-'
    return `${part(value.name)}${values(value.value)}r${repeat}s${Number(value.selfClosing ?? false)}a${attributes}c[${value.children.map(node).join('')}]`
}

export const canonicalNodes = (nodes) => nodes.map(node).join('')
