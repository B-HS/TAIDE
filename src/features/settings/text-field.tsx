import type { FC } from 'react'

type TextFieldProps = {
    label: string
    value: string
    placeholder?: string
    onCommit: (value: string) => void
}

export const TextField: FC<TextFieldProps> = ({ label, value, placeholder, onCommit }) => (
    <label className='flex items-center justify-between gap-3 text-xs'>
        <span className='text-app-foreground'>{label}</span>
        <input
            key={value}
            type='text'
            defaultValue={value}
            placeholder={placeholder}
            className='bg-panel-input-background border-panel-input-border text-app-foreground w-56 rounded-sm border px-2 py-1'
            onBlur={(event) => onCommit(event.currentTarget.value)}
        />
    </label>
)
