use rayframe::{mainloop, RayFrameApp};

struct App {
    time: f32,
}

impl App {
    pub fn new() -> Self {
        Self {
            time: 0.0,
        }
    }
}

impl RayFrameApp for App {
    fn update(&mut self, frame: &mut [[f32; 2]], sample_rate: u32) {

        for i in 0..frame.len() {
            let f = i as f32 / frame.len() as f32;
            let p = f * std::f32::consts::TAU;
            let x = p.cos() / 1.0;
            let y = p.sin() / 1.0;

            self.time += 1.0 / frame.len() as f32;
            let x = x * (self.time / 1.0).cos() / 2.0;
            let y = y * (self.time / 1.0).sin() / 2.0;

            frame[i] = [x, y];
        }


        println!("{} {}", frame.len(), sample_rate);
    }
}

fn main() {
    mainloop(App::new()).unwrap();
}
