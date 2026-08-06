use std::collections::BTreeMap;

use crate::domain::locale::types::{LocalePack, LocaleSummary, ResolvedLocale, LOCALE_SCHEMA_VERSION};
use crate::error::{AppError, AppResult};
use crate::infra::persist;
use crate::paths::AppPaths;

pub const BUILTIN_EN_ID: &str = "en";
pub const BUILTIN_KO_ID: &str = "ko";
pub const BUILTIN_JA_ID: &str = "ja";

const MESSAGE_NAMESPACES: &[(&str, &[&str])] = &[
    (
        "common",
        &[
            "cancel", "confirm", "close", "save", "delete", "rename", "open", "loading", "empty", "retry", "reset",
        ],
    ),
    (
        "app",
        &[
            "selectProject",
            "noProjectOpen",
            "openProject",
            "settings",
            "welcome",
            "openFolder",
            "openFolderHint",
            "openProjectFirst",
            "recentItems",
        ],
    ),
    (
        "explorer",
        &["title", "newFile", "newFolder", "reveal", "copyPath", "sidebarSwitchLabel"],
    ),
    (
        "search",
        &[
            "title",
            "placeholder",
            "noResults",
            "matchCount",
            "caseSensitive",
            "wholeWord",
            "searching",
            "noMatches",
            "pressEnterHint",
        ],
    ),
    (
        "git",
        &[
            "title",
            "stage",
            "unstage",
            "discard",
            "commit",
            "push",
            "pull",
            "noChanges",
            "notARepository",
            "noRepositoryLabel",
            "sync",
            "discardTitle",
            "discardDescription",
            "discardConfirm",
            "stageAllTitle",
            "stageAllDescription",
            "commitMessagePlaceholder",
            "committing",
            "timeJustNow",
            "timeDaysAgo",
            "timeHoursAgo",
            "timeMinutesAgo",
        ],
    ),
    ("terminal", &["title", "newTerminal", "processExited", "restart"]),
    (
        "editor",
        &[
            "noFileOpen",
            "unsavedChanges",
            "reloadFromDisk",
            "keepMine",
            "openFailed",
            "cannotOpen",
            "binaryOrTooLarge",
            "readOnlyLargeFile",
            "changedOnDisk",
            "viewDiskContent",
            "renameUnavailable",
            "diffLoadFailed",
        ],
    ),
    (
        "tab",
        &[
            "close",
            "closeOthers",
            "closeAll",
            "closeToRight",
            "pin",
            "unpin",
            "split",
            "moveToSplit",
            "closeAriaLabel",
            "unpinAriaLabel",
        ],
    ),
    ("editorArea", &["splitLeft", "splitRight", "splitTop", "splitBottom"]),
    (
        "palette",
        &[
            "commandPlaceholder",
            "filePlaceholder",
            "noResults",
            "commands",
            "files",
            "title",
            "notRunnable",
        ],
    ),
    (
        "settings",
        &[
            "title",
            "appearance",
            "theme",
            "followSystemTheme",
            "language",
            "editorFontSize",
            "terminalFontSize",
            "shell",
            "lspStatus",
            "plugins",
            "loading",
            "lspDescription",
            "themeDark",
            "themeLight",
            "lspInstalled",
            "lspNotInstalled",
            "pluginsManifestHint",
            "pluginsListPlaceholder",
        ],
    ),
    ("project", &["close", "openInFileManager", "copyPath"]),
    ("sidebar", &["projectsAriaLabel", "openFolderAriaLabel", "settingsAriaLabel"]),
    (
        "keymap",
        &[
            "quickOpen",
            "closeTab",
            "toggleSidebar",
            "search",
            "explorer",
            "split",
            "tabCycleNext",
            "tabCyclePrev",
            "reopenClosedTab",
            "save",
            "toggleTerminal",
            "newTerminal",
            "commandPalette",
        ],
    ),
];

pub fn required_message_keys() -> Vec<String> {
    MESSAGE_NAMESPACES
        .iter()
        .flat_map(|(namespace, keys)| keys.iter().map(move |key| format!("{namespace}.{key}")))
        .collect()
}

fn map_from_pairs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect()
}

fn en_messages() -> BTreeMap<String, String> {
    map_from_pairs(&[
        ("common.cancel", "Cancel"),
        ("common.confirm", "Confirm"),
        ("common.close", "Close"),
        ("common.save", "Save"),
        ("common.delete", "Delete"),
        ("common.rename", "Rename"),
        ("common.open", "Open"),
        ("common.loading", "Loading..."),
        ("common.empty", "Empty"),
        ("common.retry", "Retry"),
        ("common.reset", "Reset"),
        ("app.selectProject", "Select a project"),
        ("app.noProjectOpen", "No project is open"),
        ("app.openProject", "Open Project"),
        ("app.settings", "Settings"),
        ("app.welcome", "Welcome"),
        ("app.openFolder", "Open Folder"),
        ("app.openFolderHint", "Open a folder to start a project"),
        ("app.openProjectFirst", "Open a project first"),
        ("app.recentItems", "Recent"),
        ("explorer.title", "Explorer"),
        ("explorer.newFile", "New File"),
        ("explorer.newFolder", "New Folder"),
        ("explorer.reveal", "Reveal in Explorer"),
        ("explorer.copyPath", "Copy Path"),
        ("explorer.sidebarSwitchLabel", "Switch sidebar view"),
        ("search.title", "Search"),
        ("search.placeholder", "Search"),
        ("search.noResults", "No results"),
        ("search.matchCount", "{{count}} results in {{files}} files"),
        ("search.caseSensitive", "Match Case"),
        ("search.wholeWord", "Match Whole Word"),
        ("search.searching", "Searching…"),
        ("search.noMatches", "No matching results"),
        ("search.pressEnterHint", "Type a search term and press Enter"),
        ("git.title", "Git"),
        ("git.stage", "Stage"),
        ("git.unstage", "Unstage"),
        ("git.discard", "Discard Changes"),
        ("git.commit", "Commit"),
        ("git.push", "Push"),
        ("git.pull", "Pull"),
        ("git.noChanges", "No changes"),
        ("git.notARepository", "This project is not a Git repository"),
        ("git.noRepositoryLabel", "No repository"),
        ("git.sync", "Sync"),
        ("git.discardTitle", "Discard these changes?"),
        ("git.fileCount", "{{count}} files"),
        (
            "git.discardDescription",
            "This reverts changes in {{target}}. This action cannot be undone.",
        ),
        ("git.discardConfirm", "Discard Changes"),
        ("git.stageAllTitle", "Stage all changes and commit?"),
        (
            "git.stageAllDescription",
            "There are no staged changes. All changes will be staged before committing.",
        ),
        ("git.commitMessagePlaceholder", "Enter commit message"),
        ("git.committing", "Committing…"),
        ("git.timeJustNow", "just now"),
        ("git.timeDaysAgo", "{{n}}d ago"),
        ("git.timeHoursAgo", "{{n}}h ago"),
        ("git.timeMinutesAgo", "{{n}}m ago"),
        ("terminal.title", "Terminal"),
        ("terminal.newTerminal", "New Terminal"),
        ("terminal.processExited", "Process exited"),
        ("terminal.restart", "Restart"),
        ("editor.noFileOpen", "No file is open"),
        ("editor.unsavedChanges", "You have unsaved changes"),
        ("editor.reloadFromDisk", "Reload from Disk"),
        ("editor.keepMine", "Keep My Changes"),
        ("editor.openFailed", "Failed to open file"),
        ("editor.cannotOpen", "This file cannot be opened in the editor"),
        ("editor.binaryOrTooLarge", "It is binary or too large"),
        ("editor.readOnlyLargeFile", "Opened as read-only because the file is large"),
        ("editor.changedOnDisk", "Changed on disk"),
        ("editor.viewDiskContent", "View Disk Content"),
        ("editor.renameUnavailable", "This position cannot be renamed"),
        ("editor.diffLoadFailed", "Failed to load diff"),
        ("tab.close", "Close"),
        ("tab.closeOthers", "Close Other Tabs"),
        ("tab.closeAll", "Close All Tabs"),
        ("tab.closeToRight", "Close Tabs to the Right"),
        ("tab.pin", "Pin"),
        ("tab.unpin", "Unpin"),
        ("tab.split", "Split"),
        ("tab.moveToSplit", "Move to Split"),
        ("tab.closeAriaLabel", "Close {title}"),
        ("tab.unpinAriaLabel", "Unpin {title}"),
        ("editorArea.splitLeft", "Split Left"),
        ("editorArea.splitRight", "Split Right"),
        ("editorArea.splitTop", "Split Up"),
        ("editorArea.splitBottom", "Split Down"),
        ("palette.commandPlaceholder", "Type a command to run..."),
        ("palette.filePlaceholder", "Search by file name..."),
        ("palette.noResults", "No results"),
        ("palette.commands", "Commands"),
        ("palette.files", "Files"),
        ("palette.title", "Command Palette"),
        (
            "palette.notRunnable",
            "\"{{description}}\" cannot be run from the command palette yet",
        ),
        ("settings.title", "Settings"),
        ("settings.appearance", "Appearance"),
        ("settings.editor", "Editor"),
        ("settings.terminal", "Terminal"),
        ("settings.systemLanguage", "System"),
        ("settings.theme", "Theme"),
        ("settings.followSystemTheme", "Follow system theme"),
        ("settings.language", "Language"),
        ("settings.editorFontSize", "Font Size"),
        ("settings.terminalFontSize", "Font Size"),
        ("settings.shell", "Shell Override"),
        ("settings.lspStatus", "LSP Servers"),
        ("settings.plugins", "Plugins"),
        ("settings.loading", "Loading..."),
        (
            "settings.lspDescription",
            "Detected status of language servers installed on the system.",
        ),
        ("settings.themeDark", "Dark"),
        ("settings.themeLight", "Light"),
        ("settings.lspInstalled", "Installed"),
        ("settings.lspNotInstalled", "Not Installed"),
        (
            "settings.pluginsManifestHint",
            "Place a taide-plugin.json manifest under {app_data}/plugins/{plugin-id}/ to add languages, LSPs, and themes.",
        ),
        (
            "settings.pluginsListPlaceholder",
            "The plugin list command will appear here once it is wired up in the core.",
        ),
        ("project.close", "Close Project"),
        ("project.openInFileManager", "Open in File Manager"),
        ("project.copyPath", "Copy Path"),
        ("sidebar.projectsAriaLabel", "Projects"),
        ("sidebar.openFolderAriaLabel", "Open Folder"),
        ("sidebar.settingsAriaLabel", "Settings"),
        ("keymap.quickOpen", "Quick Open File"),
        ("keymap.closeTab", "Close Tab"),
        ("keymap.toggleSidebar", "Toggle Sidebar"),
        ("keymap.search", "Search"),
        ("keymap.explorer", "Explorer"),
        ("keymap.split", "Split Editor"),
        ("keymap.tabCycleNext", "Next Tab"),
        ("keymap.tabCyclePrev", "Previous Tab"),
        ("keymap.reopenClosedTab", "Reopen Closed Tab"),
        ("keymap.save", "Save"),
        ("keymap.toggleTerminal", "Toggle Terminal"),
        ("keymap.newTerminal", "New Terminal"),
        ("keymap.commandPalette", "Command Palette"),
    ])
}

fn ko_messages() -> BTreeMap<String, String> {
    map_from_pairs(&[
        ("common.cancel", "취소"),
        ("common.confirm", "확인"),
        ("common.close", "닫기"),
        ("common.save", "저장"),
        ("common.delete", "삭제"),
        ("common.rename", "이름 변경"),
        ("common.open", "열기"),
        ("common.loading", "불러오는 중..."),
        ("common.empty", "비어 있음"),
        ("common.retry", "다시 시도"),
        ("common.reset", "초기화"),
        ("app.selectProject", "프로젝트를 선택하세요"),
        ("app.noProjectOpen", "열려 있는 프로젝트가 없습니다"),
        ("app.openProject", "프로젝트 열기"),
        ("app.settings", "설정"),
        ("app.welcome", "환영합니다"),
        ("app.openFolder", "폴더 열기"),
        ("app.openFolderHint", "폴더를 열어 프로젝트를 시작하세요"),
        ("app.openProjectFirst", "먼저 프로젝트를 여세요"),
        ("app.recentItems", "최근 항목"),
        ("explorer.title", "탐색기"),
        ("explorer.newFile", "새 파일"),
        ("explorer.newFolder", "새 폴더"),
        ("explorer.reveal", "파일 위치 열기"),
        ("explorer.copyPath", "경로 복사"),
        ("explorer.sidebarSwitchLabel", "탐색 사이드바 전환"),
        ("search.title", "검색"),
        ("search.placeholder", "검색어 입력"),
        ("search.noResults", "결과 없음"),
        ("search.matchCount", "{{count}}개 결과, 파일 {{files}}개"),
        ("search.caseSensitive", "대소문자 구분"),
        ("search.wholeWord", "단어 단위 검색"),
        ("search.searching", "검색 중…"),
        ("search.noMatches", "일치하는 결과가 없습니다"),
        ("search.pressEnterHint", "검색어를 입력하고 Enter 를 누르세요"),
        ("git.title", "Git"),
        ("git.stage", "스테이지"),
        ("git.unstage", "언스테이지"),
        ("git.discard", "변경 취소"),
        ("git.commit", "커밋"),
        ("git.push", "푸시"),
        ("git.pull", "풀"),
        ("git.noChanges", "변경사항 없음"),
        ("git.notARepository", "이 프로젝트는 Git 저장소가 아닙니다"),
        ("git.noRepositoryLabel", "리포지토리 없음"),
        ("git.sync", "동기화"),
        ("git.discardTitle", "변경사항을 취소할까요?"),
        ("git.fileCount", "{{count}}개 파일"),
        (
            "git.discardDescription",
            "{{target}}의 변경사항을 되돌립니다. 이 작업은 취소할 수 없습니다.",
        ),
        ("git.discardConfirm", "변경 취소"),
        ("git.stageAllTitle", "변경 전체를 스테이지하고 커밋할까요?"),
        (
            "git.stageAllDescription",
            "스테이지된 변경사항이 없습니다. 모든 변경사항을 스테이지한 뒤 커밋합니다.",
        ),
        ("git.commitMessagePlaceholder", "커밋 메시지 입력"),
        ("git.committing", "커밋 중…"),
        ("git.timeJustNow", "방금"),
        ("git.timeDaysAgo", "{{n}}일 전"),
        ("git.timeHoursAgo", "{{n}}시간 전"),
        ("git.timeMinutesAgo", "{{n}}분 전"),
        ("terminal.title", "터미널"),
        ("terminal.newTerminal", "새 터미널"),
        ("terminal.processExited", "프로세스가 종료되었습니다"),
        ("terminal.restart", "다시 시작"),
        ("editor.noFileOpen", "파일을 열지 않았습니다"),
        ("editor.unsavedChanges", "저장하지 않은 변경사항이 있습니다"),
        ("editor.reloadFromDisk", "디스크에서 다시 불러오기"),
        ("editor.keepMine", "내 변경 유지"),
        ("editor.openFailed", "파일을 열지 못했습니다"),
        ("editor.cannotOpen", "이 파일은 에디터로 열 수 없습니다"),
        ("editor.binaryOrTooLarge", "바이너리이거나 크기가 너무 큽니다"),
        ("editor.readOnlyLargeFile", "큰 파일이라 열람 전용으로 열렸습니다"),
        ("editor.changedOnDisk", "디스크에서 변경됨"),
        ("editor.viewDiskContent", "디스크 내용 보기"),
        ("editor.renameUnavailable", "이름을 바꿀 수 없는 위치입니다"),
        ("editor.diffLoadFailed", "diff 를 불러오지 못했습니다"),
        ("tab.close", "닫기"),
        ("tab.closeOthers", "다른 탭 모두 닫기"),
        ("tab.closeAll", "모든 탭 닫기"),
        ("tab.closeToRight", "오른쪽 탭 닫기"),
        ("tab.pin", "고정"),
        ("tab.unpin", "고정 해제"),
        ("tab.split", "분할"),
        ("tab.moveToSplit", "스플릿으로 이동"),
        ("tab.closeAriaLabel", "{title} 닫기"),
        ("tab.unpinAriaLabel", "{title} 고정 해제"),
        ("editorArea.splitLeft", "왼쪽으로 분할"),
        ("editorArea.splitRight", "오른쪽으로 분할"),
        ("editorArea.splitTop", "위로 분할"),
        ("editorArea.splitBottom", "아래로 분할"),
        ("palette.commandPlaceholder", "실행할 명령을 입력하세요..."),
        ("palette.filePlaceholder", "파일 이름으로 검색..."),
        ("palette.noResults", "결과가 없습니다"),
        ("palette.commands", "명령"),
        ("palette.files", "파일"),
        ("palette.title", "커맨드 팔레트"),
        (
            "palette.notRunnable",
            "\"{{description}}\"은 아직 커맨드 팔레트에서 실행할 수 없습니다",
        ),
        ("settings.title", "설정"),
        ("settings.appearance", "모양"),
        ("settings.editor", "에디터"),
        ("settings.terminal", "터미널"),
        ("settings.systemLanguage", "시스템 설정"),
        ("settings.theme", "테마"),
        ("settings.followSystemTheme", "시스템 테마를 따라간다"),
        ("settings.language", "언어"),
        ("settings.editorFontSize", "폰트 크기"),
        ("settings.terminalFontSize", "폰트 크기"),
        ("settings.shell", "셸 오버라이드"),
        ("settings.lspStatus", "LSP 서버"),
        ("settings.plugins", "플러그인"),
        ("settings.loading", "불러오는 중..."),
        ("settings.lspDescription", "시스템에 설치된 언어 서버 감지 상태입니다."),
        ("settings.themeDark", "다크"),
        ("settings.themeLight", "라이트"),
        ("settings.lspInstalled", "설치됨"),
        ("settings.lspNotInstalled", "미설치"),
        (
            "settings.pluginsManifestHint",
            "{app_data}/plugins/{plugin-id}/ 아래에 taide-plugin.json 매니페스트를 배치하면 언어·LSP·테마를 추가할 수 있습니다.",
        ),
        (
            "settings.pluginsListPlaceholder",
            "플러그인 목록 조회 커맨드는 코어에 배선되는 대로 이 자리에 표시됩니다.",
        ),
        ("project.close", "프로젝트 닫기"),
        ("project.openInFileManager", "파일 관리자에서 열기"),
        ("project.copyPath", "경로 복사"),
        ("sidebar.projectsAriaLabel", "프로젝트"),
        ("sidebar.openFolderAriaLabel", "폴더 열기"),
        ("sidebar.settingsAriaLabel", "설정"),
        ("keymap.quickOpen", "파일 퀵 오픈"),
        ("keymap.closeTab", "탭 닫기"),
        ("keymap.toggleSidebar", "사이드바 토글"),
        ("keymap.search", "검색"),
        ("keymap.explorer", "탐색기"),
        ("keymap.split", "에디터 분할"),
        ("keymap.tabCycleNext", "다음 탭"),
        ("keymap.tabCyclePrev", "이전 탭"),
        ("keymap.reopenClosedTab", "닫은 탭 다시 열기"),
        ("keymap.save", "저장"),
        ("keymap.toggleTerminal", "터미널 토글"),
        ("keymap.newTerminal", "새 터미널"),
        ("keymap.commandPalette", "커맨드 팔레트"),
    ])
}

fn ja_messages() -> BTreeMap<String, String> {
    map_from_pairs(&[
        ("common.cancel", "キャンセル"),
        ("common.confirm", "確認"),
        ("common.close", "閉じる"),
        ("common.save", "保存"),
        ("common.delete", "削除"),
        ("common.rename", "名前の変更"),
        ("common.open", "開く"),
        ("common.loading", "読み込み中..."),
        ("common.empty", "空です"),
        ("common.retry", "再試行"),
        ("common.reset", "リセット"),
        ("app.selectProject", "プロジェクトを選択してください"),
        ("app.noProjectOpen", "開いているプロジェクトがありません"),
        ("app.openProject", "プロジェクトを開く"),
        ("app.settings", "設定"),
        ("app.welcome", "ようこそ"),
        ("app.openFolder", "フォルダを開く"),
        ("app.openFolderHint", "フォルダを開いてプロジェクトを開始してください"),
        ("app.openProjectFirst", "先にプロジェクトを開いてください"),
        ("app.recentItems", "最近の項目"),
        ("explorer.title", "エクスプローラー"),
        ("explorer.newFile", "新規ファイル"),
        ("explorer.newFolder", "新規フォルダ"),
        ("explorer.reveal", "エクスプローラーで表示"),
        ("explorer.copyPath", "パスをコピー"),
        ("explorer.sidebarSwitchLabel", "サイドバー表示切替"),
        ("search.title", "検索"),
        ("search.placeholder", "検索キーワードを入力"),
        ("search.noResults", "結果なし"),
        ("search.matchCount", "{{count}} 件の結果、{{files}} ファイル"),
        ("search.caseSensitive", "大文字小文字を区別"),
        ("search.wholeWord", "単語単位で検索"),
        ("search.searching", "検索中…"),
        ("search.noMatches", "一致する結果がありません"),
        ("search.pressEnterHint", "検索語を入力してEnterを押してください"),
        ("git.title", "Git"),
        ("git.stage", "ステージ"),
        ("git.unstage", "ステージ解除"),
        ("git.discard", "変更を破棄"),
        ("git.commit", "コミット"),
        ("git.push", "プッシュ"),
        ("git.pull", "プル"),
        ("git.noChanges", "変更なし"),
        ("git.notARepository", "このプロジェクトはGitリポジトリではありません"),
        ("git.noRepositoryLabel", "リポジトリなし"),
        ("git.sync", "同期"),
        ("git.discardTitle", "変更を破棄しますか?"),
        ("git.fileCount", "{{count}}個のファイル"),
        (
            "git.discardDescription",
            "{{target}} の変更を元に戻します。この操作は取り消せません。",
        ),
        ("git.discardConfirm", "変更を破棄"),
        ("git.stageAllTitle", "すべての変更をステージしてコミットしますか?"),
        (
            "git.stageAllDescription",
            "ステージされた変更がありません。すべての変更をステージしてからコミットします。",
        ),
        ("git.commitMessagePlaceholder", "コミットメッセージを入力"),
        ("git.committing", "コミット中…"),
        ("git.timeJustNow", "たった今"),
        ("git.timeDaysAgo", "{{n}}日前"),
        ("git.timeHoursAgo", "{{n}}時間前"),
        ("git.timeMinutesAgo", "{{n}}分前"),
        ("terminal.title", "ターミナル"),
        ("terminal.newTerminal", "新規ターミナル"),
        ("terminal.processExited", "プロセスが終了しました"),
        ("terminal.restart", "再起動"),
        ("editor.noFileOpen", "ファイルが開かれていません"),
        ("editor.unsavedChanges", "未保存の変更があります"),
        ("editor.reloadFromDisk", "ディスクから再読み込み"),
        ("editor.keepMine", "自分の変更を保持"),
        ("editor.openFailed", "ファイルを開けませんでした"),
        ("editor.cannotOpen", "このファイルはエディタで開けません"),
        ("editor.binaryOrTooLarge", "バイナリまたはサイズが大きすぎます"),
        ("editor.readOnlyLargeFile", "ファイルが大きいため読み取り専用で開きました"),
        ("editor.changedOnDisk", "ディスクで変更されました"),
        ("editor.viewDiskContent", "ディスクの内容を表示"),
        ("editor.renameUnavailable", "この位置は名前を変更できません"),
        ("editor.diffLoadFailed", "diffの読み込みに失敗しました"),
        ("tab.close", "閉じる"),
        ("tab.closeOthers", "他のタブをすべて閉じる"),
        ("tab.closeAll", "すべてのタブを閉じる"),
        ("tab.closeToRight", "右側のタブを閉じる"),
        ("tab.pin", "ピン留め"),
        ("tab.unpin", "ピン留め解除"),
        ("tab.split", "分割"),
        ("tab.moveToSplit", "分割に移動"),
        ("tab.closeAriaLabel", "{title} を閉じる"),
        ("tab.unpinAriaLabel", "{title} のピン留めを解除"),
        ("editorArea.splitLeft", "左に分割"),
        ("editorArea.splitRight", "右に分割"),
        ("editorArea.splitTop", "上に分割"),
        ("editorArea.splitBottom", "下に分割"),
        ("palette.commandPlaceholder", "実行するコマンドを入力..."),
        ("palette.filePlaceholder", "ファイル名で検索..."),
        ("palette.noResults", "結果がありません"),
        ("palette.commands", "コマンド"),
        ("palette.files", "ファイル"),
        ("palette.title", "コマンドパレット"),
        ("palette.notRunnable", "「{{description}}」はまだコマンドパレットから実行できません"),
        ("settings.title", "設定"),
        ("settings.appearance", "外観"),
        ("settings.editor", "エディター"),
        ("settings.terminal", "ターミナル"),
        ("settings.systemLanguage", "システム設定"),
        ("settings.theme", "テーマ"),
        ("settings.followSystemTheme", "システムテーマに従う"),
        ("settings.language", "言語"),
        ("settings.editorFontSize", "フォントサイズ"),
        ("settings.terminalFontSize", "フォントサイズ"),
        ("settings.shell", "シェルオーバーライド"),
        ("settings.lspStatus", "LSPサーバー"),
        ("settings.plugins", "プラグイン"),
        ("settings.loading", "読み込み中..."),
        (
            "settings.lspDescription",
            "システムにインストールされた言語サーバーの検出状態です。",
        ),
        ("settings.themeDark", "ダーク"),
        ("settings.themeLight", "ライト"),
        ("settings.lspInstalled", "インストール済み"),
        ("settings.lspNotInstalled", "未インストール"),
        (
            "settings.pluginsManifestHint",
            "{app_data}/plugins/{plugin-id}/ の下に taide-plugin.json マニフェストを配置すると、言語・LSP・テーマを追加できます。",
        ),
        (
            "settings.pluginsListPlaceholder",
            "プラグイン一覧取得コマンドはコアに配線され次第、ここに表示されます。",
        ),
        ("project.close", "プロジェクトを閉じる"),
        ("project.openInFileManager", "ファイルマネージャーで開く"),
        ("project.copyPath", "パスをコピー"),
        ("sidebar.projectsAriaLabel", "プロジェクト"),
        ("sidebar.openFolderAriaLabel", "フォルダを開く"),
        ("sidebar.settingsAriaLabel", "設定"),
        ("keymap.quickOpen", "ファイルのクイックオープン"),
        ("keymap.closeTab", "タブを閉じる"),
        ("keymap.toggleSidebar", "サイドバー切替"),
        ("keymap.search", "検索"),
        ("keymap.explorer", "エクスプローラー"),
        ("keymap.split", "エディタを分割"),
        ("keymap.tabCycleNext", "次のタブ"),
        ("keymap.tabCyclePrev", "前のタブ"),
        ("keymap.reopenClosedTab", "閉じたタブを再度開く"),
        ("keymap.save", "保存"),
        ("keymap.toggleTerminal", "ターミナル切替"),
        ("keymap.newTerminal", "新規ターミナル"),
        ("keymap.commandPalette", "コマンドパレット"),
    ])
}

pub fn builtin_en() -> LocalePack {
    LocalePack {
        version: LOCALE_SCHEMA_VERSION,
        id: BUILTIN_EN_ID.to_string(),
        name: "English".to_string(),
        extends: None,
        messages: en_messages(),
    }
}

pub fn builtin_ko() -> LocalePack {
    LocalePack {
        version: LOCALE_SCHEMA_VERSION,
        id: BUILTIN_KO_ID.to_string(),
        name: "한국어".to_string(),
        extends: None,
        messages: ko_messages(),
    }
}

pub fn builtin_ja() -> LocalePack {
    LocalePack {
        version: LOCALE_SCHEMA_VERSION,
        id: BUILTIN_JA_ID.to_string(),
        name: "日本語".to_string(),
        extends: None,
        messages: ja_messages(),
    }
}

pub fn builtin_by_id(locale_id: &str) -> Option<LocalePack> {
    match locale_id {
        BUILTIN_EN_ID => Some(builtin_en()),
        BUILTIN_KO_ID => Some(builtin_ko()),
        BUILTIN_JA_ID => Some(builtin_ja()),
        _ => None,
    }
}

fn summarize(pack: &LocalePack, builtin: bool) -> LocaleSummary {
    LocaleSummary {
        id: pack.id.clone(),
        name: pack.name.clone(),
        builtin,
    }
}

fn resolve_pack(pack: &LocalePack, base: Option<&LocalePack>, mut warnings: Vec<String>) -> ResolvedLocale {
    let mut messages = pack.messages.clone();

    if let Some(base_pack) = base {
        for (key, value) in &base_pack.messages {
            messages.entry(key.clone()).or_insert_with(|| {
                warnings.push(format!("messages.{key} filled from base locale '{}'", base_pack.id));
                value.clone()
            });
        }
    }

    ResolvedLocale {
        id: pack.id.clone(),
        name: pack.name.clone(),
        messages,
        warnings,
    }
}

pub fn list_locales(paths: &AppPaths) -> Vec<LocaleSummary> {
    let mut list = vec![
        summarize(&builtin_en(), true),
        summarize(&builtin_ko(), true),
        summarize(&builtin_ja(), true),
    ];

    let Ok(entries) = std::fs::read_dir(paths.locales_dir()) else {
        return list;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if let Ok(Some(pack)) = persist::read_json::<LocalePack>(&path) {
            match list.iter_mut().find(|summary| summary.id == pack.id) {
                Some(existing) => *existing = summarize(&pack, false),
                None => list.push(summarize(&pack, false)),
            }
        }
    }

    list
}

pub fn load_locale(paths: &AppPaths, locale_id: &str) -> AppResult<ResolvedLocale> {
    if let Some(builtin) = builtin_by_id(locale_id) {
        return Ok(resolve_pack(&builtin, None, Vec::new()));
    }

    let path = paths.locales_dir().join(format!("{locale_id}.json"));
    let pack: LocalePack = persist::read_json(&path)?.ok_or_else(|| AppError::NotFound(format!("locale not found: {locale_id}")))?;

    let mut warnings = Vec::new();
    let base = match pack.extends.as_deref() {
        Some(extends_id) => builtin_by_id(extends_id).unwrap_or_else(|| {
            warnings.push(format!(
                "unresolved locale extends '{extends_id}', falling back to '{BUILTIN_EN_ID}'"
            ));
            builtin_en()
        }),
        None => builtin_en(),
    };

    Ok(resolve_pack(&pack, Some(&base), warnings))
}

pub fn locale_exists(paths: &AppPaths, locale_id: &str) -> bool {
    builtin_by_id(locale_id).is_some() || paths.locales_dir().join(format!("{locale_id}.json")).exists()
}

pub fn resolve_language(paths: &AppPaths, language: &str, system_language: &str) -> String {
    if language != "system" {
        if locale_exists(paths, language) {
            return language.to_string();
        }
        return BUILTIN_EN_ID.to_string();
    }

    let lower = system_language.to_lowercase();
    let prefix = lower.get(0..2).unwrap_or("");

    match prefix {
        "ko" => BUILTIN_KO_ID.to_string(),
        "ja" => BUILTIN_JA_ID.to_string(),
        _ => BUILTIN_EN_ID.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-locale-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn 내장_3종_로케일은_같은_키_집합을_가진다() {
        let en_keys: std::collections::BTreeSet<_> = builtin_en().messages.keys().cloned().collect();
        let ko_keys: std::collections::BTreeSet<_> = builtin_ko().messages.keys().cloned().collect();
        let ja_keys: std::collections::BTreeSet<_> = builtin_ja().messages.keys().cloned().collect();

        assert_eq!(en_keys, ko_keys);
        assert_eq!(en_keys, ja_keys);

        for key in required_message_keys() {
            assert!(en_keys.contains(&key), "missing message key: {key}");
        }
    }

    #[test]
    fn extends_병합이_누락_키를_base로_채운다() {
        let data_dir = temp_data_dir("extends-merge");
        let paths = AppPaths::new(data_dir);
        std::fs::create_dir_all(paths.locales_dir()).expect("create locales dir");

        let mut messages = BTreeMap::new();
        messages.insert("common.cancel".to_string(), "그만".to_string());
        let custom = LocalePack {
            version: LOCALE_SCHEMA_VERSION,
            id: "custom-ko".to_string(),
            name: "Custom Korean".to_string(),
            extends: Some(BUILTIN_KO_ID.to_string()),
            messages,
        };
        persist::write_json(&paths.locales_dir().join("custom-ko.json"), &custom).expect("write custom locale");

        let resolved = load_locale(&paths, "custom-ko").expect("load custom locale");

        assert_eq!(resolved.messages.get("common.cancel"), Some(&"그만".to_string()));
        assert_eq!(resolved.messages.get("common.save"), builtin_ko().messages.get("common.save"));
        assert!(!resolved.warnings.is_empty());
        for key in required_message_keys() {
            assert!(resolved.messages.contains_key(&key));
        }

        std::fs::remove_dir_all(paths.locales_dir()).ok();
    }

    #[test]
    fn 해석_불가_extends는_경고를_남기고_en으로_폴백한다() {
        let data_dir = temp_data_dir("extends-unresolved");
        let paths = AppPaths::new(data_dir);
        std::fs::create_dir_all(paths.locales_dir()).expect("create locales dir");

        let custom = LocalePack {
            version: LOCALE_SCHEMA_VERSION,
            id: "custom-broken".to_string(),
            name: "Broken".to_string(),
            extends: Some("does-not-exist".to_string()),
            messages: BTreeMap::new(),
        };
        persist::write_json(&paths.locales_dir().join("custom-broken.json"), &custom).expect("write broken locale");

        let resolved = load_locale(&paths, "custom-broken").expect("load broken locale");

        assert_eq!(resolved.messages.get("common.cancel"), builtin_en().messages.get("common.cancel"));
        assert!(resolved
            .warnings
            .iter()
            .any(|warning| warning.contains("unresolved locale extends")));

        std::fs::remove_dir_all(paths.locales_dir()).ok();
    }

    #[test]
    fn 없는_로케일_아이디는_notfound를_반환한다() {
        let paths = AppPaths::new(temp_data_dir("missing"));
        let result = load_locale(&paths, "does-not-exist");
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[test]
    fn list_locales는_내장_3종과_사용자_팩을_반환하고_겹치면_사용자_팩이_이긴다() {
        let data_dir = temp_data_dir("list");
        let paths = AppPaths::new(data_dir);
        std::fs::create_dir_all(paths.locales_dir()).expect("create locales dir");

        let mut overriding_en = builtin_en();
        overriding_en.name = "English (custom)".to_string();
        persist::write_json(&paths.locales_dir().join("en.json"), &overriding_en).expect("write overriding pack");

        let list = list_locales(&paths);

        assert_eq!(list.len(), 3);
        let en_summary = list.iter().find(|summary| summary.id == BUILTIN_EN_ID).expect("en summary exists");
        assert!(!en_summary.builtin);
        assert_eq!(en_summary.name, "English (custom)");

        std::fs::remove_dir_all(paths.locales_dir()).ok();
    }

    #[test]
    fn resolve_language는_system을_시스템_언어로_매핑한다() {
        let paths = AppPaths::new(temp_data_dir("resolve-system"));
        assert_eq!(resolve_language(&paths, "system", "ko-KR"), BUILTIN_KO_ID);
        assert_eq!(resolve_language(&paths, "system", "ja-JP"), BUILTIN_JA_ID);
        assert_eq!(resolve_language(&paths, "system", "fr-FR"), BUILTIN_EN_ID);
    }

    #[test]
    fn resolve_language는_명시된_언어를_그대로_사용한다() {
        let paths = AppPaths::new(temp_data_dir("resolve-explicit"));
        assert_eq!(resolve_language(&paths, BUILTIN_JA_ID, "en-US"), BUILTIN_JA_ID);
    }

    #[test]
    fn resolve_language는_존재하지_않는_언어를_en으로_폴백한다() {
        let paths = AppPaths::new(temp_data_dir("resolve-missing"));
        assert_eq!(resolve_language(&paths, "xx", "en-US"), BUILTIN_EN_ID);
    }
}
