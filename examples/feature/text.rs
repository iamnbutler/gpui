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
    App, Application, Colors, Context, FontStyle, FontWeight, Hsla, Render, Rgba, StyledText,
    Window, WindowBounds, WindowOptions, centered_bounds, div, prelude::*, px, relative, rems, size,
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
                    ))
                    .child(section(
                        &colors,
                        "Box Model: Single Line Sizes",
                        box_model_single_line_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Box Model: Line Height Variants",
                        box_model_line_height_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Box Model: Multi-Line Wrapping",
                        box_model_multiline_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Box Model: Stacked Lines",
                        box_model_stacked_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Box Model: Rems Line Height",
                        box_model_rems_section(&colors),
                    )),
            )
    }
}

// Debug colors
const DEBUG_CONTAINER: Rgba = Rgba {
    r: 0.0,
    g: 0.4,
    b: 1.0,
    a: 0.15,
};
const DEBUG_BORDER: Rgba = Rgba {
    r: 0.0,
    g: 0.4,
    b: 1.0,
    a: 0.5,
};
const DEBUG_BASELINE: Rgba = Rgba {
    r: 1.0,
    g: 0.2,
    b: 0.2,
    a: 0.6,
};
const DEBUG_TEXT_BG: Rgba = Rgba {
    r: 0.0,
    g: 0.8,
    b: 0.3,
    a: 0.15,
};

/// A labeled text sample with debug box visualization.
/// Shows the container box (blue) and text background (green).
fn debug_text_row(
    label: &str,
    text: &str,
    font_size: f32,
    line_height: Option<f32>,
    colors: &Colors,
) -> impl IntoElement {
    let mut text_div = div()
        .bg(DEBUG_TEXT_BG)
        .text_size(px(font_size))
        .text_color(colors.text)
        .child(text.to_string());

    if let Some(lh) = line_height {
        text_div = text_div.line_height(relative(lh));
    }

    div()
        .flex()
        .items_start()
        .gap_3()
        .child(
            div()
                .w(px(120.))
                .flex_shrink_0()
                .text_xs()
                .text_color(colors.text_muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .bg(DEBUG_CONTAINER)
                .border_1()
                .border_color(DEBUG_BORDER)
                .child(text_div),
        )
}

// Section 9: Box model debug — single line
fn box_model_single_line_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("Blue = container, Green = text background. All default line height (phi ≈ 1.618)."),
        )
        .child(debug_text_row("12px", "Hamburgefons", 12., None, colors))
        .child(debug_text_row("14px", "Hamburgefons", 14., None, colors))
        .child(debug_text_row("16px", "Hamburgefons", 16., None, colors))
        .child(debug_text_row("20px", "Hamburgefons", 20., None, colors))
        .child(debug_text_row("24px", "Hamburgefons", 24., None, colors))
        .child(debug_text_row("32px", "Hamburgefons", 32., None, colors))
        .child(debug_text_row("48px", "Hamburgefons", 48., None, colors))
}

// Section 10: Box model debug — line height variants
fn box_model_line_height_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("All at 16px font size with varying line heights."),
        )
        .child(debug_text_row(
            "line-height: 1.0",
            "The quick brown fox",
            16.,
            Some(1.0),
            colors,
        ))
        .child(debug_text_row(
            "line-height: 1.2",
            "The quick brown fox",
            16.,
            Some(1.2),
            colors,
        ))
        .child(debug_text_row(
            "line-height: 1.5",
            "The quick brown fox",
            16.,
            Some(1.5),
            colors,
        ))
        .child(debug_text_row(
            "line-height: 1.618 (phi)",
            "The quick brown fox",
            16.,
            Some(1.618),
            colors,
        ))
        .child(debug_text_row(
            "line-height: 2.0",
            "The quick brown fox",
            16.,
            Some(2.0),
            colors,
        ))
        .child(debug_text_row(
            "line-height: 3.0",
            "The quick brown fox",
            16.,
            Some(3.0),
            colors,
        ))
}

// Section 11: Box model debug — multi-line
fn box_model_multiline_section(colors: &Colors) -> impl IntoElement {
    let long_text = "This is a longer paragraph that should wrap to multiple lines. \
        We want to verify that line spacing is consistent and that the container \
        correctly encompasses all lines of text.";

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("Multi-line text in constrained-width containers (300px)."),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(120.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child("14px / default lh"),
                )
                .child(
                    div()
                        .w(px(300.))
                        .bg(DEBUG_CONTAINER)
                        .border_1()
                        .border_color(DEBUG_BORDER)
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(14.))
                                .text_color(colors.text)
                                .child(long_text),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(120.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child("14px / lh 1.2"),
                )
                .child(
                    div()
                        .w(px(300.))
                        .bg(DEBUG_CONTAINER)
                        .border_1()
                        .border_color(DEBUG_BORDER)
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(14.))
                                .line_height(relative(1.2))
                                .text_color(colors.text)
                                .child(long_text),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(120.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child("14px / lh 2.0"),
                )
                .child(
                    div()
                        .w(px(300.))
                        .bg(DEBUG_CONTAINER)
                        .border_1()
                        .border_color(DEBUG_BORDER)
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(14.))
                                .line_height(relative(2.0))
                                .text_color(colors.text)
                                .child(long_text),
                        ),
                ),
        )
}

// Section 12: Box model debug — stacked lines comparison
fn box_model_stacked_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("Stacked single lines — check consistent spacing between rows."),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(120.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child("16px / lh 1.5"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .bg(DEBUG_CONTAINER)
                        .border_1()
                        .border_color(DEBUG_BORDER)
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(16.))
                                .line_height(relative(1.5))
                                .text_color(colors.text)
                                .child("First line"),
                        )
                        .child(
                            div()
                                .bg(DEBUG_BASELINE)
                                .h(px(1.))
                        )
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(16.))
                                .line_height(relative(1.5))
                                .text_color(colors.text)
                                .child("Second line"),
                        )
                        .child(
                            div()
                                .bg(DEBUG_BASELINE)
                                .h(px(1.))
                        )
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(16.))
                                .line_height(relative(1.5))
                                .text_color(colors.text)
                                .child("Third line"),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(120.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child("Mixed sizes"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .bg(DEBUG_CONTAINER)
                        .border_1()
                        .border_color(DEBUG_BORDER)
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(12.))
                                .line_height(relative(1.5))
                                .text_color(colors.text)
                                .child("12px: Small text line"),
                        )
                        .child(
                            div()
                                .bg(DEBUG_BASELINE)
                                .h(px(1.))
                        )
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(24.))
                                .line_height(relative(1.5))
                                .text_color(colors.text)
                                .child("24px: Large text"),
                        )
                        .child(
                            div()
                                .bg(DEBUG_BASELINE)
                                .h(px(1.))
                        )
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(12.))
                                .line_height(relative(1.5))
                                .text_color(colors.text)
                                .child("12px: Small text line"),
                        ),
                ),
        )
}

// Section 13: Box model debug — rems line height
fn box_model_rems_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .child("Line height set in rems (1rem = 16px default)."),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(120.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child("14px / lh 1.25rem"),
                )
                .child(
                    div()
                        .bg(DEBUG_CONTAINER)
                        .border_1()
                        .border_color(DEBUG_BORDER)
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(14.))
                                .line_height(rems(1.25))
                                .text_color(colors.text)
                                .child("The quick brown fox"),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(120.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child("14px / lh 1.5rem"),
                )
                .child(
                    div()
                        .bg(DEBUG_CONTAINER)
                        .border_1()
                        .border_color(DEBUG_BORDER)
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(14.))
                                .line_height(rems(1.5))
                                .text_color(colors.text)
                                .child("The quick brown fox"),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(120.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child("14px / lh 2.0rem"),
                )
                .child(
                    div()
                        .bg(DEBUG_CONTAINER)
                        .border_1()
                        .border_color(DEBUG_BORDER)
                        .child(
                            div()
                                .bg(DEBUG_TEXT_BG)
                                .text_size(px(14.))
                                .line_height(rems(2.0))
                                .text_color(colors.text)
                                .child("The quick brown fox"),
                        ),
                ),
        )
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
        let bounds = centered_bounds(None, size(px(850.), px(1200.)), cx);
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
