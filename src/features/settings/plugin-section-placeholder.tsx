export const PluginSectionPlaceholder = () => (
    <div className='border-app-border text-app-sidebar-icon-default rounded-md border border-dashed px-3 py-4 text-xs'>
        <p>
            <code className='text-app-foreground'>{'{app_data}/plugins/{plugin-id}/'}</code> 아래에{' '}
            <code className='text-app-foreground'>taide-plugin.json</code> 매니페스트를 배치하면 언어·LSP·테마를 추가할 수 있습니다.
        </p>
        <p className='mt-1'>플러그인 목록 조회 커맨드는 코어에 배선되는 대로 이 자리에 표시됩니다.</p>
    </div>
)
