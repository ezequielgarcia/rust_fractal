use num::complex::Complex;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Point;
use threadpool::ThreadPool;
use std::thread;
use std::time::{Duration, Instant};

const WIDTH: u32 = 300;
const HEIGHT: u32 = 300;

// Function converting intensity values to RGB
// Based on http://www.efg2.com/Lab/ScienceAndEngineering/Spectra.htm
fn wavelength_to_rgb(wavelength: u32) -> Color {
    let wave = wavelength as f32;

    let (r, g, b) = match wavelength {
        380..=439 => ((440. - wave) / (440. - 380.), 0.0, 1.0),
        440..=489 => (0.0, (wave - 440.) / (490. - 440.), 1.0),
        490..=509 => (0.0, 1.0, (510. - wave) / (510. - 490.)),
        510..=579 => ((wave - 510.) / (580. - 510.), 1.0, 0.0),
        580..=644 => (1.0, (645. - wave) / (645. - 580.), 0.0),
         645..=780 => (1.0, 0.0, 0.0),
        _ => (0.0, 0.0, 0.0),
    };

    let factor = match wavelength {
        380..=419 => 0.3 + 0.7 * (wave - 380.) / (420. - 380.),
        701..=780 => 0.3 + 0.7 * (780. - wave) / (780. - 700.),
        _ => 1.0,
    };

    let (r, g, b) = (normalize(r, factor), normalize(g, factor), normalize(b, factor));
    Color::RGB(r, g, b)
}

// Maps Julia set distance estimation to intensity values
fn julia(c: Complex<f32>, x: u32, y: u32, width: u32, height: u32, max_iter: u32) -> u32 {
    let width = width as f32;
    let height = height as f32;

    let mut z = Complex {
        // scale and translate the point to image coordinates
        re: 3.0 * (x as f32 - 0.5 * width) / width,
        im: 2.0 * (y as f32 - 0.5 * height) / height,
     };

    let mut i = 0;
    for t in 0..max_iter {
        if z.norm() >= 2.0 {
            break;
        }
        z = z * z + c;
        i = t;
    }
    i
}

// Normalizes color intensity values within RGB range
fn normalize(color: f32, factor: f32) -> u8 {
    ((color * factor).powf(0.8) * 255.) as u8
}

struct PixelDrawEvent {
    x: u32,
    y: u32,
    color: Color,
}

fn main() {
    let (width, height) = (WIDTH, HEIGHT);
    let iterations = 300;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("fractal", width, height)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window
        .into_canvas()
        .build()
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let ev = sdl_context.event().unwrap();

    /* Register a pixel draw event */
    ev.register_custom_event::<PixelDrawEvent>().unwrap();

    let c = Complex::new(-0.8, 0.156);

    let pool = ThreadPool::new(num_cpus::get() * 2);

    for y in 0..height {
        let sender = ev.event_sender();
        pool.execute(move || -> () {
            for x in 0..width {
                let i = julia(c, x, y, width, height, iterations);
                let pixel = wavelength_to_rgb(380 + i * 400 / iterations);
                let pixel_draw = PixelDrawEvent { x: x, y: y, color: pixel };
                sender
                    .push_custom_event(pixel_draw)
                    .unwrap_or_else(|_| {
                        thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
                    });
            }
        });
    }

    'running: loop {
        let start = Instant::now();

        for event in event_pump.poll_iter() {
            if event.is_user_event() {
                let pixel = event.as_user_event_type::<PixelDrawEvent>().unwrap();

                canvas.set_draw_color(pixel.color);
                canvas.draw_point(Point::new(pixel.x as i32, pixel.y as i32)).unwrap();

                if start.elapsed() >= Duration::new(0, 1_000_000_000u32 / 60) {
                    canvas.present();
                }

                continue;
            }
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
    }
}
