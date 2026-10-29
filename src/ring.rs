// The ring's motion and drawing, kept free of window code so it can be tested.
use std::{mem::zeroed, ptr::null_mut};
use windows_sys::Win32::{Graphics::Gdi::*, UI::WindowsAndMessaging::*};

// colours from the site: warm off-white ring, accent gold over links (RGB)
const RING: (f32, f32, f32, f32) = (240.0, 220.0, 192.0, 0.75);
const LINK: (f32, f32, f32, f32) = (240.0, 164.0, 90.0, 1.0);
const DIAMETER: f32 = 30.0;           // logical px (scaled by DPI)
const LINK_DIAMETER: f32 = 42.0;
const FOLLOW: f32 = 0.38;             // fraction of the gap closed per 60 Hz frame (snappier than the site's 0.18)

pub struct State {
    pub x: f32,
    pub y: f32,
    size: f32,
    link: f32,                        // 0 = normal, 1 = gold; eased
    angle: f32,
    stretch: f32,
    scale: f32,
}

impl State {
    pub fn new(scale: f32) -> Self {
        Self { x: -1e4, y: -1e4, size: DIAMETER, link: 0.0, angle: 0.0, stretch: 1.0, scale }
    }

    // Moves the ring toward the pointer; returns false when nothing visibly changed (skip the redraw).
    pub fn step(&mut self, (tx, ty): (f32, f32), over_link: bool, dt: f32) -> bool {
        if self.x < -1e3 { self.x = tx; self.y = ty; }                 // first frame: start on the pointer
        let k = 1.0 - (1.0 - FOLLOW).powf(dt * 60.0);                   // frame-rate independent easing
        let (vx, vy) = ((tx - self.x) * k, (ty - self.y) * k);
        self.x += vx;
        self.y += vy;
        let speed = (vx * vx + vy * vy).sqrt() / self.scale;
        if speed > 0.3 { self.angle = vy.atan2(vx); }
        let stretch_to = 1.0 + (speed / 30.0).min(0.5);
        self.stretch += (stretch_to - self.stretch) * 0.3;
        let link_to = if over_link { 1.0 } else { 0.0 };
        self.link += (link_to - self.link) * (1.0 - 0.8f32.powf(dt * 60.0));
        let size_to = DIAMETER + (LINK_DIAMETER - DIAMETER) * self.link;
        self.size += (size_to - self.size) * 0.3;
        speed > 0.02 || (self.stretch - 1.0).abs() > 0.002 || (self.link - link_to).abs() > 0.002
    }

    // Draws the ring centred in a box x box premultiplied-BGRA buffer, anti-aliased.
    pub fn draw(&self, px: &mut [u32], boxsz: i32) {
        let r = self.size * 0.5 * self.scale;
        let (a, b) = (r * self.stretch, r / self.stretch);               // stretched along the motion
        let line = (1.0 + 0.6 * self.link) * self.scale;                 // a touch bolder over links
        let mix = |p: f32, q: f32| p + (q - p) * self.link;
        let (cr, cg, cb, ca) = (mix(RING.0, LINK.0), mix(RING.1, LINK.1), mix(RING.2, LINK.2), mix(RING.3, LINK.3));
        let (sin, cos) = self.angle.sin_cos();
        let c = boxsz as f32 / 2.0;
        for j in 0..boxsz {
            for i in 0..boxsz {
                let (dx, dy) = (i as f32 + 0.5 - c, j as f32 + 0.5 - c);
                let (lx, ly) = (dx * cos + dy * sin, -dx * sin + dy * cos);   // into the ring's frame
                let d = ((lx / a).powi(2) + (ly / b).powi(2)).sqrt();
                let dist = (d - 1.0).abs() * a.min(b);                     // ~px from the outline
                let cov = (line * 0.5 + 0.5 - dist).clamp(0.0, 1.0) * ca;
                px[(j * boxsz + i) as usize] = premul(cr, cg, cb, cov);
            }
        }
    }
}

fn premul(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let a8 = (a * 255.0) as u32;
    let ch = |v: f32| ((v * a) as u32).min(a8);
    (a8 << 24) | (ch(r) << 16) | (ch(g) << 8) | ch(b)
}

// The site's glowing dot (#fff1d6 core, warm halo), centred in an s x s buffer; `scale` = DPI factor.
fn draw_dot(px: &mut [u32], s: i32, scale: f32) {
    let c = s as f32 / 2.0;
    for j in 0..s {
        for i in 0..s {
            let d = ((i as f32 + 0.5 - c).powi(2) + (j as f32 + 0.5 - c).powi(2)).sqrt() / scale;
            let core = (3.5 - d).clamp(0.0, 1.0);
            let glow = 0.55 * (-(d / 4.5).powi(2)).exp() * (1.0 - core);
            let a = core + glow;
            if a < 0.004 { continue; }
            let mix = |p: f32, q: f32| (p * core + q * glow) / a;
            px[(j * s + i) as usize] = premul(mix(255.0, 255.0), mix(241.0, 217.0), mix(214.0, 160.0), a);
        }
    }
}

// The dot as a cursor with its hotspot in the centre, sized for the screen's DPI.
pub fn dot_cursor(scale: f32) -> HCURSOR {
    let s = ((32.0 * scale).round() as i32).max(32);
    let mut px = vec![0u32; (s * s) as usize];
    draw_dot(&mut px, s, scale);
    unsafe { make_icon(&px, s, false) }
}

// 32x32 tray icon: the ring with the dot inside
pub fn tray_icon() -> HICON {
    const S: i32 = 32;
    let mut px = vec![0u32; (S * S) as usize];
    let state = State { x: 0.0, y: 0.0, size: 26.0, link: 0.0, angle: 0.0, stretch: 1.0, scale: 1.0 };
    state.draw(&mut px, S);
    draw_dot(&mut px, S, 1.0);
    unsafe { make_icon(&px, S, true) }
}

unsafe fn make_icon(px: &[u32], s: i32, is_icon: bool) -> HICON {
    let mut bmi: BITMAPINFO = zeroed();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: s, biHeight: -s, biPlanes: 1,
        biBitCount: 32, biCompression: BI_RGB as u32, ..zeroed()
    };
    let mut bits: *mut core::ffi::c_void = null_mut();
    let color = CreateDIBSection(null_mut(), &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    std::ptr::copy_nonoverlapping(px.as_ptr(), bits as *mut u32, px.len());
    let mask = CreateBitmap(s, s, 1, 1, null_mut());
    let hot = (s / 2) as u32;
    let info = ICONINFO { fIcon: is_icon as i32, xHotspot: hot, yHotspot: hot, hbmMask: mask, hbmColor: color };
    let icon = CreateIconIndirect(&info);
    DeleteObject(color);
    DeleteObject(mask);
    icon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_catches_up_and_settles() {
        let mut s = State::new(1.0);
        s.step((0.0, 0.0), false, 1.0 / 60.0);
        for _ in 0..240 { s.step((100.0, 50.0), false, 1.0 / 60.0); }
        assert!((s.x - 100.0).abs() < 0.5 && (s.y - 50.0).abs() < 0.5);
        assert!(!s.step((100.0, 50.0), false, 1.0 / 60.0), "settled ring should skip redraws");
    }

    #[test]
    fn fast_motion_stretches_and_links_grow() {
        let mut s = State::new(1.0);
        s.step((0.0, 0.0), false, 1.0 / 60.0);
        for _ in 0..3 { s.step((400.0, 0.0), false, 1.0 / 60.0); }
        assert!(s.stretch > 1.2);
        for _ in 0..120 { s.step((400.0, 0.0), true, 1.0 / 60.0); }
        assert!(s.size > 40.0 && s.link > 0.95);
    }

    #[test]
    fn draws_an_outline_not_a_disc() {
        let s = State { x: 0.0, y: 0.0, size: 30.0, link: 0.0, angle: 0.0, stretch: 1.0, scale: 1.0 };
        let mut px = vec![0u32; 64 * 64];
        s.draw(&mut px, 64);
        let alpha = |i: usize, j: usize| px[j * 64 + i] >> 24;
        assert_eq!(alpha(32, 32), 0, "centre stays clear");
        assert!(alpha(32 + 15, 32) > 60, "outline at the radius");
        assert_eq!(alpha(2, 2), 0, "corners clear");
    }
}
