use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct Plotter(ObjectSubclass<imp::PlotterImp>) @extends gtk::Widget;
}

glib::wrapper! {
    pub struct PlotterInner(ObjectSubclass<imp::PlotterInnerImp>) @extends gtk::Widget;
}
