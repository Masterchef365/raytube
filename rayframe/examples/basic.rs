use std::f32::consts::TAU;

use rayframe::{mainloop, RayFrameApp};

struct App {
    time: u32,
}

impl App {
    pub fn new() -> Self {
        Self {
            time: 0,
        }
    }
}

fn period(time: u32, n: u32) -> f32 {
    (time % n) as f32 / n as f32
}

impl RayFrameApp for App {
    fn update(&mut self, frame: &mut [[f32; 2]], sample_rate: u32) {

        for i in 0..frame.len() {
            self.time += 1;

            let f = period(self.time, sample_rate / 93);
            let p = f * TAU;
            let x = p.cos() / 1.0;
            let y = p.sin() / 1.0;

            let q = period(self.time, sample_rate) * TAU;

            let x = x * q.cos() / 2.0;
            let y = y * q.sin() / 2.0;



            let q = period(self.time, sample_rate / 48) * TAU;

            let x = x + q.cos() / 2.0;
            let y = y + q.sin() / 2.0;

            frame[i] = [x, y];
        }
    }
}

fn main() {
    mainloop(App::new()).unwrap();
}
