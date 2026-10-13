const part = (value) => (value == null ? '-' : `${new TextEncoder().encode(value).length}:${value}`)
const valueOf = (value, previousEnd) => {
    const kinds = {
        Literal: () => `l${part(value.value)}`,
        CustomProperty: () => `c${part(value.value)}`,
        NumberValue: () => `n${value.value}:${part(value.rawValue)}${part(value.unit)}`,
        ColorValue: () => `r${value.r}:${value.g}:${value.b}:${value.a}:${part(value.raw)}`,
        StringValue: () => `s${value.quote}:${part(value.value)}`,
        Field: () => `f${value.index == null ? '-' : value.index}:${Number(value.start === previousEnd)}:${part(value.name)}`,
        FunctionCall: () => `x${part(value.name)}[${fragmentsOf(value.arguments)}]`,
    }
    const render = kinds[value.type]
    if (!render) throw new Error(`Unknown CSS value ${value.type}`)
    return render()
}
const fragmentsOf = (fragments) =>
    fragments
        .map((fragment) => `[${fragment.value.map((value, index) => valueOf(value, index > 0 ? fragment.value[index - 1]?.end : -1)).join('')}]`)
        .join('')
export const canonicalStylesheet = (properties) =>
    properties.map((property) => `${part(property.name)}i${Number(property.important)}[${fragmentsOf(property.value)}]`).join('')
