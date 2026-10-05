use std::sync::{Arc, OnceLock};

use resvg::usvg::fontdb::Database;

static FONTS: OnceLock<Arc<Database>> = OnceLock::new();

pub(crate) fn database() -> Arc<Database> {
    FONTS
        .get_or_init(|| {
            let mut fonts = Database::new();
            fonts.load_system_fonts();
            Arc::new(fonts)
        })
        .clone()
}
