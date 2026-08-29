import type { FC } from 'react'

type TextFieldProps = {
    label: string
    value: string
    placeholder?: string
    normalize?: (rawValue: string) => string
    onCommit: (value: string) => void
}

/**
 * `key={value}` remounts the uncontrolled input whenever the stored value changes, so a commit that
 * actually moves the setting is what moves the field forward.
 *
 * `normalize` is for the fields whose stored value is not literally the typed text — `editorRulers`
 * parses a comma-separated list and drops every column it cannot honor. Such a field must rewrite
 * itself to the normalized text on blur, because a normalization that lands back on the value
 * already stored leaves `value` (and therefore `key`) untouched: nothing remounts the input, and the
 * discarded text stays on screen as if it had been saved. Same rule, and the same reason, as the
 * numeric field's blur write-back (`numeric-field.tsx`, settings-ui.md §5).
 */
export const TextField: FC<TextFieldProps> = ({ label, value, placeholder, normalize, onCommit }) => (
    <label className='flex items-center justify-between gap-3 text-xs'>
        <span className='text-app-foreground min-w-0 truncate'>{label}</span>
        <input
            key={value}
            type='text'
            defaultValue={value}
            placeholder={placeholder}
            className='bg-panel-input-background border-panel-input-border text-app-foreground w-56 shrink-0 rounded-sm border px-2 py-1'
            onBlur={(event) => {
                const committedValue = normalize?.(event.currentTarget.value) ?? event.currentTarget.value
                event.currentTarget.value = committedValue
                onCommit(committedValue)
            }}
        />
    </label>
)
