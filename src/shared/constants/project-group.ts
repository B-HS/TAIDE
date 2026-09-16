/**
 * Longest group name the sidebar will accept, counted in Unicode codepoints rather than UTF-16
 * units so a name is measured the way a reader sees it. Mirrors
 * `domain::project::service::GROUP_NAME_MAX_CODEPOINTS`, which rejects anything longer — keeping the
 * same number here is what makes the dialog unable to compose a name the backend would refuse, the
 * same arrangement `PROJECT_LABEL_MAX_CODEPOINTS` has with its own sanitizer.
 */
export const PROJECT_GROUP_NAME_MAX_CODEPOINTS = 40
