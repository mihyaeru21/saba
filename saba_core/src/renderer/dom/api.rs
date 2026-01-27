use core::cell::RefCell;

use alloc::{rc::Rc, string::ToString, vec::Vec};

use crate::renderer::dom::node::{Element, ElementKind, Node, NodeKind};

pub fn get_target_element_node(
    node: Option<Rc<RefCell<Node>>>,
    element_kind: ElementKind,
) -> Option<Rc<RefCell<Node>>> {
    let Some(node) = node else { return None };

    if node.borrow().kind()
        == NodeKind::Element(Element::new(&element_kind.to_string(), Vec::new()))
    {
        return Some(node.clone());
    }
    let result1 = get_target_element_node(node.borrow().first_child(), element_kind);
    let result2 = get_target_element_node(node.borrow().next_sibling(), element_kind);

    if result1.is_none() && result2.is_none() {
        return None;
    }
    if result1.is_none() {
        return result2;
    }
    result1
}

pub fn get_style_content(root: Rc<RefCell<Node>>) -> String {
    let Some(style_node) = get_target_element_node(Some(root), ElementKind::Style) else {
        return "".to_string();
    };
    let Some(text_node) = style_node.borrow().first_child() else {
        return "".to_string();
    };
    let Some(content) = &text_node.borrow().kind() else {
        return "".to_string();
    };
    content
}
