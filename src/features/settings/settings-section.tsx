import type { FC, PropsWithChildren } from 'react'

type SettingsSectionProps = PropsWithChildren<{
    title: string
    description?: string
}>

export const SettingsSection: FC<SettingsSectionProps> = ({ title, description, children }) => (
    <section className='border-app-border flex flex-col gap-3 border-b pb-6 last:border-b-0'>
        <div className='flex flex-col gap-0.5'>
            <h2 className='text-app-foreground text-sm font-semibold'>{title}</h2>
            {description && <p className='text-app-sidebar-icon-default text-xs'>{description}</p>}
        </div>
        {children}
    </section>
)
