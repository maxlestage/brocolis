//! Polices et couleurs : celles du site (Archivo, vert forêt, citron vert).

use eframe::egui::{
    self, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle, Theme,
    vec2,
};

use crate::app::{ACCENT, ACCENT_INK, BG, CARD, INK, LINE, MUTED, SUNK};

pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "archivo".into(),
        FontData::from_static(include_bytes!("../assets/Archivo-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "archivo-bold".into(),
        FontData::from_static(include_bytes!("../assets/Archivo-ExtraBold.ttf")).into(),
    );
    // Archivo d'abord ; les polices d'egui restent en repli pour les
    // pictogrammes et caractères qu'Archivo n'a pas.
    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "archivo".into());
    let mut bold = vec!["archivo-bold".to_string()];
    bold.extend(fallback);
    fonts.families.insert(FontFamily::Name("bold".into()), bold);
    ctx.set_fonts(fonts);

    // L'application a une seule apparence, sombre, celle du site.
    ctx.set_theme(Theme::Dark);
    ctx.style_mut_of(Theme::Dark, |style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(12.0)),
            (TextStyle::Body, FontId::proportional(15.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (
                TextStyle::Heading,
                FontId::new(22.0, FontFamily::Name("bold".into())),
            ),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        style.spacing.item_spacing = vec2(10.0, 8.0);
        style.spacing.button_padding = vec2(14.0, 6.0);

        let v = &mut style.visuals;
        v.panel_fill = BG;
        v.window_fill = CARD;
        v.extreme_bg_color = SUNK;
        v.faint_bg_color = CARD;
        v.override_text_color = Some(INK);
        v.hyperlink_color = ACCENT;
        v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.text_cursor.stroke = Stroke::new(2.0, ACCENT);
        let radius = CornerRadius::same(10);
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = radius;
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, INK);
        v.widgets.inactive.weak_bg_fill = CARD;
        v.widgets.inactive.bg_fill = SUNK;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, INK);
        v.widgets.hovered.weak_bg_fill = CARD;
        v.widgets.hovered.bg_fill = SUNK;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, INK);
        v.widgets.active.weak_bg_fill = ACCENT;
        v.widgets.active.bg_fill = ACCENT;
        v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
        v.widgets.active.fg_stroke = Stroke::new(1.5, ACCENT_INK);
        v.weak_text_color = Some(MUTED);
        v.window_stroke = Stroke::new(1.0, LINE);
        v.disabled_alpha = 0.45;
    });
}
