use core::cell::RefCell;

use alloc::rc::Rc;

use crate::renderer::{
    css::cssom::StyleSheet,
    dom::{
        api::get_target_element_node,
        node::{ElementKind, Node},
    },
    layout::layout_object::LayoutObject,
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
}
