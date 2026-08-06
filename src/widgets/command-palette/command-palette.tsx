import { useEffect, useEffectEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { File, Terminal } from 'lucide-react'
import { toast } from 'sonner'
import type { KeymapEntry } from '@shared/lib/keymap'
import { APP_KEYMAP, matchesKeymapEntry } from '@shared/lib/keymap'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { IS_MAC } from '@shared/constants/platform'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandShortcut } from '@shared/ui/command'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { activeProjectQueryOptions } from '@entities/project/project.query'
import { treeRowsQueryOptions } from '@entities/tree/tree.query'
import { useOpenTab, useReopenClosedTab } from '@entities/layout/layout.query'

const FILE_RESULT_LIMIT = 200
const OPEN_PALETTE_KEYMAP_ENTRY: KeymapEntry = { id: 'quick-open', key: 'p', mods: ['mod', 'shift'], descriptionKey: 'palette.title' }

type PaletteMode = 'commands' | 'files'

const keymapShortcutLabel = (entry: KeymapEntry) => {
    const modLabel = entry.mods.includes('mod') ? (IS_MAC ? '⌘' : 'Ctrl') : ''
    const otherLabels = entry.mods.filter((mod) => mod !== 'mod').map((mod) => (mod === 'shift' ? '⇧' : mod === 'alt' ? '⌥' : 'Ctrl'))
    return [...otherLabels, modLabel, entry.key.toUpperCase()].filter(Boolean).join('')
}

export const CommandPalette = () => {
    const [open, setOpen] = useState(false)
    const [mode, setMode] = useState<PaletteMode>('commands')
    const [query, setQuery] = useState('')

    const { t } = useTranslation()
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: treePage } = useQuery({ ...treeRowsQueryOptions(activeProjectId), enabled: open && mode === 'files' && !!activeProjectId })
    const { mutate: openTab } = useOpenTab(activeProjectId)
    const { mutate: reopenClosedTab } = useReopenClosedTab(activeProjectId)

    const handleOpenChange = (next: boolean) => {
        setOpen(next)
        if (!next) {
            setMode('commands')
            setQuery('')
        }
    }

    const openTerminalTab = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'terminal', sessionId: '' }, title: t('terminal.title'), target: null, preview: false },
            { onError: (error) => toast.error(error.message) },
        )
    }

    useGlobalKeymap({
        'quick-open': () => {
            setMode('files')
            setQuery('')
            setOpen(true)
        },
        'new-terminal': openTerminalTab,
    })

    const handleOpenPaletteShortcut = useEffectEvent((event: KeyboardEvent) => {
        if (!matchesKeymapEntry(OPEN_PALETTE_KEYMAP_ENTRY, event, IS_MAC)) return
        event.preventDefault()
        setMode('commands')
        setQuery('')
        setOpen(true)
    })

    useEffect(() => {
        window.addEventListener('keydown', handleOpenPaletteShortcut, true)
        return () => window.removeEventListener('keydown', handleOpenPaletteShortcut, true)
    }, [])

    const fileRows = (treePage?.rows ?? []).filter((row) => row.kind === 'file')
    const filteredFiles = fuzzyFilter(query, fileRows, (row) => row.path).slice(0, FILE_RESULT_LIMIT)
    const filteredCommands = fuzzyFilter(query, APP_KEYMAP, (entry) => t(entry.descriptionKey))

    const runCommand = (entry: KeymapEntry) => {
        if (entry.id === 'quick-open') {
            setMode('files')
            setQuery('')
            return
        }
        if (entry.id === 'new-terminal') {
            openTerminalTab()
            handleOpenChange(false)
            return
        }
        if (entry.id === 'reopen-closed-tab') {
            if (!activeProjectId) return
            reopenClosedTab(activeProjectId, { onError: (error) => toast.error(error.message) })
            handleOpenChange(false)
            return
        }
        toast.info(t('palette.notRunnable', { description: t(entry.descriptionKey) }))
        handleOpenChange(false)
    }

    const openFile = (path: string) => {
        if (!activeProjectId) return
        const name = path.slice(path.lastIndexOf('/') + 1)
        openTab(
            { projectId: activeProjectId, kind: { kind: 'file', path }, title: name, target: null, preview: true },
            { onError: (error) => toast.error(error.message) },
        )
        handleOpenChange(false)
    }

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogHeader className='sr-only'>
                <DialogTitle>{t('palette.title')}</DialogTitle>
            </DialogHeader>
            <DialogContent className='overflow-hidden p-0' showCloseButton={false}>
                <Command shouldFilter={false} className='bg-panel-background text-app-foreground'>
                    <CommandInput
                        value={query}
                        onValueChange={setQuery}
                        placeholder={mode === 'files' ? t('palette.filePlaceholder') : t('palette.commandPlaceholder')}
                    />
                    <CommandList>
                        <CommandEmpty>{t('palette.noResults')}</CommandEmpty>
                        {mode === 'commands' && (
                            <CommandGroup heading={t('palette.commands')}>
                                {filteredCommands.map(({ item }) => (
                                    <CommandItem key={item.id} onSelect={() => runCommand(item)}>
                                        <Terminal className='size-4' />
                                        <span>{t(item.descriptionKey)}</span>
                                        <CommandShortcut>{keymapShortcutLabel(item)}</CommandShortcut>
                                    </CommandItem>
                                ))}
                            </CommandGroup>
                        )}
                        {mode === 'files' && (
                            <CommandGroup heading={t('palette.files')}>
                                {filteredFiles.map(({ item }) => (
                                    <CommandItem key={item.path} onSelect={() => openFile(item.path)}>
                                        <File className='size-4' />
                                        <span className='truncate'>{item.path}</span>
                                    </CommandItem>
                                ))}
                            </CommandGroup>
                        )}
                    </CommandList>
                </Command>
            </DialogContent>
        </Dialog>
    )
}
