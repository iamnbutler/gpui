use crate::{
    Bounds, DevicePixels, Font, FontId, FontMetrics, FontRun, FontStyle, GlyphId, LineLayout,
    Pixels, PlatformTextSystem, RenderGlyphParams, Result, SUBPIXEL_VARIANTS_X, ShapedGlyph,
    ShapedRun, Size, point, px, size, swap_rgba_pa_to_bgra,
};
use anyhow::anyhow;
use collections::HashMap;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use parley::{FontContext, LayoutContext};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size as SkriSize},
    raw::types::GlyphId as SkriGlyphId,
};
use std::{borrow::Cow, sync::Arc};

const INTER_FONT: &[u8] = include_bytes!("../../assets/fonts/InterVariable.ttf");

pub(crate) struct ParleyTextSystem(RwLock<ParleyTextSystemState>);

struct ParleyTextSystemState {
    font_context: FontContext,
    layout_context: LayoutContext<()>,
    fonts: Vec<LoadedFont>,
    font_selections: HashMap<Font, FontId>,
}

struct LoadedFont {
    font_data: Arc<Vec<u8>>,
    font_index: u32,
    is_emoji: bool,
    family_name: String,
}

impl LoadedFont {
    fn font_ref(&self) -> Option<skrifa::FontRef<'_>> {
        skrifa::FontRef::from_index(self.font_data.as_slice(), self.font_index).ok()
    }
}

impl ParleyTextSystem {
    pub(crate) fn new() -> Self {
        let mut font_context = FontContext::default();
        let layout_context = LayoutContext::new();

        // Register the embedded Inter font as a fallback
        font_context
            .collection
            .register_fonts(INTER_FONT.to_vec().into(), None);

        Self(RwLock::new(ParleyTextSystemState {
            font_context,
            layout_context,
            fonts: Vec::new(),
            font_selections: HashMap::default(),
        }))
    }
}

impl Default for ParleyTextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformTextSystem for ParleyTextSystem {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        let mut state = self.0.write();
        for font_data in fonts {
            let bytes = font_data.into_owned();
            state
                .font_context
                .collection
                .register_fonts(bytes.into(), None);
        }
        Ok(())
    }

    fn all_font_names(&self) -> Vec<String> {
        // family_names() requires &mut self on Collection
        let mut state = self.0.write();
        let mut names = Vec::new();
        for family in state.font_context.collection.family_names() {
            names.push(family.to_string());
        }
        names
    }

    fn font_id(&self, font: &Font) -> Result<FontId> {
        let lock = self.0.upgradable_read();
        if let Some(font_id) = lock.font_selections.get(font) {
            return Ok(*font_id);
        }

        let mut lock = RwLockUpgradableReadGuard::upgrade(lock);
        lock.resolve_font(font)
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        let state = self.0.read();
        let loaded_font = &state.fonts[font_id.0];
        let font_ref = loaded_font
            .font_ref()
            .expect("failed to create font reference");

        let metrics = font_ref.metrics(SkriSize::unscaled(), LocationRef::default());

        FontMetrics {
            units_per_em: metrics.units_per_em as u32,
            ascent: metrics.ascent,
            descent: metrics.descent,
            line_gap: metrics.leading,
            underline_position: metrics.underline.map(|d| d.offset).unwrap_or(0.0),
            underline_thickness: metrics.underline.map(|d| d.thickness).unwrap_or(0.0),
            cap_height: metrics.cap_height.unwrap_or(metrics.ascent),
            x_height: metrics.x_height.unwrap_or(metrics.ascent * 0.5),
            bounding_box: metrics
                .bounds
                .map(|b| Bounds {
                    origin: point(b.x_min, b.y_min),
                    size: size(b.x_max - b.x_min, b.y_max - b.y_min),
                })
                .unwrap_or_default(),
        }
    }

    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        let state = self.0.read();
        let loaded_font = &state.fonts[font_id.0];
        let font_ref = loaded_font
            .font_ref()
            .ok_or_else(|| anyhow!("failed to create font reference"))?;

        let glyph_metrics = font_ref.glyph_metrics(SkriSize::unscaled(), LocationRef::default());
        let skri_glyph = SkriGlyphId::new(glyph_id.0);

        let bounds = glyph_metrics.bounds(skri_glyph);
        match bounds {
            Some(rect) => Ok(Bounds {
                origin: point(rect.x_min, rect.y_min),
                size: size(rect.x_max - rect.x_min, rect.y_max - rect.y_min),
            }),
            None => Ok(Bounds::default()),
        }
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        let state = self.0.read();
        let loaded_font = &state.fonts[font_id.0];
        let font_ref = loaded_font
            .font_ref()
            .ok_or_else(|| anyhow!("failed to create font reference"))?;

        let glyph_metrics = font_ref.glyph_metrics(SkriSize::unscaled(), LocationRef::default());
        let skri_glyph = SkriGlyphId::new(glyph_id.0);

        let advance_width = glyph_metrics.advance_width(skri_glyph).unwrap_or(0.0);
        // skrifa doesn't expose per-glyph advance_height; use 0.0 for horizontal layout
        Ok(size(advance_width, 0.0))
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        let state = self.0.read();
        let loaded_font = &state.fonts[font_id.0];
        let font_ref = loaded_font.font_ref()?;

        let charmap = font_ref.charmap();
        // charmap.map() returns Option<skrifa::GlyphId>
        charmap
            .map(ch)
            .filter(|g| g.to_u32() != 0)
            .map(|g| GlyphId(g.to_u32()))
    }

    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        let state = self.0.read();
        let loaded_font = &state.fonts[params.font_id.0];
        let font_ref = loaded_font
            .font_ref()
            .ok_or_else(|| anyhow!("failed to create font reference"))?;

        let font_size = f32::from(params.font_size) * params.scale_factor;
        let glyph_metrics =
            font_ref.glyph_metrics(SkriSize::new(font_size), LocationRef::default());
        let skri_glyph = SkriGlyphId::new(params.glyph_id.0);

        let bounds = glyph_metrics.bounds(skri_glyph);
        match bounds {
            Some(rect) => {
                let x = rect.x_min.floor() as i32;
                let y = (-rect.y_max).floor() as i32;
                let width = (rect.x_max.ceil() - rect.x_min.floor()) as i32;
                let height = (rect.y_max.ceil() - rect.y_min.floor()) as i32;
                Ok(Bounds {
                    origin: point(DevicePixels(x), DevicePixels(y)),
                    size: size(DevicePixels(width), DevicePixels(height)),
                })
            }
            None => Ok(Bounds::default()),
        }
    }

    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        raster_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        if raster_bounds.size.width.0 == 0 || raster_bounds.size.height.0 == 0 {
            anyhow::bail!("glyph bounds are empty");
        }

        let state = self.0.read();
        let loaded_font = &state.fonts[params.font_id.0];

        let font_size = f32::from(params.font_size) * params.scale_factor;

        // Add an extra pixel when the subpixel variant isn't zero for anti-aliasing
        let mut bitmap_size = raster_bounds.size;
        if params.subpixel_variant.x > 0 {
            bitmap_size.width += DevicePixels(1);
        }
        if params.subpixel_variant.y > 0 {
            bitmap_size.height += DevicePixels(1);
        }

        let subpixel_shift = params
            .subpixel_variant
            .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32);

        let swash_font_ref = swash::FontRef::from_index(
            loaded_font.font_data.as_slice(),
            loaded_font.font_index as usize,
        )
        .ok_or_else(|| anyhow!("failed to create swash font ref"))?;

        let mut context = swash::scale::ScaleContext::new();
        let mut scaler = context.builder(swash_font_ref).size(font_size).build();

        let image = swash::scale::Render::new(&[if params.is_emoji {
            swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit)
        } else {
            swash::scale::Source::Outline
        }])
        .offset(swash::zeno::Vector {
            x: subpixel_shift.x / params.scale_factor,
            y: subpixel_shift.y / params.scale_factor,
        })
        .format(if params.is_emoji {
            swash::zeno::Format::CustomSubpixel([0.0, 0.0, 0.0])
        } else {
            swash::zeno::Format::Alpha
        })
        .render(&mut scaler, params.glyph_id.0 as u16);

        match image {
            Some(image) => {
                let mut bytes = image.data;

                if params.is_emoji {
                    // Convert from RGBA with premultiplied alpha to BGRA with straight alpha
                    for pixel in bytes.chunks_exact_mut(4) {
                        swap_rgba_pa_to_bgra(pixel);
                    }
                }

                let actual_size = size(
                    DevicePixels(image.placement.width as i32),
                    DevicePixels(image.placement.height as i32),
                );
                Ok((actual_size, bytes))
            }
            None => {
                // Return empty bitmap
                let byte_count = if params.is_emoji {
                    bitmap_size.width.0 as usize * 4 * bitmap_size.height.0 as usize
                } else {
                    bitmap_size.width.0 as usize * bitmap_size.height.0 as usize
                };
                Ok((bitmap_size, vec![0; byte_count]))
            }
        }
    }

    fn layout_line(&self, text: &str, font_size: Pixels, font_runs: &[FontRun]) -> LineLayout {
        if text.is_empty() || font_runs.is_empty() {
            return LineLayout {
                font_size,
                width: px(0.),
                ascent: px(0.),
                descent: px(0.),
                runs: Vec::new(),
                len: text.len(),
            };
        }

        let mut state = self.0.write();

        // Compute ascent/descent from font metrics
        let mut max_ascent = 0.0f32;
        let mut max_descent = 0.0f32;
        for run in font_runs {
            let loaded_font = &state.fonts[run.font_id.0];
            if let Some(font_ref) = loaded_font.font_ref() {
                let metrics = font_ref.metrics(SkriSize::unscaled(), LocationRef::default());
                let font_scale = font_size.0 / metrics.units_per_em as f32;
                max_ascent = max_ascent.max(metrics.ascent * font_scale);
                max_descent = max_descent.max(-metrics.descent * font_scale);
            }
        }

        // Build parley layout for shaping
        let state_ref = &mut *state;
        let mut builder =
            state_ref
                .layout_context
                .ranged_builder(&mut state_ref.font_context, text, 1.0, true);

        // Apply font styles per run
        let mut offset = 0;
        let mut break_ligature = true;
        for run in font_runs {
            let loaded_font = &state_ref.fonts[run.font_id.0];
            let family_name = loaded_font.family_name.clone();
            let range = offset..offset + run.len;

            // Set font family for this range
            builder.push(
                parley::style::StyleProperty::FontStack(parley::style::FontStack::Single(
                    parley::style::FontFamily::Named(family_name.into()),
                )),
                range.clone(),
            );

            // Alternate font size slightly to break ligatures across runs
            // (same trick used by CoreText and DirectWrite implementations)
            let run_font_size = if break_ligature {
                font_size.0.next_up()
            } else {
                font_size.0
            };
            builder.push(parley::style::StyleProperty::FontSize(run_font_size), range);
            break_ligature = !break_ligature;

            offset += run.len;
        }

        let mut layout = builder.build(text);
        layout.break_all_lines(None);

        // Extract glyphs from parley layout using cluster-based iteration
        let mut shaped_runs = Vec::<ShapedRun>::new();
        let mut total_width = 0.0f32;

        for line in layout.lines() {
            for item in line.items() {
                if let parley::layout::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let parley_run = glyph_run.run();
                    let font_id =
                        run_font_id_from_parley(parley_run.font(), font_runs, &state_ref.fonts);
                    let is_emoji = state_ref.fonts.get(font_id.0).is_some_and(|f| f.is_emoji);

                    let mut glyphs = Vec::new();

                    // Iterate clusters to get text_range, then get glyphs from each cluster.
                    let run_offset = glyph_run.offset();
                    let mut glyph_x = run_offset;

                    for cluster in parley_run.visual_clusters() {
                        let text_index = cluster.text_range().start;
                        for glyph in cluster.glyphs() {
                            let x = glyph_x + glyph.x;
                            let y = glyph.y;
                            total_width = total_width.max(x + glyph.advance);
                            glyphs.push(ShapedGlyph {
                                id: GlyphId(glyph.id),
                                position: point(px(x), px(y)),
                                index: text_index,
                                is_emoji,
                            });
                        }
                        glyph_x += cluster.glyphs().map(|g| g.advance).sum::<f32>();
                    }

                    // Merge into existing run if same font, or create new
                    if let Some(last_run) = shaped_runs.last_mut() {
                        if last_run.font_id == font_id {
                            last_run.glyphs.extend(glyphs);
                            continue;
                        }
                    }
                    shaped_runs.push(ShapedRun { font_id, glyphs });
                }
            }
        }

        LineLayout {
            font_size,
            width: px(total_width),
            ascent: px(max_ascent),
            descent: px(max_descent),
            runs: shaped_runs,
            len: text.len(),
        }
    }
}

/// Try to match a parley font reference back to one of our FontId's.
/// Falls back to the first font run's font_id if no match is found.
fn run_font_id_from_parley(
    parley_font: &parley::FontData,
    font_runs: &[FontRun],
    loaded_fonts: &[LoadedFont],
) -> FontId {
    // Parley may do font fallback, giving us a different font than requested.
    // Try matching by font data pointer/index.
    let parley_data = parley_font.data.data();
    let parley_index = parley_font.index;

    for (idx, loaded) in loaded_fonts.iter().enumerate() {
        if loaded.font_index == parley_index
            && loaded.font_data.len() == parley_data.len()
            && loaded.font_data.as_ptr() == parley_data.as_ptr()
        {
            return FontId(idx);
        }
    }

    // Fallback: just use the first font run's ID
    font_runs.first().map(|r| r.font_id).unwrap_or(FontId(0))
}

impl ParleyTextSystemState {
    fn resolve_font(&mut self, font: &Font) -> Result<FontId> {
        let family_name =
            crate::text_system::font_name_with_fallbacks(&font.family, "Inter Variable");

        // Query fontique for matching fonts
        let mut query = self
            .font_context
            .collection
            .query(&mut self.font_context.source_cache);

        let fontique_style = match font.style {
            FontStyle::Normal => parley::fontique::FontStyle::Normal,
            FontStyle::Italic => parley::fontique::FontStyle::Italic,
            FontStyle::Oblique => parley::fontique::FontStyle::Oblique(None),
        };

        // set_families accepts anything that converts Into<QueryFamily>,
        // &str implements From<&str> for QueryFamily::Named
        query.set_families([family_name]);
        query.set_attributes(parley::fontique::Attributes::new(
            parley::fontique::FontWidth::default(),
            fontique_style,
            parley::fontique::FontWeight::new(font.weight.0),
        ));

        // Use matches_with callback to get the first matching font
        let mut result: Option<(Arc<Vec<u8>>, u32)> = None;
        query.matches_with(|query_font| {
            let font_data = Arc::new(query_font.blob.data().to_vec());
            let font_index = query_font.index;
            result = Some((font_data, font_index));
            parley::fontique::QueryStatus::Stop
        });

        match result {
            Some((font_data, font_index)) => {
                // Check if this font has an 'm' glyph (guard against broken fonts)
                let has_m_glyph = skrifa::FontRef::from_index(&font_data, font_index)
                    .ok()
                    .and_then(|font_ref| {
                        let charmap = font_ref.charmap();
                        charmap.map('m').filter(|g| g.to_u32() != 0)
                    })
                    .is_some();

                // Exception for icon fonts like Segoe Fluent Icons
                let is_icon_font = family_name.contains("Icons") || family_name.contains("Symbol");

                if !has_m_glyph && !is_icon_font {
                    return Err(anyhow!(
                        "font '{}' has no 'm' character and was not loaded",
                        family_name
                    ));
                }

                // Detect emoji font by checking for color tables
                let is_emoji = detect_emoji_font(&font_data, font_index);

                let font_id = FontId(self.fonts.len());
                self.fonts.push(LoadedFont {
                    font_data,
                    font_index,
                    is_emoji,
                    family_name: family_name.to_string(),
                });
                self.font_selections.insert(font.clone(), font_id);
                Ok(font_id)
            }
            None => Err(anyhow!("could not find font family '{}'", font.family)),
        }
    }
}

/// Detect if a font is a color emoji font by checking for COLR, CBDT, or sbix tables.
fn detect_emoji_font(font_data: &[u8], font_index: u32) -> bool {
    let Ok(font_ref) = skrifa::FontRef::from_index(font_data, font_index) else {
        return false;
    };

    let colr = skrifa::raw::types::Tag::new(b"COLR");
    let cbdt = skrifa::raw::types::Tag::new(b"CBDT");
    let sbix = skrifa::raw::types::Tag::new(b"sbix");

    // Use table_data() to check for presence of each table
    font_ref.table_data(colr).is_some()
        || font_ref.table_data(cbdt).is_some()
        || font_ref.table_data(sbix).is_some()
}
