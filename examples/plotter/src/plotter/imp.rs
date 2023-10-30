use airap::Airap;
use gtk::gio;
use gtk::glib;
use gtk::glib::idle_add;
use gtk::glib::idle_add_local;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use log::info;
use plotters::coord::Shift;

use std::cell::Cell;
use std::cell::RefCell;
use std::error::Error;
use std::ops::Div;
use std::sync::mpsc;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

use plotters::prelude::*;
use plotters_cairo::CairoBackend;

// https://stackoverflow.com/questions/66510406/gtk-rs-how-to-update-view-from-another-thread
thread_local!(
    static GLOBAL: RefCell<
        Option<(
            mpsc::Receiver<Vec<f32>>,
            Arc<DrawingArea<CairoBackend<'static>, Shift>>,
        )>,
    > = RefCell::new(None);
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
        Self {
            time_interval_ms: Cell::new(500.0),
        }
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        // obj.init_template();
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
        let backend = CairoBackend::new(&context, (width, height)).unwrap();
        self.plot_pdf(backend).unwrap();
    }
}

impl Plotter {
    fn plot_pdf<'a>(&self, backend: CairoBackend<'a>) -> Result<(), Box<dyn Error + 'a>> {
        let root = Arc::new(backend.into_drawing_area());

        root.fill(&WHITE)?;

        let mut chart_builder = ChartBuilder::on(root.as_ref());
        chart_builder
            .margin(10)
            .set_left_and_bottom_label_area_size(20);

        let sample_rate = 480000;
        let time_interval = self.time_interval_ms.get();

        let mut chart = chart_builder.build_cartesian_2d(time_interval..0.0, -1.0f32..1.0)?;

        chart
            .configure_mesh()
            .light_line_style(BLACK.mix(0.15))
            .max_light_lines(5)
            .draw()?;

        root.present().unwrap();

        // How does each position in values relate to x-axis
        let (tx, rx) = channel::<Vec<f32>>();
        let mut airap = Airap::new().unwrap();
        airap.on_raw(move |data| {
            tx.send(data.to_vec()).unwrap();
            // println!("{data:?}");
            // plotter_data.lock().unwrap()[plotter_data_len - data.len()..plotter_data_len]
            //     .copy_from_slice(&data);
        });

        GLOBAL.with(|global| *global.borrow_mut() = Some((rx, root)));

        let plotter_data_len = sample_rate / 2;
        let mut plotter_data = vec![0.0 as f32; plotter_data_len];
        let x_rate = time_interval.div(plotter_data_len as f32);
        let update_interval = Duration::from_millis(100); // 100 milliseconds

        idle_add_local(move || {
            GLOBAL.with(|global| {
                if let Some((rx, root)) = &*global.borrow() {
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
            glib::ControlFlow::Continue
        });

        Ok(())
    }
}
