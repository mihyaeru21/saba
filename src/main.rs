#![no_std]
#![no_main]

extern crate alloc;

use core::cell::RefCell;

use alloc::{format, rc::Rc, string::ToString};
use net_wasabi::http::HttpClient;
use noli::{entry_point, println};
use saba_core::{browser::Browser, error::Error, http::HttpResponse, url::Url};
use ui_wasabi::app::WasabiUI;

fn main() -> u64 {
    let browser = Browser::new();
    let ui = Rc::new(RefCell::new(WasabiUI::new(browser)));

    if let Err(e) = ui.borrow_mut().start(handle_url) {
        println!("browser failes to start: {e:?}");
        return 1;
    }

    0
}

entry_point!(main);

fn handle_url(url: &str) -> Result<HttpResponse, Error> {
    http_get(url, false)
}

fn http_get(url: &str, redirecting: bool) -> Result<HttpResponse, Error> {
    let parsed_url = Url::new(url.to_string())
        .parse()
        .map_err(|e| Error::UnexpectedInput(format!("input url is not supported: {e:?}")))?;

    let client = HttpClient::default();
    let response = client
        .get(&parsed_url)
        .map_err(|e| Error::Network(format!("failed to get http response: {e:?}")))?;

    // 元の実装ではリダイレクトは1段だけ実装されてるのでそれを再現
    if !redirecting && response.status_code() == 302 {
        let Ok(location) = response.header_value("Location") else {
            return Ok(response);
        };
        return http_get(&location, true);
    }

    Ok(response)
}
