/// Convert a nannou screen position to texture pixel coordinates.
/// Nannou: origin at center, y up. Texture: origin at top-left, y down.
fn nannou_to_pixel(pos: Vec2, win: &Rect) -> Vec2 {
    Vec2::new(pos.x - win.left(), win.top() - pos.y)
}