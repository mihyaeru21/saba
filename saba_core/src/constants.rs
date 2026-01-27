pub static WINDOW_WIDTH: i64 = 600;
pub static WINDOW_HEIGHT: i64 = 400;
pub static WINDOW_PADDING: i64 = 5;

// noli 側で定義されている値なので変更不可
pub static TITLE_BAR_HEIGHT: i64 = 24;

pub static TOOLBAR_HEIGHT: i64 = 26;

pub static CONTENT_AREA_WIDTH: i64 = WINDOW_WIDTH - WINDOW_PADDING * 2;
pub static CONTENT_AREA_HRIGHT: i64 =
    WINDOW_HEIGHT - TITLE_BAR_HEIGHT - TOOLBAR_HEIGHT - WINDOW_PADDING * 2;

pub static CHAR_WIDTH: i64 = 8;
pub static CHAR_HEIGHT: i64 = 16;
pub static CHAR_HEIGHT_WITH_PADDING: i64 = CHAR_HEIGHT + 4;
