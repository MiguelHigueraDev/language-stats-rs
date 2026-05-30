use image::Rgba;

const BG: &str = "#0d1117";
const CARD: &str = "#161b22";
const TEXT_PRIMARY: &str = "#f0f6fc";
const TEXT_MUTED: &str = "#8b949e";
const FALLBACK: &str = "#6e7681";

/// GitHub Linguist colors (https://github.com/github-linguist/linguist) where defined.
static LANGUAGE_COLORS: &[(&str, &str)] = &[
    ("AIDL", "#34EB6B"),
    ("ASP.NET", "#9400ff"),
    ("Astro", "#ff5a03"),
    ("Batchfile", "#C1F12E"),
    ("Blade", "#f7523f"),
    ("C", "#555555"),
    ("C#", "#178600"),
    ("C++", "#f34b7d"),
    ("CMake", "#DA3434"),
    ("CSS", "#663399"),
    ("DIGITAL Command Language", "#5c6bc0"),
    ("Dart", "#00B4AB"),
    ("Dockerfile", "#384d54"),
    ("Elixir", "#6e4a7e"),
    ("GLSL", "#5686a5"),
    ("Go", "#00ADD8"),
    ("HLSL", "#aace60"),
    ("HTML", "#e34c26"),
    ("ISPC", "#2D68B1"),
    ("Java", "#b07219"),
    ("JavaScript", "#f1e05a"),
    ("Jinja", "#a52a22"),
    ("Jupyter Notebook", "#DA5B0B"),
    ("Kotlin", "#A97BFF"),
    ("Lua", "#000080"),
    ("Makefile", "#427819"),
    ("MDX", "#fcb32c"),
    ("NASL", "#26a69a"),
    ("Objective-C", "#438eff"),
    ("Objective-C++", "#6866fb"),
    ("Other", "#6e7681"),
    ("PHP", "#4F5D95"),
    ("PLSQL", "#dad8d8"),
    ("PLpgSQL", "#336790"),
    ("Perl", "#0298c3"),
    ("PowerShell", "#012456"),
    ("Python", "#3572A5"),
    ("R", "#198CE7"),
    ("Raku", "#0000fb"),
    ("Rust", "#dea584"),
    ("SCSS", "#c6538c"),
    ("Shell", "#89e051"),
    ("Svelte", "#ff3e00"),
    ("Swift", "#F05138"),
    ("TSQL", "#e38c00"),
    ("TypeScript", "#3178c6"),
    ("Vim Snippet", "#199f4b"),
    ("Vue", "#41b883"),
    ("Xmake", "#22a079"),
];

pub fn background() -> Rgba<u8> {
    hex_color(BG)
}

pub fn card() -> Rgba<u8> {
    hex_color(CARD)
}

pub fn text_primary() -> Rgba<u8> {
    hex_color(TEXT_PRIMARY)
}

pub fn text_muted() -> Rgba<u8> {
    hex_color(TEXT_MUTED)
}

pub fn language_color(name: &str) -> Rgba<u8> {
    language_hex(name)
        .map(hex_color)
        .unwrap_or_else(|| hex_color(FALLBACK))
}

fn language_hex(name: &str) -> Option<&'static str> {
    LANGUAGE_COLORS
        .iter()
        .find(|(lang, _)| *lang == name)
        .map(|(_, hex)| *hex)
}

fn hex_color(hex: &str) -> Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    let parse = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
    match hex.len() {
        6 => Rgba([parse(&hex[0..2]), parse(&hex[2..4]), parse(&hex[4..6]), 255]),
        _ => Rgba([0, 0, 0, 255]),
    }
}
