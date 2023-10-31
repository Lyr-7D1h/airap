use airap::{Airap, Options};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use plotters::prelude::*;
use plotters_bitmap::bitmap_pixel::BGRXPixel;
use plotters_bitmap::BitMapBackend;
use std::borrow::{Borrow, BorrowMut};
use std::collections::VecDeque;
use std::error::Error;
use std::ops::Div;
use std::sync::mpsc;
use std::time::SystemTime;

const W: usize = 1000;
const H: usize = 800;

const TIME_INTERVAL: f32 = 2000.0;
const SAMPLE_RATE: usize = 48_000;
const FRAME_RATE: f64 = 30.0; // TODO maximize to frame rate
const DOWN_SAMPLE: usize = 10;

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

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();

    let (tx, rx) = mpsc::channel::<Vec<f32>>();

    let mut airap = Airap::new().unwrap();
    airap.on_raw(Options::default(), move |data| {
        let data: Vec<f32> = data.to_owned().into_iter().step_by(DOWN_SAMPLE).collect();
        tx.send(data).unwrap();
    });

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

        chart
            .configure_mesh()
            .light_line_style(BLACK.mix(0.15))
            .max_light_lines(5)
            .draw()?;

        let cs = chart.into_chart_state();
        root.present()?;
        cs
    };
    window.update_with_buffer(buf.borrow(), W, H)?;

    let plotter_data_len = SAMPLE_RATE / 2 / DOWN_SAMPLE;
    let mut plotter_data = vec![0.0 as f32; plotter_data_len];
    // How does each position in values relate to x-axis
    let x_rate = TIME_INTERVAL.div(plotter_data_len as f32);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        {
            let data = rx.recv().unwrap();
            // println!("{data:?}");
            // println!("{:?}", data.len());
            plotter_data.drain(0..data.len());
            plotter_data.extend(data);
            // println!("{plotter_data_len}");
            // plotter_data[plotter_data_len - data.len()..plotter_data_len].copy_from_slice(&data);

            let root = BitMapBackend::<BGRXPixel>::with_buffer_and_format(
                buf.borrow_mut(),
                (W as u32, H as u32),
            )?
            .into_drawing_area();
            {
                let mut chart = cs.clone().restore(&root);
                chart.plotting_area().fill(&WHITE)?;

                chart
                    .draw_series(AreaSeries::new(
                        plotter_data
                            .iter()
                            .enumerate()
                            .map(|(x, y)| (TIME_INTERVAL - x as f32 * x_rate, *y)),
                        0.0,
                        RED,
                    ))
                    .unwrap();
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
