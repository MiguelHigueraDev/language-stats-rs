use crate::chart::svg;
use crate::colors;
use crate::models::LanguageStat;
use anyhow::Result;

const WIDTH: u32 = 300;
const PADDING: i32 = 16;
const TITLE_SIZE: i32 = 16;
const LEGEND_SIZE: i32 = 12;
const BAR_HEIGHT: i32 = 8;
const BAR_RADIUS: i32 = 4;
const TITLE_GAP: i32 = 12;
const BAR_GAP: i32 = 16;
const LEGEND_ROW: i32 = 22;
const DOT_RADIUS: i32 = 6;
const COLUMN_GAP: i32 = 24;

pub fn render_minimal_language_card(stats: &[LanguageStat]) -> Result<Vec<u8>> {
    let left_count = stats.len().div_ceil(2);
    let max_rows = left_count.max(stats.len() - left_count);
    let legend_height = max_rows as i32 * LEGEND_ROW;
    let height = PADDING
        + TITLE_SIZE
        + TITLE_GAP
        + BAR_HEIGHT
        + BAR_GAP
        + legend_height
        + PADDING;

    let bar_x = PADDING;
    let bar_y = PADDING + TITLE_SIZE + TITLE_GAP;
    let bar_width = WIDTH as i32 - PADDING * 2;
    let column_width = (bar_width - COLUMN_GAP) / 2;
    let legend_top = bar_y + BAR_HEIGHT + BAR_GAP;

    let clip_defs = format!(
        r#"<clipPath id="minimal-bar-clip">
<rect x="{bar_x}" y="{bar_y}" width="{bar_width}" height="{BAR_HEIGHT}" rx="{BAR_RADIUS}" ry="{BAR_RADIUS}"/>
</clipPath>
"#
    );

    let mut svg = String::with_capacity(4096);
    svg.push_str(&svg::svg_open(WIDTH, height as u32, &clip_defs));
    svg.push_str(&format!(
        r#"<text x="{bar_x}" y="{title_y}" font-family="Roboto" font-size="{TITLE_SIZE}" font-weight="600" fill="{title_fill}">Most Used Languages</text>
<rect x="{bar_x}" y="{bar_y}" width="{bar_width}" height="{BAR_HEIGHT}" rx="{BAR_RADIUS}" fill="{track}"/>
<g clip-path="url(#minimal-bar-clip)">
"#,
        title_y = PADDING + TITLE_SIZE - 2,
        title_fill = colors::text_primary(),
        track = colors::bar_track(),
    ));

    let mut segment_x = bar_x as f64;
    for stat in stats {
        let segment_width = (stat.percentage / 100.0) * bar_width as f64;
        if segment_width >= 0.5 {
            svg.push_str(&format!(
                r#"<rect x="{x:.2}" y="{bar_y}" width="{w:.2}" height="{BAR_HEIGHT}" fill="{color}"/>
"#,
                x = segment_x,
                w = segment_width,
                color = colors::language_color(&stat.name),
            ));
        }
        segment_x += segment_width;
    }

    svg.push_str("</g>\n");

    for (i, stat) in stats.iter().enumerate() {
        let column = if i < left_count { 0 } else { 1 };
        let row = if column == 0 { i } else { i - left_count };
        let x = bar_x + column * (column_width + COLUMN_GAP);
        let y = legend_top + row as i32 * LEGEND_ROW;
        let dot_cy = y + LEGEND_ROW / 2;
        let label = format!("{} {:.2}%", stat.name, stat.percentage);

        svg.push_str(&format!(
            r#"<circle cx="{dot_cx}" cy="{dot_cy}" r="{DOT_RADIUS}" fill="{color}"/>
<text x="{text_x}" y="{text_y}" font-family="Roboto" font-size="{LEGEND_SIZE}" fill="{text_fill}">{label}</text>
"#,
            dot_cx = x + DOT_RADIUS,
            text_x = x + DOT_RADIUS * 2 + 8,
            text_y = dot_cy + 4,
            color = colors::language_color(&stat.name),
            text_fill = colors::text_muted(),
            label = svg::escape_xml(&label),
        ));
    }

    svg.push_str("</svg>");
    Ok(svg.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LanguageStat;

    fn sample_stats() -> Vec<LanguageStat> {
        vec![
            LanguageStat {
                name: "TypeScript".into(),
                bytes: 4659,
                percentage: 46.59,
            },
            LanguageStat {
                name: "JavaScript".into(),
                bytes: 3596,
                percentage: 35.96,
            },
            LanguageStat {
                name: "Java".into(),
                bytes: 1082,
                percentage: 10.82,
            },
            LanguageStat {
                name: "Vue".into(),
                bytes: 420,
                percentage: 4.20,
            },
            LanguageStat {
                name: "Rust".into(),
                bytes: 224,
                percentage: 2.24,
            },
            LanguageStat {
                name: "Elixir".into(),
                bytes: 20,
                percentage: 0.20,
            },
        ]
    }

    #[test]
    fn minimal_card_contains_title_bar_and_legend() {
        let svg = render_minimal_language_card(&sample_stats()).unwrap();
        let text = String::from_utf8(svg).unwrap();
        assert!(text.contains("Most Used Languages"));
        assert!(text.contains("minimal-bar-clip"));
        assert!(text.contains("TypeScript 46.59%"));
        assert!(text.contains("Elixir 0.20%"));
    }
}
