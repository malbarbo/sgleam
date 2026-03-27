use std::sync::{Arc, LazyLock};

static FONT_INTER: &[u8] = include_bytes!("../fonts/InterVariable.ttf");

pub static FONTDB: LazyLock<Arc<resvg::usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut db = resvg::usvg::fontdb::Database::new();
    db.load_font_data(FONT_INTER.to_vec());
    db.load_system_fonts();
    db.set_sans_serif_family("Inter Variable");
    Arc::new(db)
});
