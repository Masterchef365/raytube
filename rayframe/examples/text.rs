use rayframe::{mainloop, RayFrameApp};
use vector_text_hershey::{HersheyFont, HersheyRenderer};
use vector_text_core::Renderer;

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

impl RayFrameApp for App {
    fn update(&mut self, frame: &mut [[f32; 2]], sample_rate: u32) {
        self.time += 1;

        let points = HersheyRenderer::render_text(&format!("Hello, world! {}", self.time), HersheyFont::Romanp);
        let max_x = points.iter().map(|pt| pt.x).max().unwrap();

        let downscale = 250.0;
        for (textpt, framept) in points.iter().cycle().zip(frame) {
            let x = -max_x as f32 / 2.0 / downscale;
            let t = self.time as f32 / 250.0;
            let x = x + t.cos() / 2.0;
            let y = t.sin() / 2.0;

            //let t = self.time as f32 / 50.0;
            //let y = (textpt.x as f32 / 10.0 + t).cos() / 30.0;

            framept[0] = textpt.x as f32 / downscale + x;
            framept[1] = -textpt.y as f32 / downscale + y;
        }
    }
}

fn main() {
    mainloop(App::new()).unwrap();
}

