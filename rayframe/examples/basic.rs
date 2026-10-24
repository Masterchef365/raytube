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

    }
}

fn main() {
    mainloop(App::new()).unwrap();
}
