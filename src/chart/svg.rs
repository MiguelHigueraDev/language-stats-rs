use base64::{Engine as _, engine::general_purpose::STANDARD};

const ROBOTO_TTF: &[u8] = include_bytes!("../../assets/Roboto-Regular.ttf");

pub fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn svg_open(width: u32, height: u32, extra_defs: &str) -> String {
    let font_base64 = STANDARD.encode(ROBOTO_TTF);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
<defs>
<style>
@font-face {{
  font-family: 'Roboto';
  src: url('data:font/ttf;base64,{font_base64}') format('truetype');
}}
</style>
{extra_defs}</defs>
"##
    )
}

pub fn roboto_capacity_hint() -> usize {
    ROBOTO_TTF.len() * 4 / 3
}
