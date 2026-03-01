use core::cell::RefCell;

use alloc::{
    rc::{Rc, Weak},
    string::String,
    vec::Vec,
};

use crate::{
    browser::Browser,
    display_item::DisplayItem,
    http::HttpResponse,
    renderer::{
        css::{
            cssom::{CssParser, StyleSheet},
            token::CssTokenizer,
        },
        dom::{
            api::get_style_content,
            node::{ElementKind, NodeKind, Window},
        },
        html::{parser::HtmlParser, token::HtmlTokenizer},
        layout::{layout_object::LayoutPoint, layout_view::LayoutView},
    },
};

#[derive(Debug, Clone)]
pub struct Page {
    browser: Weak<RefCell<Browser>>,
    frame: Option<Rc<RefCell<Window>>>,
    style: Option<StyleSheet>,
    layout_view: Option<LayoutView>,
    display_items: Vec<DisplayItem>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            browser: Weak::new(),
            frame: None,
            style: None,
            layout_view: None,
            display_items: Vec::new(),
        }
    }
}

impl Page {
    pub fn set_browser(&mut self, browser: Weak<RefCell<Browser>>) {
        self.browser = browser;
    }

    pub fn recieve_response(&mut self, response: HttpResponse) {
        self.create_frame(response.body());
        self.set_layout_view();
        self.paint_tree();
    }

    fn create_frame(&mut self, html: String) {
        let html_tokenizer = HtmlTokenizer::new(html);
        let frame = HtmlParser::new(html_tokenizer).construct_tree();
        let dom = frame.borrow().document();

        let style = get_style_content(dom);
        let css_tokenizer = CssTokenizer::new(style);
        let cssom = CssParser::new(css_tokenizer).parse_stylesheet();

        self.frame = Some(frame);
        self.style = Some(cssom);
    }

    fn set_layout_view(&mut self) {
        let Some(frame) = &self.frame else { return };
        let Some(style) = self.style.clone() else {
            return;
        };

        let layout_view = LayoutView::new(frame.borrow().document(), &style);
        self.layout_view = Some(layout_view);
    }

    fn paint_tree(&mut self) {
        let Some(layout_view) = &self.layout_view else {
            return;
        };

        self.display_items = layout_view.paint();
    }

    pub fn display_items(&self) -> Vec<DisplayItem> {
        self.display_items.clone()
    }

    pub fn clear_display_items(&mut self) {
        self.display_items = Vec::new();
    }

    pub fn clicked(&self, position: LayoutPoint) -> Option<String> {
        let view = self.layout_view.as_ref()?;
        let node = view.find_node_by_position(position)?;
        let parent = node.borrow().parent().upgrade()?;

        if let NodeKind::Element(e) = parent.borrow().node_kind()
            && e.kind() == ElementKind::A
        {
            return e.get_attribute("href");
        }

        None
    }
}
