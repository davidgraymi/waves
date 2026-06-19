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

fn distribute_points_1d(num_points: i32, start: f32, end: f32) -> impl Iterator<Item = f32> {
    let denominator = if num_points > 1 { num_points - 1 } else { 1 };
    let step = (end - start) / denominator as f32;

    (0..num_points).map(move |i| start + (i as f32 * step))
}
