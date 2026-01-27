use core::cell::RefCell;

use alloc::rc::Rc;

use crate::renderer::{
    css::cssom::StyleSheet,
    dom::{
        api::get_target_element_node,
        node::{ElementKind, Node},
    },
    layout::{
        computed_style::DisplayType,
        layout_object::{LayoutObject, LayoutObjectKind, LayoutPoint, LayoutSize},
    },
};

#[derive(Debug, Clone)]
pub struct LayoutView {
    root: Option<Rc<RefCell<LayoutObject>>>,
}

impl LayoutView {
    pub fn new(root: Rc<RefCell<Node>>, cssom: &StyleSheet) -> Self {
        // レイアウトツリーは描画される要素だけを持つツリーなので、<body>タグを取得し、
        // その子要素以下をレイアウトツリーのノードに変換する。
        let body_root = get_target_element_node(Some(root), ElementKind::Body);

        let mut tree = Self {
            root: build_layout_tree(&body_root, &None, cssom),
        };
        tree.update_layout();

        tree
    }

    pub fn root(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.root.clone()
    }

    fn update_layout(&mut self) {
        Self::calculate_node_size(&self.root, LayoutSize::new(CONTENT_AREA_WIDTH, 0));
        Self::calculate_node_position(
            &self.root,
            LayoutPoint::new(0, 0),
            LayoutObjectKind::Block,
            None,
            None,
        );
    }

    fn calculate_node_size(node: &Option<Rc<RefCell<LayoutObject>>>, parent_size: LayoutSize) {
        let Some(node) = node else { return };

        // ノードがブロック要素の場合、子ノードのレイアウトを計算する前に横幅を決める
        if node.borrow().kind() == LayoutObjectKind::Block {
            node.borrow_mut().compute_size(parent_size);
        }

        let first_child = node.borrow().first_child();
        Self::calculate_node_size(&first_child, parent_size);

        let next_sibling = node.borrow().next_sibling();
        Self::calculate_node_size(&next_sibling, parent_size);

        // 子ノードのサイズが決まったあとにサイズを計算する
        // ブロック要素のとき、高さは子ノードの高さに依存する
        // インライン要素のとき、高さも横幅も子ノードに依存する
        node.borrow_mut().compute_size(parent_size);
    }

    fn calculate_node_position(
        node: &Option<Rc<RefCell<LayoutObject>>>,
        parent_point: LayoutPoint,
        prev_sibling_kind: LayoutObjectKind,
        prev_sibling_point: Option<LayoutPoint>,
        prev_sibling_size: Option<LayoutSize>,
    ) {
        let Some(node) = node else { return };

        node.borrow_mut().compute_position(
            parent_point,
            prev_sibling_kind,
            prev_sibling_point,
            prev_sibling_size,
        );

        let first_child = node.borrow().first_child();
        Self::calculate_node_position(
            &first_child,
            node.borrow().point(),
            LayoutObjectKind::Block,
            None,
            None,
        );

        let next_sibling = node.borrow().next_sibling();
        Self::calculate_node_position(
            &next_sibling,
            parent_point,
            node.borrow().kind(),
            Some(node.borrow().point()),
            Some(node.borrow().size()),
        );
    }
}

fn build_layout_tree(
    node: &Option<Rc<RefCell<Node>>>,
    parent_obj: &Option<Rc<RefCell<LayoutObject>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject>>> {
    let mut target_node = node.clone();
    let mut layout_object = create_layout_object(node, parent_obj, cssom);

    while layout_object.is_none() {
        let Some(n) = target_node else {
            return layout_object;
        };
        target_node = n.borrow().next_sibling().clone();
        layout_object = create_layout_object(&target_node, parent_obj, cssom);
    }

    if let Some(n) = target_node {
        let original_first_child = n.borrow().first_child();
        let original_next_sibling = n.borrow().next_sibling();
        let mut first_child = build_layout_tree(&original_first_child, &layout_object, cssom);
        let mut next_sibling = build_layout_tree(&original_next_sibling, &layout_object, cssom);

        if first_child.is_none()
            && let Some(ofc) = original_first_child
        {
            let mut original_dom_node = ofc.borrow().next_sibling();

            loop {
                first_child = build_layout_tree(&original_dom_node, &layout_object, cssom);

                if first_child.is_none()
                    && let Some(odn) = original_dom_node
                {
                    original_dom_node = odn.borrow().next_sibling();
                    continue;
                }

                break;
            }
        }

        if next_sibling.is_none()
            && let Some(ons) = original_next_sibling
        {
            let mut original_dom_node = ons.borrow().next_sibling();

            loop {
                next_sibling = build_layout_tree(&original_dom_node, &None, cssom);

                if next_sibling.is_none()
                    && let Some(odn) = original_dom_node
                {
                    original_dom_node = odn.borrow().next_sibling();
                    continue;
                }

                break;
            }
        }

        let Some(ref obj) = layout_object else {
            panic!("render object should exist here");
        };
        obj.borrow_mut().set_first_child(first_child);
        obj.borrow_mut().set_next_sibling(next_sibling);
    }

    layout_object
}

fn create_layout_object(
    node: &Option<Rc<RefCell<Node>>>,
    parent_obj: &Option<Rc<RefCell<LayoutObject>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject>>> {
    let Some(node) = node else { return None };

    let layout_object = Rc::new(RefCell::new(LayoutObject::new(node.clone(), parent_obj)));

    for rule in &cssom.rules {
        if layout_object.borrow().is_node_selected(&rule.selector) {
            layout_object
                .borrow_mut()
                .cascading_style(rule.declarations.clone());
        }
    }

    let parent_style = parent_obj.map(|p| p.borrow().style());
    layout_object
        .borrow_mut()
        .defaulting_style(node, parent_style);

    if layout_object.borrow().style().display() == DisplayType::DisplayNone {
        return None;
    }

    layout_object.borrow_mut().update_kind();
    Some(layout_object)
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use crate::renderer::{
        css::{cssom::CssParser, token::CssTokenizer},
        html::{parser::HtmlParser, token::HtmlTokenizer},
    };

    use super::*;

    fn create_layout_view(html: String) -> LayoutView {
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let dom = window.borrow().document();
        let css = get_style_content(dom.clone());
        let css_tokenizer = CssTokenizer::new(css);
        let cssom = CssParser::new(css_tokenizer).parse_stylesheet();
        LayoutView::new(dom, &cssom)
    }
}
