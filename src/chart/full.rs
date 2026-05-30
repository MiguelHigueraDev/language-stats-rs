use crate::chart::svg;
use crate::colors;
use crate::models::LanguageStat;
use anyhow::Result;
use std::f32::consts::PI;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 630;
const CARD_RADIUS: i32 = 24;
const CARD_MARGIN: i32 = 28;
const INNER_PADDING: i32 = 36;
const PIE_RADIUS: i32 = 165;
const PIE_INNER_RADIUS: i32 = 98;
const BARS_X_OFFSET: i32 = 520;
const LEGEND_WIDTH: i32 = 148;
const PCT_COLUMN_WIDTH: i32 = 58;
const BAR_PCT_GAP: i32 = 16;
const ROW_HEIGHT: i32 = 52;
const BAR_HEIGHT: i32 = 8;
const BAR_RADIUS: i32 = 4;
const SWATCH_SIZE: i32 = 12;
const HEADER_GAP: i32 = 12;
const SLICE_GAP: f32 = 0.012;

pub fn render_language_card(
    username: &str,
    stats: &[LanguageStat],
    show_username: bool,
) -> Result<Vec<u8>> {
    let card_x0 = CARD_MARGIN;
    let card_y0 = CARD_MARGIN;
    let card_x1 = (WIDTH as i32) - CARD_MARGIN;
    let card_y1 = (HEIGHT as i32) - CARD_MARGIN;
    let card_width = card_x1 - card_x0;
    let card_height = card_y1 - card_y0;

    let mut svg = String::with_capacity(svg::roboto_capacity_hint() + 8192);
    svg.push_str(&svg::svg_open(WIDTH, HEIGHT, ""));
    svg.push_str(&format!(
        r#"<rect x="{card_x0}" y="{card_y0}" width="{card_width}" height="{card_height}" rx="{CARD_RADIUS}" ry="{CARD_RADIUS}" fill="none" stroke="{border}" stroke-width="1"/>
"#,
        border = colors::card_border(),
    ));

    let mut header_bottom = card_y0 + INNER_PADDING;

    if show_username {
        let username_y = header_bottom + 34;
        svg.push_str(&format!(
            r#"<text x="{x}" y="{username_y}" font-family="Roboto" font-size="32" fill="{fill}">{text}</text>
"#,
            x = card_x0 + INNER_PADDING,
            fill = colors::text_primary(),
            text = svg::escape_xml(&format!("@{username}")),
        ));
        header_bottom += 34 + HEADER_GAP;
    }

    let subtitle_y = header_bottom + 22;
    svg.push_str(&format!(
        r#"<text x="{x}" y="{subtitle_y}" font-family="Roboto" font-size="18" letter-spacing="0.3" fill="{fill}">Repository languages</text>
"#,
        x = card_x0 + INNER_PADDING,
        fill = colors::text_muted(),
    ));

    let content_area_top = subtitle_y + 28;
    let content_area_bottom = card_y1 - INNER_PADDING;
    let content_area_height = content_area_bottom - content_area_top;

    let bar_block_height = stats.len() as i32 * ROW_HEIGHT;
    let pie_block_height = PIE_RADIUS * 2;
    let content_block_height = pie_block_height.max(bar_block_height);
    let content_top =
        content_area_top + ((content_area_height - content_block_height).max(0) / 2);

    let pie_cx = card_x0 + INNER_PADDING + PIE_RADIUS + 16;
    let pie_cy = content_top + content_block_height / 2;
    let bars_x = card_x0 + BARS_X_OFFSET;
    let bars_top = content_top + (content_block_height - bar_block_height) / 2;
    let divider_x = bars_x - 28;

    svg.push_str(&format!(
        r#"<line x1="{divider_x}" y1="{y0}" x2="{divider_x}" y2="{y1}" stroke="{stroke}" stroke-width="1"/>
"#,
        y0 = content_top + 8,
        y1 = content_top + content_block_height - 8,
        stroke = colors::divider(),
    ));

    svg.push_str(&donut_chart_svg(pie_cx, pie_cy, PIE_RADIUS, PIE_INNER_RADIUS, stats));

    let bar_x = bars_x + LEGEND_WIDTH + 20;
    let pct_x = card_x1 - INNER_PADDING;
    let bar_area_width = pct_x - PCT_COLUMN_WIDTH - BAR_PCT_GAP - bar_x;

    for (i, stat) in stats.iter().enumerate() {
        let y = bars_top + (i as i32) * ROW_HEIGHT;
        let color = colors::language_color(&stat.name);
        let row_center = y + ROW_HEIGHT / 2;

        svg.push_str(&format!(
            r#"<rect x="{x}" y="{sy}" width="{SWATCH_SIZE}" height="{SWATCH_SIZE}" rx="3" fill="{color}"/>
<text x="{nx}" y="{ny}" font-family="Roboto" font-size="20" fill="{text}">{name}</text>
"#,
            x = bars_x,
            sy = row_center - SWATCH_SIZE / 2,
            nx = bars_x + SWATCH_SIZE + 12,
            ny = row_center + 7,
            text = colors::text_primary(),
            name = svg::escape_xml(&stat.name),
        ));

        let bar_y = row_center - BAR_HEIGHT / 2;
        let bar_width =
            ((stat.percentage / 100.0) * bar_area_width as f64).round().max(0.0) as i32;
        svg.push_str(&format!(
            r#"<rect x="{bar_x}" y="{bar_y}" width="{bar_area_width}" height="{BAR_HEIGHT}" rx="{BAR_RADIUS}" fill="{track}"/>
"#,
            track = colors::bar_track(),
        ));
        if bar_width > 0 {
            svg.push_str(&format!(
                r#"<rect x="{bar_x}" y="{bar_y}" width="{bar_width}" height="{BAR_HEIGHT}" rx="{BAR_RADIUS}" fill="{color}"/>
"#,
            ));
        }

        let pct_label = format!("{:.1}%", stat.percentage);
        svg.push_str(&format!(
            r#"<text x="{pct_x}" y="{py}" font-family="Roboto" font-size="18" text-anchor="end" fill="{fill}">{pct}</text>
"#,
            py = row_center + 6,
            fill = colors::text_muted(),
            pct = svg::escape_xml(&pct_label),
        ));
    }

    svg.push_str("</svg>");
    Ok(svg.into_bytes())
}

fn donut_chart_svg(
    cx: i32,
    cy: i32,
    outer: i32,
    inner: i32,
    stats: &[LanguageStat],
) -> String {
    let mut out = String::new();
    let mut start = -PI / 2.0;

    for stat in stats {
        let sweep = (stat.percentage as f32 / 100.0) * 2.0 * PI;
        if sweep <= 0.0 {
            continue;
        }
        let gap = SLICE_GAP.min(sweep / 4.0);
        let slice_start = start + gap;
        let slice_end = start + sweep - gap;
        if slice_end > slice_start {
            let color = colors::language_color(&stat.name);
            let sweep = slice_end - slice_start;
            if sweep >= 2.0 * PI - 0.02 {
                out.push_str(&format!(
                    r#"<circle cx="{cx}" cy="{cy}" r="{outer}" fill="{color}"/>
<circle cx="{cx}" cy="{cy}" r="{inner}" fill="none"/>
"#
                ));
            } else {
                out.push_str(&format!(
                    r#"<path d="{path}" fill="{color}" stroke="none"/>
"#,
                    path = donut_slice_path(cx, cy, outer, inner, slice_start, slice_end),
                ));
            }
        }
        start += sweep;
    }

    out
}

fn donut_slice_path(
    cx: i32,
    cy: i32,
    outer: i32,
    inner: i32,
    start: f32,
    end: f32,
) -> String {
    let sweep = end - start;
    let x1o = cx as f32 + outer as f32 * start.cos();
    let y1o = cy as f32 + outer as f32 * start.sin();
    let x2o = cx as f32 + outer as f32 * end.cos();
    let y2o = cy as f32 + outer as f32 * end.sin();
    let x1i = cx as f32 + inner as f32 * end.cos();
    let y1i = cy as f32 + inner as f32 * end.sin();
    let x2i = cx as f32 + inner as f32 * start.cos();
    let y2i = cy as f32 + inner as f32 * start.sin();
    let large_arc = if sweep > PI { 1 } else { 0 };

    format!(
        "M {x1o:.2} {y1o:.2} A {outer} {outer} 0 {large_arc} 1 {x2o:.2} {y2o:.2} L {x1i:.2} {y1i:.2} A {inner} {inner} 0 {large_arc} 0 {x2i:.2} {y2i:.2} Z",
    )
}
