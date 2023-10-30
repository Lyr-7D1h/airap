use airap::Airap;
use gtk::cairo;
use gtk::gio;

use gtk::glib;
use gtk::glib::idle_add_local;
use gtk::glib::timeout_add_local;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use log::info;
use plotters::coord::types::RangedCoordf32;
use plotters::coord::Shift;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use plotters::prelude::*;
use plotters_cairo::CairoBackend;
use std::borrow::BorrowMut;
use std::cell::Cell;
use std::cell::RefCell;
use std::error::Error;
use std::ops::Div;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use super::PlotterInner;

// https://stackoverflow.com/questions/66510406/gtk-rs-how-to-update-view-from-another-thread
// Example Graph: https://github.com/GuillaumeGomez/process-viewer/tree/master

thread_local!(
    static AIRAP: RefCell<Option<mpsc::Receiver<Vec<f32>>>> = RefCell::new(None);
);

#[derive(Debug)]
pub struct PlotterImp {
    inner: PlotterInner,
}
#[glib::object_subclass]
impl ObjectSubclass for PlotterImp {
    const NAME: &'static str = "Plotter";
    type Type = super::Plotter;
    type ParentType = gtk::Widget;
    fn class_init(_klass: &mut Self::Class) {
        // let inner = PlotterInner::new();
        // timeout_add_local(Duration::from_millis(16), move || {
        //     println!("B");
        //     inner.queue_draw();
        //     glib::ControlFlow::Continue
        // });
    }
}
impl Default for PlotterImp {
    fn default() -> Self {
        let inner = PlotterInner::new();
        Self { inner }
    }
}
impl ObjectImpl for PlotterImp {}
impl WindowImpl for PlotterImp {}
impl WidgetImpl for PlotterImp {}

impl PlotterInner {
    pub fn new() -> PlotterInner {
        glib::Object::new()
    }
}

#[derive(Debug, Default, glib::Properties)]
#[properties(wrapper_type = super::PlotterInner)]
pub struct PlotterInnerImp {
    #[property(get, set, minimum = 20.0, maximum = 5000.0, default = 500.0)]
    time_interval_ms: Cell<f32>,
}
#[glib::object_subclass]
impl ObjectSubclass for PlotterInnerImp {
    const NAME: &'static str = "PlotterInner";
    type Type = super::PlotterInner;
    type ParentType = gtk::Widget;

    fn new() -> Self {
        info!("Creating plotter");

        let (tx, rx) = channel::<Vec<f32>>();
        let mut airap = Airap::new().unwrap();
        airap.on_raw(move |data| {
            tx.send(data.to_vec()).unwrap();
        });
        AIRAP.with(|r| *r.borrow_mut() = Some(rx));

        Self {
            time_interval_ms: Cell::new(500.0),
        }
    }
}
impl ObjectImpl for PlotterInnerImp {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn constructed(&self) {
        self.parent_constructed();
    }

    fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        Self::derived_set_property(self, id, value, pspec);
        self.obj().queue_draw();
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        Self::derived_property(self, id, pspec)
    }
}
impl WindowImpl for PlotterInnerImp {}
impl DrawingAreaImpl for PlotterInnerImp {}
impl WidgetImpl for PlotterInnerImp {
    fn show(&self) {
        self.parent_show()
    }
    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let width = self.obj().width() as u32;
        let height = self.obj().height() as u32;
        if width == 0 || height == 0 {
            return;
        }

        let bounds = gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
        let context = snapshot.append_cairo(&bounds);
        // CONTEXT.with(move |c| *c.borrow_mut() = Some(context));
        let backend = CairoBackend::new(&context, (width, height)).unwrap();
        self.plot_pdf(backend).unwrap();
    }
}
impl PlotterInnerImp {
    fn plot_pdf<'a>(&self, backend: CairoBackend<'a>) -> Result<(), Box<dyn Error + 'a>> {
        println!("A");
        let time_interval = self.time_interval_ms.get();

        let sample_rate = 480000;
        let plotter_data_len = sample_rate / 2;
        let mut plotter_data = vec![0.0 as f32; plotter_data_len];
        // How does each position in values relate to x-axis
        let x_rate = time_interval.div(plotter_data_len as f32);

        AIRAP.with(|airap| {
            if let Some(rx) = &*airap.borrow() {
                println!("Plotting");
                let root = backend.into_drawing_area();

                let mut chart_builder = ChartBuilder::on(&root);
                chart_builder
                    .margin(10)
                    .set_left_and_bottom_label_area_size(20);
                let mut chart = chart_builder
                    .build_cartesian_2d(time_interval..0.0, -1.0f32..1.0)
                    .unwrap();

                chart
                    .configure_mesh()
                    .light_line_style(BLACK.mix(0.15))
                    .max_light_lines(5)
                    .draw()
                    .unwrap();

                let data = rx.recv().unwrap();
                plotter_data[plotter_data_len - data.len()..plotter_data_len]
                    .copy_from_slice(&data);

                chart
                    .draw_series(AreaSeries::new(
                        plotter_data
                            .iter()
                            .enumerate()
                            .map(|(x, y)| (time_interval - x as f32 * x_rate, *y)),
                        0.0,
                        RED,
                    ))
                    .unwrap();

                root.present().unwrap();
            }
        });

        Ok(())
    }
}
