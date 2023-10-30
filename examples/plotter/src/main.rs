use gtk::prelude::*;

mod plotter;
mod window;

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();
    let application = gtk::Application::new(Some("io.github.airap.plotter"), Default::default());

    application.connect_activate(|app| {
        let win = window::Window::new(app);
        win.show();
    });

    application.run();
}
