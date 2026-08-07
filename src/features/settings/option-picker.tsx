import type { FC, KeyboardEvent } from 'react'
import { useState } from 'react'
import { Check, ChevronDown } from 'lucide-react'
import { Button } from '@shared/ui/button'
import { Command, CommandGroup, CommandItem, CommandList } from '@shared/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@shared/ui/popover'

type OptionPickerOption = { id: string; label: string }

type OptionPickerProps = {
    label: string
    options: OptionPickerOption[]
    value: string
    onSelect: (id: string) => void
}

export const OptionPicker: FC<OptionPickerProps> = ({ label, options, value, onSelect }) => {
    const [open, setOpen] = useState(false)

    const activeOption = options.find((option) => option.id === value) ?? options[0]
    const handleSelect = (id: string) => {
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
                        <span className='truncate text-xs'>{activeOption?.label ?? ''}</span>
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
