const CSS_VARIABLE_PREFIX = '--taide'

const toKebab = (value: string) => value.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase()

export const toCssVariableName = (token: string) => `${CSS_VARIABLE_PREFIX}-${token.split('.').map(toKebab).join('-')}`

export const toCssVariables = (colors: Record<string, string>) =>
    Object.fromEntries(Object.entries(colors).map(([token, value]) => [toCssVariableName(token), value]))

export const applyThemeVariables = (colors: Record<string, string>, target: HTMLElement) => {
    for (const [name, value] of Object.entries(toCssVariables(colors))) target.style.setProperty(name, value)
}
