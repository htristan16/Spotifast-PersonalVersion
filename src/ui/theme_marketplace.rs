//! The bundled marketplace. Previewing a scheme never changes the active UI.

use crate::app::App;
use crate::i18n::gettext;
use crate::model::Action;
use crate::theme::{self, Icon, marketplace};
use egui::{Align2, CornerRadius, Frame, Id, Margin, Rect, Sense, Vec2, pos2, vec2};

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.theme_marketplace_open {
        return;
    }
    let locale = app.locale;
    let palette = app.palette;
    let mut open = true;
    let search_id = Id::new("marketplace-search");
    let scheme_id = Id::new("marketplace-scheme");
    let installed_id = Id::new("marketplace-installed");
    let mut search = ctx
        .data(|d| d.get_temp::<String>(search_id))
        .unwrap_or_default();
    let mut installed_only = ctx
        .data(|d| d.get_temp::<bool>(installed_id))
        .unwrap_or(false);
    let mut selected = ctx
        .data(|d| d.get_temp::<String>(scheme_id))
        .or_else(|| app.settings.sleek_scheme.clone())
        .unwrap_or_else(|| "Wealthy".into());
    egui::Window::new(gettext(locale, "Theme marketplace"))
        .id(Id::new("theme-marketplace-window"))
        .open(&mut open).collapsible(false).resizable(true)
        .default_width(600.0).default_height(560.0).max_width((ctx.content_rect().width() - 40.0).max(240.0))
        .max_height((ctx.content_rect().height() - 80.0).max(200.0))
        .show(ctx, |ui| {
            theme::text(ui, gettext(locale, "Native theme designs, bundled for offline use."), theme::regular(13.0), palette.secondary);
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut search)
                    .hint_text(gettext(locale, "Search")).desired_width(180.0));
                ui.checkbox(&mut installed_only, gettext(locale, "Installed"));
            });
            ui.add_space(8.0);
            let needle = search.trim().to_lowercase();
            let matches = "sleek harbassan spicetify".contains(&needle)
                || marketplace::schemes().iter().any(|scheme| scheme.name.to_lowercase().contains(&needle));
            egui::ScrollArea::vertical().id_salt("marketplace-results").show(ui, |ui| {
                if !matches || (installed_only && !app.settings.sleek_installed) {
                    theme::text(ui, gettext(locale, "No themes found."), theme::regular(14.0), palette.secondary);
                    return;
                }
                Frame::new().fill(palette.surface).corner_radius(8).inner_margin(Margin::same(16))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            theme::text(ui, "Sleek", theme::bold(24.0), palette.text);
                            theme::text(ui, format!("harbassan · MIT · v{}", marketplace::VERSION), theme::regular(12.0), palette.secondary);
                        });
                        theme::text(ui, gettext(locale, "Gradient navigation, rounded artwork and a full-width seek bar."), theme::regular(13.0), palette.secondary);
                        ui.add_space(10.0);
                        egui::ComboBox::from_id_salt("marketplace-color-scheme")
                            .selected_text(&selected).show_ui(ui, |ui| {
                                for scheme in marketplace::schemes() {
                                    ui.selectable_value(&mut selected, scheme.name.clone(), &scheme.name);
                                }
                            }).response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, ui.is_enabled(), gettext(locale, "Color scheme")));
                        let scheme = marketplace::scheme(&selected).unwrap_or(&marketplace::schemes()[0]);
                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            if !app.settings.sleek_installed {
                                if theme::soft_button(ui, &palette, Some(Icon::Plus), &gettext(locale, "Install"), true).clicked() {
                                    app.actions.push(Action::InstallSleek);
                                }
                            } else {
                                let active = app.settings.sleek_scheme.as_deref() == Some(scheme.name.as_str());
                                let label = if active { gettext(locale, "Active") } else { gettext(locale, "Apply") };
                                ui.add_enabled_ui(!active, |ui| {
                                    if theme::soft_button(ui, &palette, None, &label, true).clicked() {
                                        app.actions.push(Action::ApplySleek(scheme.name.clone()));
                                    }
                                });
                                if theme::soft_button(ui, &palette, None, &gettext(locale, "Remove"), false).clicked() {
                                    app.actions.push(Action::RemoveSleek);
                                }
                            }
                            if theme::soft_button(ui, &palette, Some(Icon::ExternalLink), &gettext(locale, "View source"), false).clicked() {
                                app.actions.push(Action::OpenUrl(marketplace::SOURCE_URL.into()));
                            }
                        });
                        ui.add_space(10.0);
                        preview(ui, scheme);
                        ui.add_space(6.0);
                        theme::text(ui, gettext(locale, "19 color schemes. Native style port; Spotify CSS and extensions do not run here."), theme::regular(12.0), palette.secondary);
                    });
            });
        });
    ctx.data_mut(|d| {
        d.insert_temp(search_id, search);
        d.insert_temp(scheme_id, selected);
        d.insert_temp(installed_id, installed_only);
    });
    app.theme_marketplace_open = open;
}

/// A small native shell preview using the same CSS tokens as the live UI.
fn preview(ui: &mut egui::Ui, scheme: &marketplace::Scheme) {
    let width = ui.available_width();
    let height = (ui.ctx().content_rect().height() * 0.24).clamp(130.0, 190.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let p = scheme.palette;
    let s = scheme.style;
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, CornerRadius::same(8), p.window);
    let side = Rect::from_min_size(rect.min, vec2(width * 0.23, height - 30.0));
    super::sidebar::sleek_gradient(&painter, side, s.sidebar, s.player);
    let nav = Rect::from_min_size(side.min + vec2(8.0, 14.0), vec2(side.width() - 16.0, 24.0));
    painter.rect_filled(nav, 8.0, s.nav_active);
    theme::paint_icon(ui, Icon::House, nav, 16.0, s.nav_text);
    for index in 0..3 {
        let x = side.right() + 16.0 + index as f32 * ((width - side.width() - 32.0) / 3.0);
        let size = ((width - side.width() - 48.0) / 3.0).clamp(16.0, (height - 90.0).min(76.0));
        let cover = Rect::from_min_size(pos2(x, rect.top() + 35.0), Vec2::splat(size));
        painter.rect_filled(cover, 8.0, s.main_secondary);
        theme::paint_icon(ui, Icon::Music, cover, size * 0.4, p.secondary);
        painter.line_segment(
            [
                pos2(x, cover.bottom() + 12.0),
                pos2(x + size * 0.8, cover.bottom() + 12.0),
            ],
            egui::Stroke::new(3.0, p.text),
        );
    }
    let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 30.0), rect.max);
    painter.rect_filled(bar, 0.0, s.player);
    painter.hline(
        bar.x_range(),
        bar.top(),
        egui::Stroke::new(3.0, s.button_disabled),
    );
    painter.hline(
        rect.left()..=rect.left() + width * 0.58,
        bar.top(),
        egui::Stroke::new(3.0, s.playback_bar),
    );
    theme::paint_icon(ui, Icon::PlayFilled, bar, 16.0, s.play_button);
    painter.text(
        pos2(side.right() + 16.0, rect.top() + 15.0),
        Align2::LEFT_CENTER,
        &scheme.name,
        theme::semibold(13.0),
        p.text,
    );
}
