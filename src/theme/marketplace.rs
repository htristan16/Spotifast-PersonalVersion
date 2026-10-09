//! Curated native theme packages. CSS behavior is ported in Rust; packages
//! contain literal colors and attribution, never executable web content.

use std::sync::OnceLock;

use egui::{Color32, Context, CornerRadius, Id, Stroke};

use super::{Palette, spicetify};
use crate::settings::Settings;

pub const SOURCE_URL: &str = "https://github.com/spicetify/spicetify-themes/tree/c432012478e1f2e2ffccca8e943e188af40d6190/Sleek";
pub const VERSION: &str = "1";

/// Extra CSS variables that ordinary palette imports cannot express.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Appearance {
    pub sidebar: Color32,
    pub player: Color32,
    pub main_secondary: Color32,
    pub nav_active: Color32,
    pub nav_text: Color32,
    pub play_button: Color32,
    pub playback_bar: Color32,
    pub button_disabled: Color32,
    pub button_secondary: Color32,
}

pub struct Scheme {
    pub name: String,
    pub palette: Palette,
    pub style: Appearance,
}

pub fn schemes() -> &'static [Scheme] {
    static SCHEMES: OnceLock<Vec<Scheme>> = OnceLock::new();
    SCHEMES.get_or_init(|| {
        spicetify::parse(include_str!("../../assets/themes/sleek/color.ini"))
            .expect("bundled Sleek palettes are validated by tests")
            .into_iter()
            .map(|scheme| {
                let pick = |name: &str| *scheme.colors.get(name).expect("Sleek role");
                let style = Appearance {
                    sidebar: pick("sidebar"),
                    player: pick("player"),
                    main_secondary: pick("main-secondary"),
                    nav_active: pick("nav-active"),
                    nav_text: pick("nav-active-text"),
                    play_button: pick("play-button"),
                    playback_bar: pick("playback-bar"),
                    button_disabled: pick("button-disabled"),
                    button_secondary: pick("button-secondary"),
                };
                let mut palette = scheme.palette;
                palette.surface = style.main_secondary;
                palette.surface_hover = style.main_secondary.gamma_multiply(0.5);
                palette.surface_active = style.main_secondary;
                // Sleek's misc is white, intended for a few source icons,
                // not a white outline around every native widget.
                palette.outline = style.button_disabled;
                palette.panel = style.player;
                Scheme {
                    name: scheme.name,
                    palette,
                    style,
                }
            })
            .collect()
    })
}

pub fn scheme(name: &str) -> Option<&'static Scheme> {
    schemes().iter().find(|scheme| scheme.name == name)
}

pub fn selected(settings: &Settings) -> Option<&'static Scheme> {
    settings.sleek_installed.then_some(())?;
    scheme(settings.sleek_scheme.as_deref()?)
}

fn appearance_id() -> Id {
    Id::new("native-theme-appearance")
}

pub fn appearance(ctx: &Context) -> Option<Appearance> {
    ctx.data(|data| data.get_temp(appearance_id()))
}

/// Called after the base palette, including when returning to the default UI.
pub fn apply(ctx: &Context, appearance: Option<Appearance>) {
    ctx.data_mut(|data| match appearance {
        Some(style) => {
            data.insert_temp(appearance_id(), style);
        }
        None => data.remove::<Appearance>(appearance_id()),
    });
    if let Some(appearance) = appearance {
        let mut style = (*ctx.global_style()).clone();
        style.spacing.scroll.bar_width = 6.0;
        style.visuals.window_corner_radius = CornerRadius::same(8);
        style.visuals.window_stroke = Stroke::NONE;
        style.visuals.selection.bg_fill = appearance.main_secondary;
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget.corner_radius = CornerRadius::same(8);
        }
        ctx.set_global_style(style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_scheme_has_complete_native_roles() {
        assert_eq!(schemes().len(), 19);
        let unique: std::collections::BTreeSet<_> = schemes().iter().map(|s| &s.name).collect();
        assert_eq!(unique.len(), 19);
        assert_eq!(
            scheme("Wealthy").unwrap().style.nav_text,
            Color32::from_rgb(32, 32, 32)
        );
        for scheme in schemes() {
            assert!(scheme.palette.dark);
            assert_eq!(scheme.style.playback_bar.a(), 255);
        }
    }

    #[test]
    fn unavailable_or_uninstalled_selection_falls_back_without_losing_settings() {
        let mut settings = Settings {
            sleek_scheme: Some("future-theme".into()),
            sleek_installed: true,
            ..Default::default()
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&encoded).unwrap();
        assert!(selected(&restored).is_none());
        settings.sleek_scheme = Some("Wealthy".into());
        assert!(selected(&settings).is_some());
        settings.sleek_installed = false;
        assert!(selected(&settings).is_none());
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert!(!old.sleek_installed);
        assert!(old.sleek_scheme.is_none());
    }

    #[test]
    fn clearing_the_native_style_restores_base_geometry() {
        let ctx = Context::default();
        let sleek = scheme("Wealthy").unwrap();
        super::super::apply(&ctx, &sleek.palette);
        apply(&ctx, Some(sleek.style));
        assert_eq!(ctx.global_style().spacing.scroll.bar_width, 6.0);
        super::super::apply(&ctx, &Palette::light());
        apply(&ctx, None);
        assert!(appearance(&ctx).is_none());
        assert_eq!(
            ctx.global_style().visuals.window_corner_radius,
            CornerRadius::same(10)
        );
    }
}
