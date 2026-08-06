import type { FC } from 'react'

type NumericFieldProps = {
    label: string
    value: number
    min: number
    max: number
    onCommit: (value: number) => void
}

export const NumericField: FC<NumericFieldProps> = ({ label, value, min, max, onCommit }) => (
    <label className='flex items-center justify-between gap-3 text-xs'>
        <span className='text-app-foreground'>{label}</span>
        <input
            key={value}
            type='number'
            defaultValue={value}
            min={min}
            max={max}
            className='bg-panel-input-background border-panel-input-border text-app-foreground w-20 rounded-sm border px-2 py-1 text-right'
            onBlur={(event) => {
                const next = event.currentTarget.valueAsNumber
                if (Number.isNaN(next)) return
                onCommit(Math.min(max, Math.max(min, next)))
            }}
        />
    </label>
)
