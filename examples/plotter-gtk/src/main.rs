use gtk::{glib::SignalHandlerId, prelude::*};
use plotters::{
    prelude::{ChartBuilder, IntoDrawingArea},
    style::{Color, BLACK},
};
use plotters_cairo::CairoBackend;

mod plotter;
mod window;

const UI: &str = include_str!("window/ui.xml");
fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();
    let application = gtk::Application::new(Some("io.github.airap.plotter"), Default::default());

    application.connect_activate(|app| {
        let win = window::Window::new(app);

        win.show();
    });

    application.run();
}
