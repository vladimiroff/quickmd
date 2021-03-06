//! The GTK user interface.

use anyhow::anyhow;
use gdk::enums::key;
use gtk::prelude::*;
use gtk::{Window, WindowType, HeaderBar};
use log::{debug, warn};
use webkit2gtk::{WebContext, WebView, WebViewExt};

use crate::assets::Assets;

/// Events that trigger UI changes.
///
#[derive(Debug)]
pub enum Event {
    /// Load the given HTML string into the webview.
    LoadHtml(String),
    /// Refresh the webview.
    Reload,
}

/// The container for all the GTK widgets of the app -- window, header bar, etc.
/// Reference-counted, so should be cheap to clone.
///
#[derive(Clone)]
pub struct App {
    window: Window,
    header_bar: HeaderBar,
    webview: WebView,
    assets: Assets,
}

impl App {
    /// Construct a new app.
    ///
    /// The optional `title` parameter is a string shown in the header bar. Initialization could
    /// fail due to `WebContext` or `Assets` failures.
    ///
    pub fn init(title: Option<&str>) -> anyhow::Result<Self> {
        let window = Window::new(WindowType::Toplevel);
        window.set_default_size(1024, 768);

        let header_bar = HeaderBar::new();
        header_bar.set_title(Some("Quickmd"));
        header_bar.set_show_close_button(true);
        header_bar.set_title(title);

        let web_context = WebContext::get_default().
            ok_or_else(|| anyhow!("Couldn't initialize GTK WebContext"))?;
        let webview = WebView::new_with_context(&web_context);

        window.set_titlebar(Some(&header_bar));
        window.add(&webview);

        let assets = Assets::init()?;

        Ok(App { window, header_bar, webview, assets })
    }

    /// Start listening to events from the `ui_receiver` and trigger the relevant methods on the
    /// `App`. Doesn't block.
    ///
    pub fn init_render_loop(&self, ui_receiver: glib::Receiver<Event>) {
        let mut app_clone = self.clone();

        ui_receiver.attach(None, move |event| {
            match event {
                Event::LoadHtml(html) => {
                    app_clone.load_html(&html).
                        unwrap_or_else(|e| warn!("Couldn't update HTML: {}", e))
                },
                Event::Reload => app_clone.reload(),
            }
            glib::Continue(true)
        });
    }

    /// Actually start the UI, blocking the main thread.
    ///
    pub fn run(&self) {
        self.connect_events();
        self.window.show_all();
        gtk::main();
    }

    fn load_html(&mut self, html: &str) -> anyhow::Result<()> {
        let scroll_top = self.webview.get_title().
            and_then(|t| t.parse::<f64>().ok()).
            unwrap_or(0.0);

        let output_path = self.assets.build(html, scroll_top)?;

        debug!("Loading HTML:");
        debug!(" > output_path = {}", output_path.display());

        self.webview.load_uri(&format!("file://{}", output_path.display()));
        Ok(())
    }

    fn reload(&self) {
        self.webview.reload();
    }

    fn connect_events(&self) {
        use std::cell::RefCell;
        let self_clone = RefCell::new(Some(self.clone()));

        // Each key press will invoke this function.
        self.window.connect_key_press_event(move |_window, gdk| {
            if let key::Escape = gdk.get_keyval() {
                self_clone.borrow_mut().take().unwrap().assets.delete();
                gtk::main_quit()
            }
            Inhibit(false)
        });

        self.window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });
    }
}
