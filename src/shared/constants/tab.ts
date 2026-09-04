/**
 * Title of the `welcome` tab, kept byte-identical to the literal `layout::service::default_layout`
 * writes when it seeds a new project's tabs. `open_tab` reuses an existing tab of the same
 * `TabKind` instead of creating a second one and — unlike the create path — never rewrites the
 * existing tab's `title`, so a localized title passed from the palette would only ever apply to the
 * first Welcome tab of a session and leave every restored one reading `Welcome`. Keeping both
 * producers on one literal makes that reuse invisible; the localized string (`app.welcome`) is used
 * for the command/menu label only.
 */
export const WELCOME_TAB_TITLE = 'Welcome'
