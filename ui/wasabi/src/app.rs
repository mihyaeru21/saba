use core::cell::RefCell;

use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
};
use noli::{
    error::Result as OsResult,
    prelude::{Api, MouseEvent, SystemApi},
    println,
    rect::Rect,
    window::{StringSize, Window},
};
use saba_core::{
    browser::Browser,
    constants::{
        ADDRESSBAR_HEIGHT, BLACK, CONTENT_AREA_HRIGHT, CONTENT_AREA_WIDTH, DARKGREY, GREY,
        LIGHTGREY, TITLE_BAR_HEIGHT, TOOLBAR_HEIGHT, WHITE, WINDOW_HEIGHT, WINDOW_INIT_X_POS,
        WINDOW_INIT_Y_POS, WINDOW_WIDTH,
    },
    error::Error,
    http::HttpResponse,
};

use crate::cursor::Cursor;

type UrlHandler = fn(String) -> Result<HttpResponse, Error>;

pub struct WasabiUI {
    browser: Rc<RefCell<Browser>>,
    input_url: String,
    input_mode: InputMode,
    window: Window,
    cursor: Cursor,
}

impl WasabiUI {
    pub fn new(browser: Rc<RefCell<Browser>>) -> Self {
        Self {
            browser,
            input_url: String::new(),
            input_mode: InputMode::Normal,
            window: Window::new(
                "saba".to_string(),
                WHITE,
                WINDOW_INIT_X_POS,
                WINDOW_INIT_Y_POS,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
            )
            .unwrap(),
            cursor: Cursor::new(),
        }
    }

    pub fn start(&mut self, handle_url: UrlHandler) -> Result<(), Error> {
        self.setup()?;

        self.run_app(handle_url)?;

        Ok(())
    }

    fn setup(&mut self) -> Result<(), Error> {
        self.setup_toolbar().map_err(|e| {
            Error::InvalidUI(format!(
                "failed to initialize a toolbar with error: {:#?}",
                e
            ))
        })?;

        self.window.flush();

        Ok(())
    }

    fn setup_toolbar(&mut self) -> OsResult<()> {
        // ツールバーの背景の四角を描画
        self.window
            .fill_rect(LIGHTGREY, 0, 0, WINDOW_WIDTH, TOOLBAR_HEIGHT)?;

        // ツールバーとコンテンツエリアの境目の線を描画
        self.window
            .draw_line(GREY, 0, TOOLBAR_HEIGHT, WINDOW_WIDTH - 1, TOOLBAR_HEIGHT)?;
        self.window.draw_line(
            DARKGREY,
            0,
            TOOLBAR_HEIGHT + 1,
            WINDOW_WIDTH - 1,
            TOOLBAR_HEIGHT + 1,
        )?;

        // アドレスバーの横に "Address:" という文字列を描画
        self.window.draw_string(
            BLACK,
            5,
            5,
            "Address:",
            noli::window::StringSize::Medium,
            false,
        )?;

        // アドレスバーの四角を描画
        self.window
            .fill_rect(WHITE, 70, 2, WINDOW_WIDTH - 74, 2 + ADDRESSBAR_HEIGHT)?;

        // アドレスバーの影の線を描画
        self.window.draw_line(GREY, 70, 2, WINDOW_WIDTH - 4, 2)?;
        self.window
            .draw_line(BLACK, 70, 2, 70, 2 + ADDRESSBAR_HEIGHT)?;
        self.window.draw_line(BLACK, 71, 3, WINDOW_WIDTH - 5, 3)?;

        self.window
            .draw_line(GREY, 71, 3, 71, 1 + ADDRESSBAR_HEIGHT)?;

        Ok(())
    }

    fn run_app(&mut self, handle_url: UrlHandler) -> Result<(), Error> {
        loop {
            self.handle_mouse_input()?;
            self.handle_key_input(handle_url)?;
        }
    }

    fn handle_mouse_input(&mut self) -> Result<(), Error> {
        struct Position {
            x: i64,
            y: i64,
        }

        let Some(MouseEvent { button, position }) = Api::get_mouse_cursor_info() else {
            return Ok(());
        };

        self.window.flush_area(self.cursor.rect());
        self.cursor.set_position(position.x, position.y);
        self.window.flush_area(self.cursor.rect());
        self.cursor.flush();

        if !button.l() && !button.c() && !button.r() {
            return Ok(());
        }

        let relative_pos = Position {
            x: position.x - WINDOW_INIT_X_POS,
            y: position.y - WINDOW_INIT_Y_POS,
        };

        // ウィンドウ外
        if relative_pos.x < 0
            || relative_pos.x > WINDOW_WIDTH
            || relative_pos.y < 0
            || relative_pos.y > WINDOW_HEIGHT
        {
            println!("button clicked OUTSIDE window: {button:?} {position:?}");
            return Ok(());
        }

        // ツールバー
        if relative_pos.y < TOOLBAR_HEIGHT + TITLE_BAR_HEIGHT && relative_pos.y >= TITLE_BAR_HEIGHT
        {
            self.clear_address_bar()?;
            self.input_url = String::new();
            self.input_mode = InputMode::Editing;
            println!("button clicked in toolbar: {button:?} {position:?}");
            return Ok(());
        }

        // 入力をやめる
        self.input_mode = InputMode::Normal;

        Ok(())
    }

    fn handle_key_input(&mut self, handle_url: UrlHandler) -> Result<(), Error> {
        match self.input_mode {
            InputMode::Normal => {
                let _ = Api::read_key();
            }
            InputMode::Editing => {
                if let Some(c) = Api::read_key() {
                    match c as u8 {
                        0x0a => {
                            // Enterキーが押されたので、ナビゲーションを開始する
                            self.start_navigation(handle_url, self.input_url.clone())?;
                            self.input_url = String::new();
                            self.input_mode = InputMode::Normal;
                        }
                        0x7f | 0x08 => {
                            // Delete キーまたは BackSpace キーが押されたので、最後の文字を削除する
                            self.input_url.pop();
                            self.update_address_bar()?;
                        }
                        _ => {
                            self.input_url.push(c);
                            self.update_address_bar()?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn update_address_bar(&mut self) -> Result<(), Error> {
        self.window
            .fill_rect(WHITE, 72, 4, WINDOW_WIDTH - 76, ADDRESSBAR_HEIGHT - 2)
            .map_err(|_| Error::InvalidUI("failed to clear an address bar".to_string()))?;
        self.window
            .draw_string(BLACK, 74, 6, &self.input_url, StringSize::Medium, false)
            .map_err(|_| Error::InvalidUI("failed to update an address bar".to_string()))?;

        self.window.flush_area(
            Rect::new(
                WINDOW_INIT_X_POS,
                WINDOW_INIT_Y_POS + TITLE_BAR_HEIGHT,
                WINDOW_WIDTH,
                TOOLBAR_HEIGHT,
            )
            .expect("failed to create a rect for the address bar"),
        );

        Ok(())
    }

    fn clear_address_bar(&mut self) -> Result<(), Error> {
        self.window
            .fill_rect(WHITE, 72, 4, WINDOW_WIDTH - 76, ADDRESSBAR_HEIGHT - 2)
            .map_err(|_| Error::InvalidUI("failed to clear an address bar".to_string()))?;

        self.window.flush_area(
            Rect::new(
                WINDOW_INIT_X_POS,
                WINDOW_INIT_Y_POS,
                WINDOW_WIDTH,
                TOOLBAR_HEIGHT,
            )
            .expect("failed to create a rect for the address bar"),
        );

        Ok(())
    }

    fn start_navigation(
        &mut self,
        handle_url: UrlHandler,
        destination: String,
    ) -> Result<(), Error> {
        self.clear_content_area()?;

        let response = handle_url(destination)?;
        let page = self.browser.borrow().current_page();
        page.borrow_mut().recieve_response(response);

        Ok(())
    }

    fn clear_content_area(&mut self) -> Result<(), Error> {
        self.window
            .fill_rect(
                WHITE,
                0,
                TOOLBAR_HEIGHT + 2,
                CONTENT_AREA_WIDTH,
                CONTENT_AREA_HRIGHT - 2,
            )
            .map_err(|_| Error::InvalidUI("failed to clear a content area".to_string()))?;

        self.window.flush();

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Editing,
}
