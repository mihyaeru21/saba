use alloc::string::String;

use crate::renderer::layout::{
    computed_style::ComputedStyle,
    layout_object::{LayoutPoint, LayoutRect},
};

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayItem {
    Rect {
        style: ComputedStyle,
        layout_rect: LayoutRect,
    },
    Text {
        text: String,
        style: ComputedStyle,
        layout_point: LayoutPoint,
    },
}
