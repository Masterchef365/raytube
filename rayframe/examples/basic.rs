use rayframe::{mainloop, RayFrameApp};

struct App {

}

impl App {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl RayFrameApp for App {
    fn update(&mut self, frame: &mut [[f32; 2]], sample_rate: u32) {
        for i in 0..frame.len() {
            let f = i as f32 / frame.len() as f32;
            let p = f * std::f32::consts::TAU;
            let x = p.cos() / 2.0;
            let y = p.sin() / 2.0;

            frame[i] = [x, y];
        }

        println!("{}", frame.len())
    }
}

fn main() {
    mainloop(App::new()).unwrap();
}
