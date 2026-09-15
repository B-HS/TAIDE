/**
 * How many recent projects the app sidebar's `+` menu lists, newest first. Mirrors Rust's
 * `constants::RECENT_PROJECT_MENU_LIMIT`, which caps the native `File > Open Recent` submenu, so
 * both "recent projects" surfaces offer the same depth — a project that fell off one has fallen off
 * the other, and the two never disagree about what "recent" means.
 */
export const RECENT_PROJECT_MENU_LIMIT = 10

/**
 * How many recent projects the Welcome screen lists. Deliberately shorter than
 * {@link RECENT_PROJECT_MENU_LIMIT}: the Welcome list is a full-width column that shares vertical
 * space with the action row and the shortcuts card, while a menu is a popover that can scroll.
 */
export const RECENT_PROJECT_DISPLAY_LIMIT = 8
