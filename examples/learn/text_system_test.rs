//! Text System Test
//!
//! A comprehensive visual test for the Parley text system migration.
//! Exercises font resolution, shaping, metrics, and rendering across:
//!
//! 1. Multiple font families and sizes
//! 2. All font weights (100-900)
//! 3. Italic and oblique styles
//! 4. Unicode scripts (Latin, Greek, CJK, Arabic, Devanagari)
//! 5. Emoji (color and monochrome)
//! 6. Mixed-script runs
//! 7. Edge cases (empty string, BOM, zero-width chars, single char)
//! 8. Font fallback verification

#[path = "../prelude.rs"]
mod example_prelude;

use example_prelude::init_example;
use gpui::{
    App, Application, Bounds, Colors, Context, FontStyle, FontWeight, Hsla, Render, StyledText,
    Window, WindowBounds, WindowOptions, div, prelude::*, px, size,
};

// Section 1: Font sizes from tiny to large
fn font_sizes_section(colors: &Colors) -> impl IntoElement {
    let sizes: &[(f32, &str)] = &[
        (8., "8px"),
        (10., "10px"),
        (12., "12px"),
        (14., "14px"),
        (16., "16px"),
        (20., "20px"),
        (24., "24px"),
        (32., "32px"),
        (48., "48px"),
        (72., "72px"),
    ];

    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(sizes.iter().map(|(sz, label)| {
            div()
                .flex()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .w(px(40.))
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .text_size(px(*sz))
                        .text_color(colors.text)
                        .child("Hamburgefons"),
                )
        }))
}

// Section 2: All font weights
fn font_weights_section(colors: &Colors) -> impl IntoElement {
    let weights: &[(FontWeight, &str)] = &[
        (FontWeight::THIN, "100 Thin"),
        (FontWeight::EXTRA_LIGHT, "200 ExtraLight"),
        (FontWeight::LIGHT, "300 Light"),
        (FontWeight::NORMAL, "400 Normal"),
        (FontWeight::MEDIUM, "500 Medium"),
        (FontWeight::SEMIBOLD, "600 Semibold"),
        (FontWeight::BOLD, "700 Bold"),
        (FontWeight::EXTRA_BOLD, "800 ExtraBold"),
        (FontWeight::BLACK, "900 Black"),
    ];

    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(weights.iter().map(|(weight, label)| {
            div()
                .text_base()
                .text_color(colors.text)
                .font_weight(*weight)
                .child(format!("{label} — The quick brown fox"))
        }))
}

// Section 3: Font styles
fn font_styles_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .child("Normal: The quick brown fox jumps over the lazy dog"),
        )
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .italic()
                .child("Italic: The quick brown fox jumps over the lazy dog"),
        )
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .font_weight(FontWeight::BOLD)
                .child("Bold: The quick brown fox jumps over the lazy dog"),
        )
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .font_weight(FontWeight::BOLD)
                .italic()
                .child("Bold Italic: The quick brown fox jumps over the lazy dog"),
        )
}

// Section 4: Unicode scripts
fn unicode_scripts_section(colors: &Colors) -> impl IntoElement {
    let scripts: &[(&str, &str)] = &[
        ("Latin", "ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz"),
        ("Numbers", "0123456789 ½ ¼ ¾ ⅓ ⅔"),
        ("Latin Extended", "àáâãäå æçèéêë ìíîïðñ òóôõö ùúûüý"),
        ("Greek", "αβγδεζηθικλμνξοπρστυφχψω ΑΒΓΔΕΖΗΘ"),
        ("Cyrillic", "абвгдежзийклмнопрстуфхцчшщъыьэюя"),
        ("CJK", "你好世界 日本語テスト 한국어"),
        ("Arabic", "مرحبا بالعالم"),
        ("Devanagari", "नमस्ते दुनिया"),
        ("Thai", "สวัสดีชาวโลก"),
        ("Math", "∀∃∈∉∅ ∪∩⊂⊃ ∑∏∫ ≤≥≠≈ ∞√±"),
        ("Box Drawing", "┌─┬─┐│ │ ││ │ │├─┼─┤│ │ │└─┴─┘"),
        ("Symbols", "→←↑↓ •★♠♥♦♣ ©®™ §¶†‡"),
    ];

    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(scripts.iter().map(|(name, text)| {
            div()
                .flex()
                .gap_2()
                .child(
                    div()
                        .w(px(100.))
                        .text_xs()
                        .text_color(colors.text_muted)
                        .flex_shrink_0()
                        .child(name.to_string()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.text)
                        .child(text.to_string()),
                )
        }))
}

// Section 5: Emoji
fn emoji_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("Color emoji"),
        )
        .child(
            div()
                .text_xl()
                .text_color(colors.text)
                .child("😀 😃 😄 😁 😆 🤣 😂 🙂 😊 😇 🥰 😍"),
        )
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("ZWJ sequences"),
        )
        .child(
            div()
                .text_xl()
                .text_color(colors.text)
                .child("👨‍👩‍👧‍👦 👩‍💻 🏳️‍🌈 👨‍🍳 🧑‍🚀"),
        )
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("Skin tone variants"),
        )
        .child(
            div()
                .text_xl()
                .text_color(colors.text)
                .child("👋👋🏻👋🏼👋🏽👋🏾👋🏿"),
        )
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("Flags"),
        )
        .child(
            div()
                .text_xl()
                .text_color(colors.text)
                .child("🇺🇸 🇬🇧 🇫🇷 🇩🇪 🇯🇵 🇰🇷 🇨🇳 🇧🇷 🇮🇳"),
        )
}

// Section 6: Mixed-script runs
fn mixed_scripts_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .child("English mixed with 中文 characters and back to English"),
        )
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .child("Emoji 🎉 mid-sentence with 日本語 and more 🚀 emoji"),
        )
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .child("Numbers 42 mixed with Greek π≈3.14159 and symbols →←"),
        )
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .child("Combining chars: café résumé naïve über año"),
        )
}

// Section 7: Edge cases
fn edge_cases_section(colors: &Colors) -> impl IntoElement {
    let surface = colors.surface;
    let border = colors.border;

    let cases: &[(&str, &str)] = &[
        ("Empty string", ""),
        ("Single char", "A"),
        ("BOM prefix", "\u{feff}Hello with BOM"),
        ("Zero-width", "a\u{200b}b\u{200c}c\u{200d}d"),
        ("Tabs", "col1\tcol2\tcol3"),
        ("Soft hyphen", "super\u{00ad}cali\u{00ad}fragil\u{00ad}istic"),
        ("Long word", "Supercalifragilisticexpialidocious"),
        ("RTL embed", "Hello \u{202b}مرحبا\u{202c} World"),
    ];

    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(cases.iter().map(|(label, text)| {
            div()
                .flex()
                .gap_2()
                .child(
                    div()
                        .w(px(100.))
                        .text_xs()
                        .text_color(colors.text_muted)
                        .flex_shrink_0()
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .px_2()
                        .py_0p5()
                        .bg(surface)
                        .border_1()
                        .border_color(border)
                        .rounded_sm()
                        .text_sm()
                        .text_color(colors.text)
                        .min_w(px(40.))
                        .child(text.to_string()),
                )
        }))
}

// Section 8: Styled text with inline variations
fn styled_text_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div().text_lg().text_color(colors.text).child(
                StyledText::new("Normal Bold Italic Light Semibold Black").with_highlights([
                    (7..11, FontWeight::BOLD.into()),
                    (12..18, FontStyle::Italic.into()),
                    (19..24, FontWeight::LIGHT.into()),
                    (25..33, FontWeight::SEMIBOLD.into()),
                    (34..39, FontWeight::BLACK.into()),
                ]),
            ),
        )
        .child(
            div()
                .text_base()
                .text_color(colors.text)
                .child(
                    StyledText::new("Mixed weights in a single line demonstrate shaping across font runs")
                        .with_highlights([
                            (0..5, FontWeight::BOLD.into()),
                            (6..13, FontWeight::LIGHT.into()),
                            (19..25, FontWeight::SEMIBOLD.into()),
                        ]),
                ),
        )
}

// Main view
struct TextSystemTest;

impl Render for TextSystemTest {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);

        div()
            .id("main")
            .size_full()
            .p_4()
            .bg(colors.background)
            .overflow_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .max_w(px(800.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(colors.text)
                                    .child("Text System Test"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text_muted)
                                    .child("Comprehensive visual test for the Parley text system"),
                            ),
                    )
                    .child(section(
                        &colors,
                        "Font Sizes (8px → 72px)",
                        font_sizes_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Font Weights (100 → 900)",
                        font_weights_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Font Styles",
                        font_styles_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Unicode Scripts",
                        unicode_scripts_section(&colors),
                    ))
                    .child(section(&colors, "Emoji", emoji_section(&colors)))
                    .child(section(
                        &colors,
                        "Mixed Scripts",
                        mixed_scripts_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Styled Text (Inline Weight Changes)",
                        styled_text_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Edge Cases",
                        edge_cases_section(&colors),
                    )),
            )
    }
}

fn section(colors: &Colors, title: &'static str, content: impl IntoElement) -> impl IntoElement {
    let surface: Hsla = colors.surface.into();

    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .bg(surface.opacity(0.5))
        .rounded_lg()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.text)
                .child(title),
        )
        .child(content)
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(850.), px(1200.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| TextSystemTest),
        )
        .expect("Failed to open window");

        init_example(cx, "Text System Test");
    });
}
