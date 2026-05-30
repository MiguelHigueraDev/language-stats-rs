use crate::colors;
use crate::models::LanguageStat;
use ab_glyph::{FontRef, PxScale};
use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use std::f32::consts::PI;
use std::io::Cursor;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 630;
const CARD_RADIUS: i32 = 24;
const CARD_MARGIN: i32 = 28;
const INNER_PADDING: i32 = 36;

pub fn render_language_card(username: &str, stats: &[LanguageStat]) -> Result<Vec<u8>> {
    let mut img = RgbaImage::from_pixel(WIDTH, HEIGHT, colors::background());
    let font = FontRef::try_from_slice(include_bytes!("../assets/Roboto-Regular.ttf"))
        .context("failed to load embedded font")?;

    let card_x0 = CARD_MARGIN;
    let card_y0 = CARD_MARGIN;
    let card_x1 = (WIDTH as i32) - CARD_MARGIN;
    let card_y1 = (HEIGHT as i32) - CARD_MARGIN;
    fill_rounded_rect(
        &mut img,
        card_x0,
        card_y0,
        card_x1,
        card_y1,
        CARD_RADIUS,
        colors::card(),
    );

    draw_text(
        &mut img,
        &font,
        34.0,
        (card_x0 + INNER_PADDING) as f32,
        (card_y0 + INNER_PADDING) as f32,
        &format!("@{username}"),
        colors::text_primary(),
    );
    draw_text(
        &mut img,
        &font,
        22.0,
        (card_x0 + INNER_PADDING) as f32,
        (card_y0 + INNER_PADDING + 44) as f32,
        "Repository languages",
        colors::text_muted(),
    );

    let content_top = card_y0 + INNER_PADDING + 88;
    let pie_cx = card_x0 + 280;
    let pie_cy = content_top + 200;
    let pie_radius = 175;

    draw_pie_chart(&mut img, pie_cx, pie_cy, pie_radius, stats);

    let bars_x = card_x0 + 560;
    let bars_top = content_top + 10;
    let bar_max_width = 500;
    let row_height = 58;

    for (i, stat) in stats.iter().enumerate() {
        let y = bars_top + (i as i32) * row_height;
        let color = colors::language_color(&stat.name);

        draw_text(
            &mut img,
            &font,
            24.0,
            bars_x as f32,
            y as f32,
            &stat.name,
            colors::text_primary(),
        );

        let bar_y = y + 30;
        let bar_width = ((stat.percentage / 100.0) * bar_max_width as f64).round() as u32;
        draw_filled_rect_mut(
            &mut img,
            imageproc::rect::Rect::at(bars_x, bar_y).of_size(bar_max_width as u32, 10),
            Rgba([48, 54, 61, 255]),
        );
        if bar_width > 0 {
            draw_filled_rect_mut(
                &mut img,
                imageproc::rect::Rect::at(bars_x, bar_y).of_size(bar_width, 10),
                color,
            );
        }

        let pct_label = format!("{:.0}%", stat.percentage);
        draw_text(
            &mut img,
            &font,
            22.0,
            (bars_x + bar_max_width + 16) as f32,
            (bar_y - 4) as f32,
            &pct_label,
            colors::text_muted(),
        );
    }

    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .context("failed to encode PNG")?;
    Ok(buf.into_inner())
}

fn draw_pie_chart(img: &mut RgbaImage, cx: i32, cy: i32, radius: i32, stats: &[LanguageStat]) {
    let mut start = -PI / 2.0;
    for stat in stats {
        let sweep = (stat.percentage as f32 / 100.0) * 2.0 * PI;
        if sweep <= 0.0 {
            continue;
        }
        let end = start + sweep;
        let color = colors::language_color(&stat.name);
        fill_pie_slice(img, cx, cy, radius, start, end, color);
        start = end;
    }
}

fn fill_pie_slice(
    img: &mut RgbaImage,
    cx: i32,
    cy: i32,
    radius: i32,
    start: f32,
    end: f32,
    color: Rgba<u8>,
) {
    let r2 = (radius * radius) as i32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let dist2 = dx * dx + dy * dy;
            if dist2 > r2 {
                continue;
            }
            let angle = (dy as f32).atan2(dx as f32);
            if angle_in_slice(angle, start, end) {
                let x = cx + dx;
                let y = cy + dy;
                if x >= 0 && y >= 0 && x < img.width() as i32 && y < img.height() as i32 {
                    img.put_pixel(x as u32, y as u32, color);
                }
            }
        }
    }
}

fn angle_in_slice(angle: f32, start: f32, end: f32) -> bool {
    let a = normalize_angle(angle);
    let s = normalize_angle(start);
    let e = normalize_angle(end);

    if s <= e {
        a >= s && a < e
    } else {
        a >= s || a < e
    }
}

fn normalize_angle(mut angle: f32) -> f32 {
    use std::f32::consts::TAU;
    angle = angle % TAU;
    if angle < 0.0 {
        angle += TAU;
    }
    angle
}

fn fill_rounded_rect(
    img: &mut RgbaImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    let width = (x1 - x0).max(0) as u32;
    let height = (y1 - y0).max(0) as u32;
    draw_filled_rect_mut(
        img,
        imageproc::rect::Rect::at(x0, y0).of_size(width, height),
        color,
    );

    let bg = colors::background();
    let r = radius;
    let r2 = r * r;
    let cx_tl = x0 + r;
    let cy_tl = y0 + r;
    let cx_tr = x1 - r;
    let cy_tr = y0 + r;
    let cx_bl = x0 + r;
    let cy_bl = y1 - r;
    let cx_br = x1 - r;
    let cy_br = y1 - r;

    for y in y0..y0 + r {
        for x in x0..x0 + r {
            let dx = x - cx_tl;
            let dy = y - cy_tl;
            if dx * dx + dy * dy > r2 {
                img.put_pixel(x as u32, y as u32, bg);
            }
        }
    }

    for y in y0..y0 + r {
        for x in x1 - r..x1 {
            let dx = x - cx_tr;
            let dy = y - cy_tr;
            if dx * dx + dy * dy > r2 {
                img.put_pixel(x as u32, y as u32, bg);
            }
        }
    }

    for y in y1 - r..y1 {
        for x in x0..x0 + r {
            let dx = x - cx_bl;
            let dy = y - cy_bl;
            if dx * dx + dy * dy > r2 {
                img.put_pixel(x as u32, y as u32, bg);
            }
        }
    }

    for y in y1 - r..y1 {
        for x in x1 - r..x1 {
            let dx = x - cx_br;
            let dy = y - cy_br;
            if dx * dx + dy * dy > r2 {
                img.put_pixel(x as u32, y as u32, bg);
            }
        }
    }
}

fn draw_text(
    img: &mut RgbaImage,
    font: &FontRef<'_>,
    size: f32,
    x: f32,
    y: f32,
    text: &str,
    color: Rgba<u8>,
) {
    draw_text_mut(
        img,
        color,
        x.round() as i32,
        y.round() as i32,
        PxScale::from(size),
        font,
        text,
    );
}
