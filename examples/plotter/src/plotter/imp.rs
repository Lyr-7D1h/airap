use airap::Airap;
use gtk::cairo;
use gtk::gio;
use gtk::glib;
use gtk::glib::idle_add;
use gtk::glib::idle_add_local;
use gtk::glib::idle_add_once;
use gtk::glib::timeout_add;
use gtk::glib::timeout_add_local;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use log::info;
use plotters::coord::types::RangedCoordf32;
use plotters::coord::Shift;

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

use plotters::prelude::*;
use plotters_cairo::CairoBackend;

// https://stackoverflow.com/questions/66510406/gtk-rs-how-to-update-view-from-another-thread
thread_local!(
    static CONTEXT: RefCell<Option<cairo::Context>> = RefCell::new(None);
    static AIRAP: RefCell<Option<mpsc::Receiver<Vec<f32>>>> = RefCell::new(None);
);

#[derive(Debug, glib::Properties)]
#[properties(wrapper_type = super::Plotter)]
pub struct Plotter {
    #[property(get, set, minimum = 20.0, maximum = 5000.0, default = 500.0)]
    time_interval_ms: Cell<f32>,
}

#[glib::object_subclass]
impl ObjectSubclass for Plotter {
    const NAME: &'static str = "Plotter";
    type Type = super::Plotter;
    type ParentType = gtk::Widget;

    fn new() -> Self {
        info!("Creating plotter");

        let (tx, rx) = channel::<Vec<f32>>();
        // let mut airap = Airap::new().unwrap();
        // airap.on_raw(move |data| {
        //     // tx.send(data.to_vec()).unwrap();
        //     // println!("{data:?}");
        //     // plotter_data.lock().unwrap()[plotter_data_len - data.len()..plotter_data_len]
        //     //     .copy_from_slice(&data);
        // });
        AIRAP.with(|r| *r.borrow_mut() = Some(rx));

        Self {
            time_interval_ms: Cell::new(500.0),
        }
    }
}

impl ObjectImpl for Plotter {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        Self::derived_set_property(self, id, value, pspec);
        self.obj().queue_draw();
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        Self::derived_property(self, id, pspec)
    }
}

impl WidgetImpl for Plotter {
    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let width = self.obj().width() as u32;
        let height = self.obj().height() as u32;
        if width == 0 || height == 0 {
            return;
        }

        let bounds = gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
        let context = snapshot.append_cairo(&bounds);
        CONTEXT.with(move |c| *c.borrow_mut() = Some(context));
        // let backend = CairoBackend::new(&context, (width, height)).unwrap();
        self.plot_pdf().unwrap();
    }
}

impl Plotter {
    fn plot_pdf<'a>(&self) -> Result<(), Box<dyn Error + 'a>> {
        println!("A");
        let width = self.obj().width() as u32;
        let height = self.obj().height() as u32;
        let time_interval = self.time_interval_ms.get();

        // CHART.with(move |c| *c.borrow_mut() = Some(chart));

        // root.present().unwrap();

        let sample_rate = 480000;

        let plotter_data_len = sample_rate / 2;
        let mut plotter_data = vec![0.0 as f32; plotter_data_len];
        // How does each position in values relate to x-axis
        let x_rate = time_interval.div(plotter_data_len as f32);

        timeout_add_local(Duration::from_millis(2000), move || {
            AIRAP.with(|airap| {
                if let Some(rx) = &*airap.borrow() {
                    CONTEXT.with(|context| {
                        if let Some(context) = &*context.borrow() {
                            println!("Plotting");
                            let backend = CairoBackend::new(context, (width, height)).unwrap();
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

                            // let data = rx.recv().unwrap();
                            // plotter_data[plotter_data_len - data.len()..plotter_data_len]
                            //     .copy_from_slice(&data);

                            // chart
                            //     .draw_series(AreaSeries::new(
                            //         plotter_data
                            //             .iter()
                            //             .enumerate()
                            //             .map(|(x, y)| (time_interval - x as f32 * x_rate, *y)),
                            //         0.0,
                            //         RED,
                            //     ))
                            //     .unwrap();

                            root.present().unwrap();
                        }
                    });
                }
            });
            glib::ControlFlow::Continue
        });

        Ok(())
    }
}
