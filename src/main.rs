use nannou::{color::rgb::Rgb, prelude::*};
use noise::NoiseFn;
use rand::prelude::*;

fn main() {
    nannou::app(model).update(update).simple_window(view).run();
}

struct Model {
    mover: Mover,
    map: Map,
}

fn model(app: &App) -> Model {
    let window = app.main_window();
    let window_rect = window.rect();
    let width = window_rect.w() as u32;
    let height = window_rect.h() as u32;

    let mut rng: ThreadRng = ThreadRng::default();
    let seed: u32 = rng.random();
    let perlin = noise::Perlin::new(seed);

    let texture = wgpu::TextureBuilder::new()
        .size([width, height])
        .format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .usage(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST)
        .build(window.device());

    Model {
        mover: Mover::default(),
        map: Map {
            perlin,
            texture,
            time: 0.0,
        },
    }
}

// Fixed: Added the required third parameter `Update`
fn update(app: &App, model: &mut Model, _update: Update) {
    model.mover.position += model.mover.velocity * model.mover.acceleration;

    model.map.step(app);
}

// Fixed: Changed `Entity` to `window::Id`
fn view(app: &App, model: &Model, frame: Frame) {
    let win: Rect = app.window_rect();
    let draw = app.draw();

    draw.ellipse()
        .xy(model.mover.position)
        .radius(1.0)
        .color(model.mover.color);

    model.map.show(&draw, &win);

    // let mut rng = ThreadRng::default();
    // let normal = Normal::new(0.0, 100.0).unwrap();

    // let num_points = 250;
    // let xs = normal.sample_iter(rng.clone()).take(num_points);
    // let ys = normal.sample_iter(&mut rng).take(num_points);
    // let points = xs.zip(ys).map(|(x, y)| pt2(x, y));

    // for pt in points {
    //     draw.ellipse()
    //         .xy(pt)
    //         .radius(10.0)
    //         .color(rgba(1.0, 1.0, 1.0, 0.01));
    // }

    // let points = distribute_points_1d(num_points, win.left(), win.right());
    // let points = points.map(|x| {
    //     let point = pt2(x, (x / 20.0).sin() * 50.0);
    //     (point, STEELBLUE)
    // });
    // draw.polyline().weight(3.0).points_colored(points);

    draw.to_frame(app, &frame).unwrap();
}

fn distribute_points_1d(num_points: i32, start: f32, end: f32) -> impl Iterator<Item = f32> {
    let denominator = if num_points > 1 { num_points - 1 } else { 1 };
    let step = (end - start) / denominator as f32;

    (0..num_points).map(move |i| start + (i as f32 * step))
}

struct Map {
    perlin: noise::Perlin,
    texture: wgpu::Texture,
    time: f64,
}

impl Map {
    fn step(&mut self, app: &App) {
        let window = app.main_window();
        let texture_size = self.texture.size();
        let width = texture_size[0] as usize;
        let height = texture_size[1] as usize;

        let scale = 0.008;

        // Generate raw pixel data (a checkerboard pattern)
        let mut pixels = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 4;

                let nx = x as f64 * scale;
                let ny = y as f64 * scale;

                let red_noise: f64 = self.perlin.get([nx, ny, self.time]);
                // let green_noise: f64 = self.perlin.get([nx, ny, self.time + 10000.0]);
                // let blue_noise: f64 = self.perlin.get([nx, ny, self.time + 20000.0]);
                // let alpha_noise: f64 = self.perlin.get([nx, ny, self.time + 30000.0]);
                let r = ((red_noise + 1.0) * 127.5) as u8;
                // let g = ((green_noise + 1.0) * 127.5) as u8;
                // let b = ((blue_noise + 1.0) * 127.5) as u8;
                // let a = ((alpha_noise + 1.0) * 127.5) as u8;

                pixels[i] = r; // Red
                pixels[i + 1] = 0; // Green
                pixels[i + 2] = 0; // Blue
                pixels[i + 3] = 255; // Alpha
            }
        }

        // Upload pixel data to the GPU Texture
        let slice = pixels.as_slice();
        let mut encoder = window
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("upload_perlin_noise"),
            });
        self.texture
            .upload_data(window.device(), &mut encoder, slice);
        window.queue().submit(Some(encoder.finish()));

        self.time += 0.01;
    }

    fn show(&self, draw: &Draw, win: &Rect) {
        draw.texture(&self.texture).xy(win.xy()).wh(win.wh());
    }
}

#[derive(Default)]
struct Mover {
    position: Vec2,
    velocity: Vec2,
    acceleration: Vec2,
    color: Rgb,
    rng: ThreadRng,
}
