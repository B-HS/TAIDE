import type { FC } from 'react'
import { resolveNumericFieldCommit } from '@shared/lib/numeric-field-commit'

type NumericFieldProps = {
    label: string
    value: number
    min: number
    max: number
    onCommit: (value: number) => void
}

/**
 * `key={value}` remounts the uncontrolled input whenever the stored value changes, so a *successful*
 * commit is what moves the field forward. On blur the field is rewritten to the stored value first —
 * see {@link resolveNumericFieldCommit} for why the typed text must never be left standing.
 */
export const NumericField: FC<NumericFieldProps> = ({ label, value, min, max, onCommit }) => (
    <label className='flex items-center justify-between gap-3 text-xs'>
        <span className='text-app-foreground min-w-0 truncate'>{label}</span>
        <input
            key={value}
            type='number'
            defaultValue={value}
            min={min}
            max={max}
            className='bg-panel-input-background border-panel-input-border text-app-foreground w-20 shrink-0 rounded-sm border px-2 py-1 text-right'
            onBlur={(event) => {
                const commitValue = resolveNumericFieldCommit({ rawValue: event.currentTarget.valueAsNumber, committedValue: value, min, max })
                event.currentTarget.value = String(value)
                if (commitValue !== null) onCommit(commitValue)
            }}
        />
    </label>
)
