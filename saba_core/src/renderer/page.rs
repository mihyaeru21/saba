use core::cell::RefCell;

use alloc::{
    rc::{Rc, Weak},
    string::{String, ToString},
};

use crate::{
    browser::Browser,
    http::HttpResponse,
    renderer::{
        dom::node::Window,
        html::{parser::HtmlParser, token::HtmlTokenizer},
    },
    utils::convert_dom_to_string,
};

#[derive(Debug, Clone)]
pub struct Page {
    browser: Weak<RefCell<Browser>>,
    frame: Option<Rc<RefCell<Window>>>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            browser: Weak::new(),
            frame: None,
        }
    }
}

impl Page {
    pub fn set_browser(&mut self, browser: Weak<RefCell<Browser>>) {
        self.browser = browser;
    }

    pub fn recieve_response(&mut self, response: HttpResponse) -> String {
        self.create_frame(response.body());

        // debug print
        if let Some(frame) = &self.frame {
            let dom = frame.borrow().document().clone();
            let debug = convert_dom_to_string(&Some(dom));
            return debug;
        }

        "".to_string()
    }

    fn create_frame(&mut self, html: String) {
        let html_tokenizer = HtmlTokenizer::new(html);
        let frame = HtmlParser::new(html_tokenizer).construct_tree();
        self.frame = Some(frame);
    }
}
