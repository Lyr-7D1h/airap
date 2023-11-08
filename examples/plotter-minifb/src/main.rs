use airap::feature::{feature_flags, Feature, RawFeature};
use airap::{Device, MovingAverageEvent, RawEvent, Runner};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use plotters::prelude::*;
use plotters_bitmap::bitmap_pixel::BGRXPixel;
use plotters_bitmap::BitMapBackend;
use std::borrow::{Borrow, BorrowMut};
use std::collections::VecDeque;
use std::error::Error;
use std::ops::Div;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::{Duration, SystemTime};

const W: usize = 1000;
const H: usize = 800;

const TIME_INTERVAL: f32 = 200.0;
const SAMPLE_RATE: usize = 48_000;
const FRAME_RATE: f64 = 30.0; // TODO maximize to frame rate
const DOWN_SAMPLE: usize = 5;

struct BufferWrapper(Vec<u32>);
impl Borrow<[u8]> for BufferWrapper {
    fn borrow(&self) -> &[u8] {
        // Safe for alignment: align_of(u8) <= align_of(u32)
        // Safe for cast: u32 can be thought of as being transparent over [u8; 4]
        unsafe { std::slice::from_raw_parts(self.0.as_ptr() as *const u8, self.0.len() * 4) }
    }
}
impl BorrowMut<[u8]> for BufferWrapper {
    fn borrow_mut(&mut self) -> &mut [u8] {
        // Safe for alignment: align_of(u8) <= align_of(u32)
        // Safe for cast: u32 can be thought of as being transparent over [u8; 4]
        unsafe { std::slice::from_raw_parts_mut(self.0.as_mut_ptr() as *mut u8, self.0.len() * 4) }
    }
}
impl Borrow<[u32]> for BufferWrapper {
    fn borrow(&self) -> &[u32] {
        self.0.as_slice()
    }
}
impl BorrowMut<[u32]> for BufferWrapper {
    fn borrow_mut(&mut self) -> &mut [u32] {
        self.0.as_mut_slice()
    }
}

pub struct Audio<const CHANNELS: usize> {
    channels: [Receiver<Vec<f32>>; CHANNELS],
}

impl<const CHANNELS: usize> Audio<CHANNELS> {
    fn load(features: &[Feature; 2]) -> Audio<2> {
        let (raw_tx, raw_rx) = mpsc::channel::<Vec<f32>>();
        let (average_tx, average_rx) = mpsc::channel::<Vec<f32>>();

        thread::spawn(move || {
            let i = Arc::new(Mutex::new(0));
            Runner::new()
                .subscribe(&[Feature::default(feature_flags::MOVING_AVERAGE)])
                .unwrap()
                .listen(move |e| match e {
                    airap::Event::Raw(RawEvent { data, latency }) => {
                        // print latency every 100 events
                        let mut mi = i.lock().unwrap();
                        *mi = (*mi + 1) % 100;
                        if *mi == 0 {
                            println!("{:?}", latency.internal)
                        }

                        let data: Vec<f32> = data.iter().step_by(DOWN_SAMPLE).cloned().collect();
                        raw_tx.send(data).unwrap();
                    }
                    airap::Event::MovingAverage(MovingAverageEvent { average, .. }) => {
                        average_tx.send(average).unwrap();
                    }
                    _ => {}
                })
                .unwrap();
        });

        Audio {
            channels: [raw_rx, average_rx],
        }
    }

    fn try_recv(&self) -> [Vec<f32>; CHANNELS] {
        let mut data: [Vec<f32>; CHANNELS] = vec![Vec::new(); CHANNELS].try_into().expect("static");

        for (i, c) in self.channels.iter().enumerate() {
            while let Ok(rd) = c.try_recv() {
                data[i].extend(rd);
            }
        }

        data
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();

    let audio = Audio::<2>::load(&[Feature::default(feature_flags::RAW), Feature::MovingAverage]);

    show_window(audio)?;
    // loop {}
    Ok(())
}

pub fn show_window(audio: Audio<2>) -> Result<(), Box<dyn Error>> {
    let mut buf = BufferWrapper(vec![0u32; W * H]);

    let mut window = Window::new(
        "Plotter",
        // &get_window_title(fx, fy, yphase - xphase),
        W,
        H,
        WindowOptions::default(),
    )?;
    let cs = {
        let root = BitMapBackend::<BGRXPixel>::with_buffer_and_format(
            buf.borrow_mut(),
            (W as u32, H as u32),
        )?
        .into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .set_left_and_bottom_label_area_size(20)
            .build_cartesian_2d(TIME_INTERVAL..0.0, -1.0f32..1.0)
            .unwrap();

        chart.configure_mesh().disable_mesh().draw()?;

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperLeft);
        let cs = chart.into_chart_state();
        root.present()?;
        cs
    };
    window.update_with_buffer(buf.borrow(), W, H)?;

    let plotter_raw_data_len = SAMPLE_RATE as f32 / (1000.0 / TIME_INTERVAL) / DOWN_SAMPLE as f32;
    // Adding an extra data point because we also want 0 to contain data
    let mut plotter_raw_data = vec![0.0 as f32; plotter_raw_data_len as usize + 1];
    // How does each position in values relate to x-axis
    let plotter_raw_data_x_rate = TIME_INTERVAL.div(plotter_raw_data_len);

    // fragrate / 4 = 240
    let plotter_average_data_len = SAMPLE_RATE as f32 / (1000.0 / TIME_INTERVAL) / 240 as f32;
    // Adding an extra data point because we also want 0 to contain data
    let mut plotter_average_data = vec![0.0 as f32; plotter_average_data_len as usize + 1];
    let plotter_average_data_x_rate = TIME_INTERVAL.div(plotter_average_data_len as f32);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        {
            let [raw_data, average_data] = audio.try_recv();
            plotter_raw_data.drain(0..raw_data.len());
            plotter_raw_data.extend(raw_data);
            plotter_average_data.drain(0..average_data.len());
            plotter_average_data.extend(average_data);

            let root = BitMapBackend::<BGRXPixel>::with_buffer_and_format(
                buf.borrow_mut(),
                (W as u32, H as u32),
            )?
            .into_drawing_area();
            {
                let mut chart = cs.clone().restore(&root);
                chart.plotting_area().fill(&WHITE)?;

                chart
                    .draw_series(LineSeries::new(
                        plotter_raw_data
                            .iter()
                            .enumerate()
                            .map(|(x, y)| (TIME_INTERVAL - x as f32 * plotter_raw_data_x_rate, *y)),
                        &RED,
                    ))
                    .unwrap()
                    .label("raw");

                chart
                    .draw_series(LineSeries::new(
                        plotter_average_data.iter().enumerate().map(|(x, y)| {
                            (TIME_INTERVAL - x as f32 * plotter_average_data_x_rate, *y)
                        }),
                        BLUE.stroke_width(2),
                    ))
                    .unwrap()
                    .label("average")
                    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));
            }
            root.present()?;
        }

        // let keys = window.get_keys_pressed(KeyRepeat::Yes);
        // for key in keys {
        //     let old_fx = fx;
        //     let old_fy = fy;
        //     match key {
        //         Key::Equal => {
        //             fy += 0.1;
        //         }
        //         Key::Minus => {
        //             fy -= 0.1;
        //         }
        //         Key::Key0 => {
        //             fx += 0.1;
        //         }
        //         Key::Key9 => {
        //             fx -= 0.1;
        //         }
        //         _ => {
        //             continue;
        //         }
        //     }
        //     xphase += 2.0 * epoch * std::f64::consts::PI * (old_fx - fx);
        //     yphase += 2.0 * epoch * std::f64::consts::PI * (old_fy - fy);
        //     window.set_title(&get_window_title(fx, fy, yphase - xphase));
        // }
        window.update_with_buffer(buf.borrow(), W, H)?;
    }

    Ok(())
}
