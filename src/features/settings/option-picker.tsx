import type { KeyboardEvent } from 'react'
import { useState } from 'react'
import { Check, ChevronDown } from 'lucide-react'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
import { Command, CommandGroup, CommandItem, CommandList } from '@shared/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@shared/ui/popover'

type OptionPickerOption<T extends string> = { id: T; label: string }

type OptionPickerProps<T extends string> = {
    label: string
    options: OptionPickerOption<T>[]
    value: T
    placeholder?: string
    onSelect: (id: T) => void
}

/**
 * A `value` matching no option renders `placeholder` (muted), never the first option's label. The
 * old `?? options[0]` fallback made "nothing is selected" look identical to "the first entry is
 * selected": switching the AI provider clears `settings.aiModel`, so the model picker advertised a
 * model that had never been chosen and that nothing downstream would actually use (audit §4-B C12).
 * Callers whose `value` always matches (the settings enums, which read through their own defaults)
 * are unaffected and pass no placeholder.
 */
export const OptionPicker = <T extends string>({ label, options, value, placeholder, onSelect }: OptionPickerProps<T>) => {
    const [open, setOpen] = useState(false)

    const activeOption = options.find((option) => option.id === value)
    const handleSelect = (id: T) => {
        onSelect(id)
        setOpen(false)
    }
    const handleTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
        if (event.key !== 'ArrowDown') return
        event.preventDefault()
        setOpen(true)
    }

    return (
        <label className='flex items-center justify-between gap-3 text-xs'>
            <span className='text-app-foreground min-w-0 truncate'>{label}</span>
            <Popover open={open} onOpenChange={setOpen}>
                <PopoverTrigger asChild>
                    <Button
                        type='button'
                        variant='outline'
                        size='sm'
                        role='combobox'
                        aria-expanded={open}
                        aria-label={label}
                        className='w-32 shrink-0 justify-between font-normal'
                        onKeyDown={handleTriggerKeyDown}>
                        <span className={cn('truncate text-xs', !activeOption && 'text-app-sidebar-icon-default')}>
                            {activeOption?.label ?? placeholder ?? ''}
                        </span>
                        <ChevronDown className='text-app-sidebar-icon-default size-4 shrink-0' />
                    </Button>
                </PopoverTrigger>
                <PopoverContent align='end' className='w-(--radix-popover-trigger-width) p-0'>
                    <Command>
                        <CommandList className='max-h-48'>
                            <CommandGroup>
                                {options.map((option) => (
                                    <CommandItem key={option.id} value={option.label} onSelect={() => handleSelect(option.id)} className='text-xs'>
                                        <span className='flex-1 truncate'>{option.label}</span>
                                        {option.id === value && <Check className='text-app-accent size-4 shrink-0' />}
                                    </CommandItem>
                                ))}
                            </CommandGroup>
                        </CommandList>
                    </Command>
                </PopoverContent>
            </Popover>
        </label>
    )
}
