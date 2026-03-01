use core::cell::RefCell;

use alloc::string::String;
use alloc::vec;
use alloc::{
    rc::{Rc, Weak},
    string::ToString,
    vec::Vec,
};

use crate::constants::{WINDOW_PADDING, WINDOW_WIDTH};
use crate::{
    constants::{CHAR_HEIGHT_WITH_PADDING, CHAR_WIDTH, CONTENT_AREA_WIDTH},
    display_item::DisplayItem,
    renderer::{
        css::cssom::{ComponentValue, Declaration, Selector},
        dom::node::{Node, NodeKind},
        layout::computed_style::{Color, ComputedStyle, DisplayType, FontSize},
    },
};

#[derive(Debug, Clone)]
pub struct LayoutObject {
    kind: LayoutObjectKind,
    node: Rc<RefCell<Node>>,
    first_child: Option<Rc<RefCell<LayoutObject>>>,
    next_sibling: Option<Rc<RefCell<LayoutObject>>>,
    parent: Weak<RefCell<LayoutObject>>,
    style: ComputedStyle,
    rect: LayoutRect,
}

impl LayoutObject {
    pub fn new(node: Rc<RefCell<Node>>, parent_obj: &Option<Rc<RefCell<LayoutObject>>>) -> Self {
        let parent = match parent_obj {
            Some(p) => Rc::downgrade(p),
            None => Weak::new(),
        };

        Self {
            kind: LayoutObjectKind::Block,
            node: node.clone(),
            first_child: None,
            next_sibling: None,
            parent,
            style: ComputedStyle::default(),
            rect: LayoutRect {
                point: LayoutPoint { x: 0, y: 0 },
                size: LayoutSize {
                    width: 0,
                    height: 0,
                },
            },
        }
    }

    pub fn kind(&self) -> LayoutObjectKind {
        self.kind
    }

    pub fn node_kind(&self) -> NodeKind {
        self.node.borrow().kind().clone()
    }

    pub fn set_first_child(&mut self, first_child: Option<Rc<RefCell<LayoutObject>>>) {
        self.first_child = first_child;
    }

    pub fn first_child(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.first_child.as_ref().cloned()
    }

    pub fn set_next_sibling(&mut self, next_sibling: Option<Rc<RefCell<LayoutObject>>>) {
        self.next_sibling = next_sibling;
    }

    pub fn next_sibling(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.next_sibling.as_ref().cloned()
    }

    pub fn parent(&self) -> Weak<RefCell<Self>> {
        self.parent.clone()
    }

    pub fn style(&self) -> ComputedStyle {
        self.style.clone()
    }

    pub fn rect(&self) -> LayoutRect {
        self.rect
    }

    pub fn point(&self) -> LayoutPoint {
        self.rect.point
    }

    pub fn size(&self) -> LayoutSize {
        self.rect.size
    }

    pub fn is_node_selected(&self, selector: &Selector) -> bool {
        let NodeKind::Element(element) = &self.node_kind() else {
            return false;
        };

        match selector {
            Selector::TypeSelector(type_name) => element.kind().to_string() == *type_name,
            Selector::ClassSelector(class_name) => element
                .attributes()
                .iter()
                .any(|a| a.name() == "class" && a.value() == *class_name),
            Selector::IdSelector(id_name) => element
                .attributes()
                .iter()
                .any(|a| a.name() == "id" && a.value() == *id_name),
            Selector::UnknownSelector => false,
        }
    }

    pub fn cascading_style(&mut self, declarations: Vec<Declaration>) {
        for declaration in declarations {
            match declaration.property.as_ref() {
                "background-color" => {
                    if let ComponentValue::Ident(value) = &declaration.value {
                        let color = match Color::from_name(value) {
                            Ok(color) => color,
                            Err(_) => Color::white(),
                        };
                        self.style.set_background_color(color);
                        continue;
                    }

                    if let ComponentValue::HashToken(color_code) = &declaration.value {
                        let color = match Color::from_code(color_code) {
                            Ok(color) => color,
                            Err(_) => Color::white(),
                        };
                        self.style.set_background_color(color);
                        continue;
                    }
                }
                "color" => {
                    if let ComponentValue::Ident(value) = &declaration.value {
                        let color = match Color::from_name(value) {
                            Ok(color) => color,
                            Err(_) => Color::black(),
                        };
                        self.style.set_color(color);
                    }

                    if let ComponentValue::HashToken(color_code) = &declaration.value {
                        let color = match Color::from_code(color_code) {
                            Ok(color) => color,
                            Err(_) => Color::black(),
                        };
                        self.style.set_color(color);
                    }
                }
                "display" => {
                    if let ComponentValue::Ident(value) = declaration.value {
                        let display_type = match DisplayType::from_str(&value) {
                            Ok(display_type) => display_type,
                            Err(_) => DisplayType::DisplayNone,
                        };
                        self.style.set_display(display_type);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn defaulting_style(
        &mut self,
        node: &Rc<RefCell<Node>>,
        parent_style: Option<ComputedStyle>,
    ) {
        self.style.defaulting(node, parent_style);
    }

    pub fn update_kind(&mut self) {
        self.kind = match self.node_kind() {
            NodeKind::Document => {
                panic!("should not create Blocka layout object for a Document node")
            }
            NodeKind::Element(_) => match self.style.display() {
                DisplayType::Block => LayoutObjectKind::Block,
                DisplayType::Inline => LayoutObjectKind::Inline,
                DisplayType::DisplayNone => {
                    panic!("hould not create a layout object for display:none")
                }
            },
            NodeKind::Text(_) => LayoutObjectKind::Text,
        };
    }

    pub fn compute_size(&mut self, parent_size: LayoutSize) {
        let mut size = LayoutSize {
            width: 0,
            height: 0,
        };

        match self.kind() {
            LayoutObjectKind::Block => {
                size.width = parent_size.width;

                // すべての子ノードの高さを足し合わせた結果が高さになる。
                // ただし、インライン要素が横に並んでいる場合は注意が必要
                let mut height = 0;
                let mut child = self.first_child();
                let mut prev_child_kind = LayoutObjectKind::Block;
                while let Some(c) = child {
                    if prev_child_kind == LayoutObjectKind::Block
                        || c.borrow().kind() == LayoutObjectKind::Block
                    {
                        height += c.borrow().size().height;
                    }

                    prev_child_kind = c.borrow().kind();
                    child = c.borrow().next_sibling();
                }
                size.height = height;
            }
            LayoutObjectKind::Inline => {
                // すべての子ノードの高さと横幅を足し合わせた結果が現在のノードの高さと横幅とになる
                let mut width = 0;
                let mut height = 0;
                let mut child = self.first_child();
                while let Some(c) = child {
                    let c = c.borrow();
                    width += c.size().width;
                    height += c.size().height;
                    child = c.next_sibling();
                }
                size.width = width;
                size.height = height;
            }
            LayoutObjectKind::Text => {
                if let NodeKind::Text(t) = self.node_kind() {
                    let ratio = match self.style.font_size() {
                        FontSize::Medium => 1,
                        FontSize::XLarge => 2,
                        FontSize::XXLarge => 3,
                    };
                    let width = CHAR_WIDTH * ratio * t.len() as i64;
                    if width > CONTENT_AREA_WIDTH {
                        size.width = CONTENT_AREA_WIDTH;
                        let line_num = if width.wrapping_rem(CONTENT_AREA_WIDTH) == 0 {
                            width.wrapping_div(CONTENT_AREA_WIDTH)
                        } else {
                            width.wrapping_div(CONTENT_AREA_WIDTH) + 1
                        };
                        size.height = CHAR_HEIGHT_WITH_PADDING * ratio * line_num;
                    } else {
                        size.width = width;
                        size.height = CHAR_HEIGHT_WITH_PADDING * ratio;
                    }
                }
            }
        }

        self.rect.size = size;
    }

    pub fn compute_position(
        &mut self,
        parent_point: LayoutPoint,
        prev_sibling_kind: LayoutObjectKind,
        prev_sibling_rect: Option<LayoutRect>,
    ) {
        let mut point = LayoutPoint { x: 0, y: 0 };

        match (self.kind(), prev_sibling_kind) {
            (LayoutObjectKind::Block, _) | (_, LayoutObjectKind::Block) => {
                if let Some(LayoutRect { point: pos, size }) = prev_sibling_rect {
                    point.y = pos.y + size.height;
                } else {
                    point.y = parent_point.y;
                }
                point.x = parent_point.x;
            }
            (LayoutObjectKind::Inline, LayoutObjectKind::Inline) => {
                if let Some(LayoutRect { point: pos, size }) = prev_sibling_rect {
                    point.x = pos.x + size.width;
                    point.y = pos.y;
                } else {
                    point.x = parent_point.x;
                    point.y = parent_point.y;
                }
            }
            _ => {
                point.x = parent_point.x;
                point.y = parent_point.y;
            }
        }

        self.rect.point = point;
    }

    pub fn paint(&mut self) -> Vec<DisplayItem> {
        if self.style.display() == DisplayType::DisplayNone {
            return Vec::new();
        }

        match self.kind {
            LayoutObjectKind::Block => {
                if let NodeKind::Element(_) = self.node_kind() {
                    return vec![DisplayItem::Rect {
                        style: self.style(),
                        layout_rect: self.rect,
                    }];
                }
            }
            LayoutObjectKind::Inline => {
                // 本書のブラウザでは、描画するインライン要素はない。
                // <img> タグなどをサポートした場合はこのアームの中で処理をする
            }
            LayoutObjectKind::Text => {
                if let NodeKind::Text(t) = self.node_kind() {
                    let mut v = Vec::new();

                    let ratio = match self.style.font_size() {
                        FontSize::Medium => 1,
                        FontSize::XLarge => 2,
                        FontSize::XXLarge => 3,
                    };
                    let plain_text = t
                        .replace("\n", " ")
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let lines = split_text(plain_text, CHAR_WIDTH * ratio);
                    for (i, line) in lines.into_iter().enumerate() {
                        let item = DisplayItem::Text {
                            text: line,
                            style: self.style(),
                            layout_point: LayoutPoint {
                                x: self.rect.point.x,
                                y: self.rect.point.y + CHAR_HEIGHT_WITH_PADDING * i as i64,
                            },
                        };
                        v.push(item);
                    }

                    return v;
                }
            }
        }

        Vec::new()
    }
}

impl PartialEq for LayoutObject {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LayoutObjectKind {
    Block,
    Inline,
    Text,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LayoutPoint {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LayoutSize {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LayoutRect {
    pub point: LayoutPoint,
    pub size: LayoutSize,
}

impl LayoutRect {
    pub fn is_hit(&self, point: LayoutPoint) -> bool {
        let is_hit_x = self.point.x <= point.x && point.x <= (self.point.x + self.size.width);
        let is_hit_y = self.point.y <= point.y && point.y <= (self.point.y + self.size.height);
        is_hit_x && is_hit_y
    }
}

fn find_index_for_line_break(line: &str, max_index: usize) -> usize {
    for i in (0..max_index).rev() {
        if line.chars().collect::<Vec<char>>()[i] == ' ' {
            return i;
        }
    }
    max_index
}

fn split_text(line: String, char_width: i64) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let width = WINDOW_WIDTH + WINDOW_PADDING;
    if line.len() as i64 * char_width > width {
        let s = line.split_at(find_index_for_line_break(
            &line,
            (width / char_width) as usize,
        ));
        result.push(s.0.to_string());
        result.extend(split_text(s.1.trim().to_string(), char_width));
    } else {
        result.push(line);
    }

    result
}
