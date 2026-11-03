use taide_model::tree::TreeRow;

/// `explorer-auto-reveal.ts` `AutoRevealDecision`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Skip,
    SelectOnly,
    RevealThenSelect,
}

pub(crate) struct Input<'a> {
    pub(crate) is_enabled: bool,
    pub(crate) active_path: Option<&'a str>,
    pub(crate) rows: &'a [TreeRow],
    pub(crate) is_sidebar_visible: bool,
    pub(crate) is_files_view: bool,
    pub(crate) last_revealed: Option<&'a str>,
}

/// Whether the explorer follows the active file: skipped while the tree cannot be seen or the
/// path was the last one acted on, select-only when the page already lists the row.
pub(crate) fn decide(input: Input<'_>) -> Decision {
    let Some(active_path) = input
        .active_path
        .filter(|_| input.is_enabled && input.is_sidebar_visible && input.is_files_view)
    else {
        return Decision::Skip;
    };
    if input.last_revealed == Some(active_path) {
        return Decision::Skip;
    }
    if input.rows.iter().any(|row| row.path == active_path) {
        Decision::SelectOnly
    } else {
        Decision::RevealThenSelect
    }
}

#[cfg(test)]
#[path = "explorer-auto-reveal-tests.rs"]
mod tests;
