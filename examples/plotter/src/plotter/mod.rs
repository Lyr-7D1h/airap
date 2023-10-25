use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct Plotter(ObjectSubclass<imp::Plotter>) @extends gtk::Widget;
}
