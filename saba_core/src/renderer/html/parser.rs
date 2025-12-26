use alloc::{rc::Rc, string::String, vec::Vec};
use core::{cell::RefCell, str::FromStr};

use crate::renderer::{
    dom::node::{Element, ElementKind, Node, NodeKind, Window},
    html::{
        attribute::Attribute,
        token::{HtmlToken, HtmlTokenizer},
    },
};

#[derive(Debug, Clone)]
pub struct HtmlParser {
    window: Rc<RefCell<Window>>,
    mode: InsertionMode,
    original_mode: InsertionMode,
    stack_of_open_elements: Vec<Rc<RefCell<Node>>>,
    t: HtmlTokenizer,
}

impl HtmlParser {
    pub fn new(t: HtmlTokenizer) -> Self {
        Self {
            window: Rc::new(RefCell::new(Window::new())),
            mode: InsertionMode::Initial,
            original_mode: InsertionMode::Initial,
            stack_of_open_elements: Vec::new(),
            t,
        }
    }

    pub fn construct_tree(&mut self) -> Rc<RefCell<Window>> {
        let mut token = self.t.next();
        while let Some(ref t) = token {
            match self.mode {
                InsertionMode::Initial => {
                    // 本書では、DOCTYPEトークンをサポートしていないため、
                    // <!doctype html> のようなトークンは文字トークンとして表される。
                    // 文字トークンは無視する
                    if let HtmlToken::Char(_) = t {
                        token = self.t.next();
                        continue;
                    }

                    self.mode = InsertionMode::BeforeHtml;
                    continue;
                }
                InsertionMode::BeforeHtml => {
                    match *t {
                        HtmlToken::Char(c) => {
                            if c == ' ' || c == '\n' {
                                token = self.t.next();
                                continue;
                            }
                        }
                        HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        } => {
                            if tag == "html" {
                                self.insert_element(tag, attributes.to_vec());
                                self.mode = InsertionMode::BeforeHead;
                                token = self.t.next();
                                continue;
                            }
                        }
                        HtmlToken::Eof => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    // <html> を省略した場合向け
                    self.insert_element("html", Vec::new());
                    self.mode = InsertionMode::BeforeHead;
                    continue;
                }
                InsertionMode::BeforeHead => {
                    match *t {
                        HtmlToken::Char(c) => {
                            if c == ' ' || c == '\n' {
                                token = self.t.next();
                                continue;
                            }
                        }
                        HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        } => {
                            if tag == "head" {
                                self.insert_element(tag, attributes.to_vec());
                                self.mode = InsertionMode::InHead;
                                token = self.t.next();
                                continue;
                            }
                        }
                        HtmlToken::Eof => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    // <head> を省略した場合向け
                    self.insert_element("head", Vec::new());
                    self.mode = InsertionMode::InHead;
                    continue;
                }
                InsertionMode::InHead => {
                    match *t {
                        HtmlToken::Char(c) => {
                            if c == ' ' || c == '\n' {
                                self.insert_char(c);
                                token = self.t.next();
                                continue;
                            }
                        }
                        HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        } => {
                            if tag == "style" || tag == "script" {
                                self.insert_element(tag, attributes.to_vec());
                                self.original_mode = self.mode;
                                self.mode = InsertionMode::Text;
                                token = self.t.next();
                                continue;
                            }
                            // 仕様書には定められていないが、このブラウザは仕様をすべて実装しているわけではないので、
                            // <head>が省略されているHTML文書を扱うために必要。
                            // これがないと <head> が省略されている HTML 文書で無限ループが発生
                            if tag == "body" {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }
                            if let Ok(_element_kind) = ElementKind::from_str(tag) {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }
                        }
                        HtmlToken::EndTag { ref tag } => {
                            if tag == "head" {
                                self.mode = InsertionMode::AfterHead;
                                token = self.t.next();
                                self.pop_until(ElementKind::Head);
                                continue;
                            }
                        }
                        HtmlToken::Eof => {
                            return self.window.clone();
                        }
                    }

                    // <meta> や <title> などのサポートしていないタグは無視する
                    token = self.t.next();
                    continue;
                }
                InsertionMode::AfterHead => {
                    match *t {
                        HtmlToken::Char(c) => {
                            if c == ' ' || c == '\n' {
                                self.insert_char(c);
                                token = self.t.next();
                                continue;
                            }
                        }
                        HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        } => {
                            if tag == "body" {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                self.mode = InsertionMode::InBody;
                                continue;
                            }
                        }
                        HtmlToken::Eof => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    self.insert_element("body", Vec::new());
                    self.mode = InsertionMode::InBody;
                    continue;
                }
                InsertionMode::InBody => {
                    match *t {
                        HtmlToken::StartTag {
                            ref tag,
                            self_closing: _,
                            ref attributes,
                        } => match tag.as_str() {
                            "p" | "h1" | "h2" | "a" | "span" => {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                continue;
                            }
                            _ => token = self.t.next(),
                        },
                        HtmlToken::EndTag { ref tag } => {
                            match tag.as_str() {
                                "body" => {
                                    self.mode = InsertionMode::AfterBody;
                                    token = self.t.next();
                                    if !self.contain_in_stack(ElementKind::Body) {
                                        // パースの失敗。トークンを無視する
                                        continue;
                                    }
                                    self.pop_until(ElementKind::Body);
                                    continue;
                                }
                                "html" => {
                                    if self.pop_current_node(ElementKind::Body) {
                                        self.mode = InsertionMode::AfterBody;
                                        assert!(self.pop_current_node(ElementKind::Html));
                                    } else {
                                        token = self.t.next();
                                    }
                                    continue;
                                }
                                "p" | "h1" | "h2" | "a" | "span" => {
                                    let element_kind = ElementKind::from_str(tag)
                                        .expect("failed to convert string to ElementKind");
                                    token = self.t.next();
                                    self.pop_until(element_kind);
                                    continue;
                                }
                                _ => {
                                    token = self.t.next();
                                }
                            }
                        }
                        HtmlToken::Eof => {
                            return self.window.clone();
                        }
                        HtmlToken::Char(c) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }
                    }
                }
                InsertionMode::Text => {
                    match *t {
                        HtmlToken::Eof => {
                            return self.window.clone();
                        }
                        HtmlToken::EndTag { ref tag } => {
                            if tag == "style" {
                                self.pop_until(ElementKind::Style);
                                self.mode = self.original_mode;
                                token = self.t.next();
                                continue;
                            }
                            if tag == "script" {
                                self.pop_until(ElementKind::Script);
                                self.mode = self.original_mode;
                                token = self.t.next();
                                continue;
                            }
                        }
                        HtmlToken::Char(c) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }
                        _ => {}
                    }

                    self.mode = self.original_mode;
                }
                InsertionMode::AfterBody => {
                    match *t {
                        HtmlToken::Char(_c) => {
                            token = self.t.next();
                            continue;
                        }
                        HtmlToken::EndTag { ref tag } => {
                            if tag == "html" {
                                self.mode = InsertionMode::AfterAfterBody;
                                token = self.t.next();
                                continue;
                            }
                        }
                        HtmlToken::Eof => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    self.mode = InsertionMode::InBody;
                }
                InsertionMode::AfterAfterBody => {
                    match t {
                        HtmlToken::Char(_c) => {
                            token = self.t.next();
                            continue;
                        }
                        HtmlToken::Eof => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    // パースの失敗
                    self.mode = InsertionMode::InBody;
                }
            }
        }

        self.window.clone()
    }

    fn create_element(&self, tag: &str, attributes: Vec<Attribute>) -> Node {
        Node::new(NodeKind::Element(Element::new(tag, attributes)))
    }

    fn insert_element(&mut self, tag: &str, attributes: Vec<Attribute>) {
        let window = self.window.borrow();
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n.clone(),
            None => window.document(),
        };

        let node = Rc::new(RefCell::new(self.create_element(tag, attributes)));

        if let Some(mut last_sibling) = current.borrow().first_child() {
            loop {
                let Some(next_sibling) = node.borrow().next_sibling() else {
                    break;
                };
                last_sibling = next_sibling;
            }

            last_sibling
                .borrow_mut()
                .set_next_sibling(Some(node.clone()));
            node.borrow_mut()
                .set_previous_sibling(Rc::downgrade(&last_sibling));
        } else {
            current.borrow_mut().set_first_child(Some(node.clone()));
        }

        current.borrow_mut().set_last_child(Rc::downgrade(&node));
        node.borrow_mut().set_parent(Rc::downgrade(&current));

        self.stack_of_open_elements.push(node);
    }

    fn pop_current_node(&mut self, element_kind: ElementKind) -> bool {
        let Some(current) = self.stack_of_open_elements.last() else {
            return false;
        };

        if current.borrow().element_kind() == Some(element_kind) {
            self.stack_of_open_elements.pop();
            return true;
        }

        false
    }

    fn pop_until(&mut self, element_kind: ElementKind) {
        assert!(
            self.contain_in_stack(element_kind),
            "stack doesn't have an element {:?}",
            element_kind
        );

        loop {
            let Some(current) = self.stack_of_open_elements.pop() else {
                return;
            };

            if current.borrow().element_kind() == Some(element_kind) {
                return;
            }
        }
    }

    fn contain_in_stack(&mut self, element_kind: ElementKind) -> bool {
        for i in 0..self.stack_of_open_elements.len() {
            if self.stack_of_open_elements[i].borrow().element_kind() == Some(element_kind) {
                return true;
            }
        }

        false
    }

    fn create_char(&self, c: char) -> Node {
        let mut s = String::new();
        s.push(c);
        Node::new(NodeKind::Text(s))
    }

    fn insert_char(&mut self, c: char) {
        let Some(current) = self.stack_of_open_elements.last() else {
            return;
        };

        if let NodeKind::Text(ref mut s) = current.borrow_mut().kind {
            s.push(c);
            return;
        }

        if c == '\n' || c == ' ' {
            return;
        }

        let node = Rc::new(RefCell::new(self.create_char(c)));

        if let Some(first_child) = current.borrow().first_child() {
            first_child
                .borrow_mut()
                .set_next_sibling(Some(node.clone()));
            // TODO: 正誤表で消されてるけど、構造としては previous_sibling に設定すべきに見える？
            // node.borrow_mut()
            //     .set_previous_sibling(Rc::downgrade(&first_child));
        } else {
            current.borrow_mut().set_first_child(Some(node.clone()));
        }

        current.borrow_mut().set_last_child(Rc::downgrade(&node));
        node.borrow_mut().set_parent(Rc::downgrade(current));

        self.stack_of_open_elements.push(node);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    Text,
    AfterBody,
    AfterAfterBody,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::string::ToString;

    #[test]
    fn test_empty() {
        let html = "".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();

        assert_eq!(doc_node(), document);
        assert_eq!(None, document.borrow().first_child())
    }

    #[test]
    fn test_body_text() {
        let html = "<html><head></head><body>text value</body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();
        assert_eq!(doc_node(), document);

        let html = document.borrow().first_child().unwrap();
        assert_eq!(elem_node("html", &[]), html);

        let head = html.borrow().first_child().unwrap();
        assert_eq!(elem_node("head", &[]), head);

        let body = head.borrow().next_sibling().unwrap();
        assert_eq!(elem_node("body", &[]), body);

        let text = body.borrow().first_child().unwrap();
        assert_eq!(text_node("text value"), text);
    }

    #[test]
    fn test_multiple_nodes() {
        let html = r#"<html><head></head><body><p><a foo=bar>text value</a><span class="hoge">xxx</span></p></body></html>"#.to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();

        let body = document
            .borrow()
            .first_child()
            .unwrap()
            .borrow()
            .first_child()
            .unwrap()
            .borrow()
            .next_sibling()
            .unwrap();
        assert_eq!(elem_node("body", &[]), body);

        let p = body.borrow().first_child().unwrap();
        assert_eq!(elem_node("p", &[]), p);

        let a_attr = Attribute::nv("foo", "bar");
        let a = p.borrow().first_child().unwrap();
        assert_eq!(elem_node("a", &[a_attr]), a);

        let text = a.borrow().first_child().unwrap();
        assert_eq!(text_node("text value"), text);

        let span_attr = Attribute::nv("class", "hoge");
        let span = a.borrow().next_sibling().unwrap();
        assert_eq!(elem_node("span", &[span_attr]), span);
    }

    fn doc_node() -> Rc<RefCell<Node>> {
        Rc::new(RefCell::new(Node::new(NodeKind::Document)))
    }

    fn elem_node(name: &str, attributes: &[Attribute]) -> Rc<RefCell<Node>> {
        let kind = NodeKind::Element(Element::new(name, attributes.to_vec()));
        Rc::new(RefCell::new(Node::new(kind)))
    }

    fn text_node(text: &str) -> Rc<RefCell<Node>> {
        let kind = NodeKind::Text(text.to_string());
        Rc::new(RefCell::new(Node::new(kind)))
    }
}
