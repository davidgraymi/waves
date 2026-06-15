use nannou::{color::rgb::Rgb, prelude::*};
use noise::NoiseFn;
use rand::prelude::*;

fn main() {
    nannou::app(model)
        .event(event)
        .update(update)
        .simple_window(view)
        .run();
}

// Slow
const PERLIN_SCALE: f64 = 0.005;
const TIME_STEP: f64 = 0.001;
/// How far out (pixels) to sample for the finite-difference gradient.
/// Larger = reads broader landscape features.
const GRADIENT_H: f64 = 40.0;
/// How strongly the slope accelerates the balloon (px/s² per gradient unit).
const FORCE_SCALE: f32 = 20000.0;
/// Velocity damping (drag). Higher = slower, less oscillation.
const DAMPING: f32 = 1.2;
const NUM_BALLS: i32 = 36;
const BALL_STROKE: f32 = 10.0;

// Fast
// const PERLIN_SCALE: f64 = 0.008;
// const TIME_STEP: f64 = 0.004;
// /// How far out (pixels) to sample for the finite-difference gradient.
// /// Larger = reads broader landscape features.
// const GRADIENT_H: f64 = 10.0;
// /// How strongly the slope accelerates the balloon (px/s² per gradient unit).
// const FORCE_SCALE: f32 = 50000.0;
// /// Velocity damping (drag). Higher = slower, less oscillation.
// const DAMPING: f32 = 0.8;
// const NUM_BALLS: i32 = 36;
// const BALL_STROKE: f32 = 10.0;

fn model(app: &App) -> Model {
    let window: std::cell::Ref<'_, Window> = app.main_window();
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
        movers: movers(app),
        map: Map {
            perlin,
            texture,
            time: 0.0,
        },
        reset: ScreenReset::None,
    }
}

fn event(app: &App, model: &mut Model, event: Event) {
    match event {
        Event::WindowEvent { id: _, simple } => match simple {
            Some(WindowEvent::KeyPressed(key)) => {
                println!("Key pressed: {:?}", key);

                if key == Key::Space {
                    model.movers = movers(app);
                    model.reset = ScreenReset::Pending;
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn update(app: &App, model: &mut Model, update: Update) {
    let dt = update.since_last.as_secs_f32();
    let win = app.window_rect();
    match model.reset {
        ScreenReset::None => {}
        ScreenReset::Pending => {
            model.reset = ScreenReset::Confirmed;
        }
        ScreenReset::Confirmed => {
            model.reset = ScreenReset::None;
        }
    };

    model.movers.iter_mut().for_each(|m| {
        let (new_pos, new_vel) = rk4_step(
            &model.map.perlin,
            m.position,
            m.velocity,
            model.map.time,
            dt,
            &win,
        );
        let mut pos = new_pos;
        let mut vel = new_vel;

        if pos.x < win.left() {
            pos.x = win.left();
            vel.x = vel.x.abs();
        } else if pos.x > win.right() {
            pos.x = win.right();
            vel.x = -vel.x.abs();
        }
        if pos.y < win.bottom() {
            pos.y = win.bottom();
            vel.y = vel.y.abs();
        } else if pos.y > win.top() {
            pos.y = win.top();
            vel.y = -vel.y.abs();
        }

        m.position = pos;
        m.velocity = vel;
    });

    model.map.step(app);
}

fn view(app: &App, model: &Model, frame: Frame) {
    let win: Rect = app.window_rect();
    let draw = app.draw();

    match model.reset {
        ScreenReset::None => {
            // model.map.show(&draw, &win);

            model.movers.iter().for_each(|m| {
                draw.ellipse().xy(m.position).radius(BALL_STROKE).color(m.color);
            });
        }
        ScreenReset::Confirmed => {
            draw.background().color(BLACK);
        }
        ScreenReset::Pending => {}
    }

    draw.to_frame(app, &frame).unwrap();
}

struct Model {
    movers: Vec<Mover>,
    map: Map,
    reset: ScreenReset,
}

struct Mover {
    position: Vec2,
    velocity: Vec2,
    color: Rgb,
}

enum ScreenReset {
    None,
    Pending,
    Confirmed,
}

/// Convert a nannou screen position to texture pixel coordinates.
/// Nannou: origin at center, y up. Texture: origin at top-left, y down.
fn nannou_to_pixel(pos: Vec2, win: &Rect) -> Vec2 {
    Vec2::new(pos.x - win.left(), win.top() - pos.y)
}

/// Gradient of the perlin field at `pos` (nannou coords), returned in nannou force-space.
/// Samples in texture pixel coordinates so it matches what's displayed.
fn perlin_gradient(perlin: &noise::Perlin, pos: Vec2, time: f64, win: &Rect) -> Vec2 {
    let s = PERLIN_SCALE;
    let h = GRADIENT_H;
    let p = nannou_to_pixel(pos, win);
    let px = p.x as f64;
    let py = p.y as f64;

    // Gradient in pixel space (x right, y down)
    let gx = (perlin.get([(px + h) * s, py * s, time]) - perlin.get([(px - h) * s, py * s, time]))
        / (2.0 * h);
    let gy_down = (perlin.get([px * s, (py + h) * s, time])
        - perlin.get([px * s, (py - h) * s, time]))
        / (2.0 * h);

    // pixel y-down maps to nannou y-up, so flip gy when returning as a nannou-space force
    Vec2::new(gx as f32, -gy_down as f32)
}

/// Derivatives of the state (pos, vel) — the RHS of the ODE system.
/// dpos/dt = vel
/// dvel/dt = -grad * FORCE_SCALE  -  vel * DAMPING
fn derivatives(
    perlin: &noise::Perlin,
    pos: Vec2,
    vel: Vec2,
    time: f64,
    win: &Rect,
) -> (Vec2, Vec2) {
    let grad = perlin_gradient(perlin, pos, time, win);
    let accel = -grad * FORCE_SCALE - vel * DAMPING;
    (vel, accel)
}

/// Advance (pos, vel) one step using RK4, field frozen at `time`.
fn rk4_step(
    perlin: &noise::Perlin,
    pos: Vec2,
    vel: Vec2,
    time: f64,
    dt: f32,
    win: &Rect,
) -> (Vec2, Vec2) {
    let (dp1, dv1) = derivatives(perlin, pos, vel, time, win);
    let (dp2, dv2) = derivatives(
        perlin,
        pos + dp1 * (dt / 2.0),
        vel + dv1 * (dt / 2.0),
        time,
        win,
    );
    let (dp3, dv3) = derivatives(
        perlin,
        pos + dp2 * (dt / 2.0),
        vel + dv2 * (dt / 2.0),
        time,
        win,
    );
    let (dp4, dv4) = derivatives(perlin, pos + dp3 * dt, vel + dv3 * dt, time, win);

    let new_pos = pos + (dp1 + dp2 * 2.0 + dp3 * 2.0 + dp4) * (dt / 6.0);
    let new_vel = vel + (dv1 + dv2 * 2.0 + dv3 * 2.0 + dv4) * (dt / 6.0);
    (new_pos, new_vel)
}

fn movers(app: &App) -> Vec<Mover> {
    let window: std::cell::Ref<'_, Window> = app.main_window();
    let window_rect = window.rect();

    let ball_count = (NUM_BALLS as f32).sqrt() as i32;

    let positions: Vec<Vec2> =
        distribute_points_1d(ball_count, window_rect.bottom() * 0.75, window_rect.top() * 0.75)
            .flat_map(|y| {
                distribute_points_1d(ball_count, window_rect.left() * 0.75, window_rect.right() * 0.75)
                    .map(move |x| Vec2::new(x, y))
            })
            .collect();

    let total = positions.len() as f32;
    positions
        .into_iter()
        .enumerate()
        .map(|(i, position)| {
            let hue = (i as f32 / total) * 360.0;
            let color: Rgb = nannou::color::hsl(hue / 360.0, 1.0, 0.6).into();
            Mover {
                position,
                velocity: Vec2::ZERO,
                color,
            }
        })
        .collect()
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

        // Generate raw pixel data (a checkerboard pattern)
        let mut pixels = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 4;

                let nx = x as f64 * PERLIN_SCALE;
                let ny = y as f64 * PERLIN_SCALE;

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

        self.time += TIME_STEP;
    }

    fn show(&self, draw: &Draw, win: &Rect) {
        draw.texture(&self.texture).xy(win.xy()).wh(win.wh());
    }
}

fn distribute_points_1d(num_points: i32, start: f32, end: f32) -> impl Iterator<Item = f32> {
    let denominator = if num_points > 1 { num_points - 1 } else { 1 };
    let step = (end - start) / denominator as f32;

    (0..num_points).map(move |i| start + (i as f32 * step))
}
