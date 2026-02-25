#![no_std]
#![no_main]

extern crate alloc;

use core::cell::RefCell;

use alloc::rc::Rc;
use noli::prelude::*;
use saba_core::browser::Browser;
use ui_wasabi::app::WasabiUI;

fn main() -> u64 {
    let browser = Browser::new();
    let ui = Rc::new(RefCell::new(WasabiUI::new(browser)));

    if let Err(e) = ui.borrow_mut().start() {
        println!("browser failes to start: {e:?}");
        return 1;
    }

    0
}

entry_point!(main);
