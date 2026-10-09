//! A compact native player that shares the main window's theme and controls.

use egui::{Align, Align2, Id, Layout, Rect, Sense, Ui, UiBuilder, Vec2, ViewportCommand, vec2};

use crate::app::{App, NowPlaying};
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::player::RepeatMode;
use crate::theme::{self, Icon};
use crate::util;

use super::widgets::{self, SliderEvent};

const WIDTH: f32 = 420.0;
const HEIGHT: f32 = 260.0;
const COMPACT_HEIGHT: f32 = 138.0;

/// The legacy shaded-window preference now selects the compact layout.
pub fn initial_size(settings: &crate::settings::Settings) -> Vec2 {
    vec2(
        WIDTH,
        if settings.winamp_shaded {
            COMPACT_HEIGHT
        } else {
            HEIGHT
        },
    )
}

fn fit_window(ctx: &egui::Context, settings: &crate::settings::Settings) {
    let wanted = initial_size(settings);
    if (ctx.viewport_rect().size() - wanted).abs().max_elem() < 1.0 {
        return;
    }
    let id = Id::new("mini-player-resize-request");
    let time = ctx.input(|input| input.time);
    let previous: Option<(Vec2, f64)> = ctx.data(|data| data.get_temp(id));
    if previous.is_some_and(|(size, at)| size == wanted && time - at < 1.0) {
        return;
    }
    ctx.data_mut(|data| data.insert_temp(id, (wanted, time)));
    ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(wanted));
    ctx.send_viewport_cmd(ViewportCommand::MaxInnerSize(wanted));
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(wanted));
}

fn cell(ui: &mut Ui, rect: Rect, enabled: bool) -> Ui {
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
    );
    if !enabled {
        child.disable();
    }
    child
}

fn button(
    app: &App,
    ui: &mut Ui,
    center: egui::Pos2,
    icon: Icon,
    label: std::borrow::Cow<'static, str>,
    active: bool,
    enabled: bool,
) -> egui::Response {
    let mut child = cell(
        ui,
        Rect::from_center_size(center, Vec2::splat(30.0)),
        enabled,
    );
    theme::icon_button(
        &mut child,
        icon,
        18.0,
        if active {
            app.palette.accent
        } else {
            app.palette.secondary
        },
        app.palette.text,
        &label,
    )
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    fit_window(&ctx, &app.settings);
    if let Some(rect) = ctx.input(|input| input.viewport().outer_rect) {
        app.winamp.last_pos = Some([rect.min.x, rect.min.y]);
    }
    super::keys::handle(app, &ctx);
    let now = app.now_playing();
    let rect = Rect::from_min_size(ui.max_rect().min, initial_size(&app.settings));
    let palette = app.palette;
    let compact = app.settings.winamp_shaded;
    let fill = app.now_playing_tint().map_or(palette.panel, |tint| {
        palette.panel.lerp_to_gamma(tint, 0.12)
    });
    ui.painter().rect(
        rect.shrink(1.0),
        18.0,
        fill,
        egui::Stroke::new(1.0, palette.outline),
        egui::StrokeKind::Inside,
    );
    header(app, ui, rect);
    metadata(app, ui, rect, now.as_ref(), compact);
    progress(app, ui, rect, now.as_ref(), compact);
    transport(app, ui, rect, now.as_ref(), compact);
    if !compact {
        footer(app, ui, rect, now.as_ref());
    }
    super::toasts(app, &ctx, 12.0);
    super::update::show(app, &ctx);
    if now.as_ref().is_some_and(|now| now.playing) {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

fn header(app: &mut App, ui: &mut Ui, rect: Rect) {
    let at = |x, y| rect.min + vec2(x, y);
    let drag = ui.interact(
        Rect::from_min_size(at(12.0, 5.0), vec2(238.0, 31.0)),
        Id::new("mini-player-drag"),
        Sense::click_and_drag(),
    );
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        app.actions.push(Action::ToggleWinampShade);
    }
    ui.painter()
        .circle_filled(at(22.0, 22.0), 3.0, app.palette.accent);
    ui.painter().text(
        at(34.0, 22.0),
        Align2::LEFT_CENTER,
        gettext(app.locale, "Mini player"),
        theme::medium(11.0),
        app.palette.secondary,
    );
    let pinned = app.window_level_supported && app.settings.winamp_on_top;
    let pin = button(
        app,
        ui,
        at(282.0, 22.0),
        Icon::Pin,
        gettext(app.locale, "Always on top"),
        pinned,
        app.window_level_supported,
    );
    pin.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            app.window_level_supported,
            pinned,
            gettext(app.locale, "Always on top"),
        )
    });
    if pin.clicked() {
        app.actions.push(Action::ToggleWinampOnTop);
    }
    if button(
        app,
        ui,
        at(316.0, 22.0),
        if app.settings.winamp_shaded {
            Icon::ChevronDown
        } else {
            Icon::ChevronUp
        },
        if app.settings.winamp_shaded {
            gettext(app.locale, "Expand")
        } else {
            gettext(app.locale, "Compact view")
        },
        false,
        true,
    )
    .clicked()
    {
        app.actions.push(Action::ToggleWinampShade);
    }
    if button(
        app,
        ui,
        at(350.0, 22.0),
        Icon::Expand,
        gettext(app.locale, "Return to main window"),
        false,
        true,
    )
    .clicked()
    {
        app.actions.push(Action::ToggleWinampWindow);
    }
    if button(
        app,
        ui,
        at(390.0, 22.0),
        Icon::X,
        gettext(app.locale, "Close"),
        false,
        true,
    )
    .clicked()
    {
        app.actions.push(Action::CloseWindow);
    }
}

fn metadata(app: &mut App, ui: &mut Ui, rect: Rect, now: Option<&NowPlaying>, compact: bool) {
    let at = |x, y| rect.min + vec2(x, y);
    let size = if compact { 48.0 } else { 64.0 };
    let cover = Rect::from_min_size(at(18.0, 48.0), Vec2::splat(size));
    widgets::paint_cover(
        ui,
        &app.palette,
        now.and_then(|now| now.art_url.as_deref().or(now.art_small.as_deref())),
        cover,
        10.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let text = Rect::from_min_max(
        at(if compact { 80.0 } else { 98.0 }, 50.0),
        at(
            if compact { 248.0 } else { 363.0 },
            if compact { 99.0 } else { 112.0 },
        ),
    );
    let mut info = ui.new_child(
        UiBuilder::new()
            .max_rect(text)
            .layout(Layout::top_down(Align::Min)),
    );
    info.spacing_mut().item_spacing.y = 4.0;
    let title = now.map(|now| now.title.as_str()).unwrap_or("Spotifast");
    info.add(
        egui::Label::new(
            egui::RichText::new(title)
                .font(theme::medium(if compact { 13.0 } else { 15.0 }))
                .color(app.palette.text),
        )
        .truncate(),
    );
    let subtitle = now
        .map(|now| now.subtitle.as_str())
        .filter(|text| !text.is_empty());
    info.add(
        egui::Label::new(
            egui::RichText::new(subtitle.map_or_else(
                || gettext(app.locale, "Nothing playing"),
                std::borrow::Cow::Borrowed,
            ))
            .font(theme::regular(12.0))
            .color(app.palette.secondary),
        )
        .truncate(),
    );
    if !compact && let Some(now) = now {
        let saved = app.is_saved(&now.uri).unwrap_or(false);
        if button(
            app,
            ui,
            at(388.0, 79.0),
            if saved {
                Icon::HeartFilled
            } else {
                Icon::Heart
            },
            if saved {
                gettext(app.locale, "Remove from Liked Songs")
            } else {
                gettext(app.locale, "Save to Liked Songs")
            },
            saved,
            !now.is_episode,
        )
        .clicked()
        {
            app.actions.push(Action::ToggleSaved(now.uri.clone()));
        }
    }
}

fn progress(app: &mut App, ui: &mut Ui, rect: Rect, now: Option<&NowPlaying>, compact: bool) {
    let at = |x, y| rect.min + vec2(x, y);
    let (position, duration) = now
        .map(|now| (now.position_ms, now.duration_ms))
        .unwrap_or_default();
    let fraction = app.seek_preview.unwrap_or_else(|| {
        if duration == 0 {
            0.0
        } else {
            position as f32 / duration as f32
        }
    });
    let width = WIDTH - 36.0;
    let y = if compact { 115.0 } else { 133.0 };
    let mut slider = cell(
        ui,
        Rect::from_min_size(at(18.0, y - 8.0), vec2(width, 16.0)),
        duration > 0 && now.is_some_and(|now| now.can_control),
    );
    match widgets::thin_slider(
        &mut slider,
        &app.palette,
        Id::new("mini-player-seek"),
        &gettext(app.locale, "Playback position (%)"),
        fraction,
        width,
        None,
    ) {
        SliderEvent::Dragging(value) => app.seek_preview = Some(value),
        SliderEvent::Committed(value) => {
            app.seek_preview = None;
            if duration > 0 {
                app.actions
                    .push(Action::Seek((value * duration as f32).round() as u32));
            }
        }
        SliderEvent::None => {}
    }
    if !compact {
        ui.painter().text(
            at(18.0, 151.0),
            Align2::LEFT_CENTER,
            util::format_duration_ms((fraction * duration as f32) as u32),
            theme::regular(11.0),
            app.palette.secondary,
        );
        ui.painter().text(
            at(WIDTH - 18.0, 151.0),
            Align2::RIGHT_CENTER,
            util::format_duration_ms(duration),
            theme::regular(11.0),
            app.palette.secondary,
        );
    }
}

fn transport(app: &mut App, ui: &mut Ui, rect: Rect, now: Option<&NowPlaying>, compact: bool) {
    let at = |x, y| rect.min + vec2(x, y);
    let enabled = now.is_some_and(|now| now.can_control) || app.is_connected();
    let y = if compact { 73.0 } else { 189.0 };
    let center = if compact { 333.0 } else { WIDTH / 2.0 };
    let spread = if compact { 40.0 } else { 48.0 };
    if !compact {
        let shuffle = now.is_some_and(|now| now.shuffle);
        let response = button(
            app,
            ui,
            at(center - 92.0, y),
            Icon::Shuffle,
            gettext(app.locale, "Shuffle"),
            shuffle,
            enabled,
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                enabled,
                shuffle,
                gettext(app.locale, "Shuffle"),
            )
        });
        if response.clicked() {
            app.actions.push(Action::ToggleShuffle);
        }
    }
    if button(
        app,
        ui,
        at(center - spread, y),
        Icon::SkipBackFilled,
        gettext(app.locale, "Previous"),
        false,
        enabled,
    )
    .clicked()
    {
        app.actions.push(Action::Previous);
    }
    let playing = now.is_some_and(|now| now.playing);
    let diameter = if compact { 36.0 } else { 44.0 };
    let mut play = cell(
        ui,
        Rect::from_center_size(at(center, y), Vec2::splat(diameter)),
        enabled,
    );
    if now.is_some_and(|now| now.loading) || app.any_play_pending() {
        play.painter()
            .circle_filled(at(center, y), diameter / 2.0, app.palette.accent);
        theme::spinner(&mut play, 22.0, app.palette.on_accent);
    } else {
        let icon = if playing {
            Icon::PauseFilled
        } else {
            Icon::PlayFilled
        };
        let label = gettext(app.locale, if playing { "Pause" } else { "Play" });
        let response = if let Some(style) = theme::marketplace::appearance(ui.ctx()) {
            theme::icon_button(
                &mut play,
                icon,
                22.0,
                style.play_button,
                app.palette.accent_hover,
                &label,
            )
        } else {
            theme::circle_button(
                &mut play,
                icon,
                diameter,
                app.palette.accent,
                app.palette.accent_hover,
                app.palette.on_accent,
                &label,
            )
        };
        if response.clicked() {
            app.actions.push(Action::TogglePlay);
        }
    }
    if button(
        app,
        ui,
        at(center + spread, y),
        Icon::SkipForwardFilled,
        gettext(app.locale, "Next"),
        false,
        enabled,
    )
    .clicked()
    {
        app.actions.push(Action::Next);
    }
    if !compact {
        let repeat = now.map(|now| now.repeat).unwrap_or_default();
        let (icon, label) = match repeat {
            RepeatMode::Off => (Icon::Repeat, gettext(app.locale, "Repeat")),
            RepeatMode::Context => (Icon::Repeat, gettext(app.locale, "Repeat one")),
            RepeatMode::Track => (Icon::Repeat1, gettext(app.locale, "Repeat off")),
        };
        if button(
            app,
            ui,
            at(center + 92.0, y),
            icon,
            label,
            repeat != RepeatMode::Off,
            enabled,
        )
        .clicked()
        {
            app.actions.push(Action::CycleRepeat);
        }
    }
}

fn footer(app: &mut App, ui: &mut Ui, rect: Rect, now: Option<&NowPlaying>) {
    let at = |x, y| rect.min + vec2(x, y);
    let adjustable = now.is_none_or(|now| now.can_set_volume);
    let percent = app
        .volume_preview
        .map(|value| (value * 100.0).round() as u8)
        .unwrap_or_else(|| {
            now.map(|now| now.volume_percent)
                .unwrap_or_else(|| crate::app::volume_to_percent(app.local.volume))
        });
    if button(
        app,
        ui,
        at(30.0, 233.0),
        if percent == 0 {
            Icon::VolumeX
        } else {
            Icon::Volume2
        },
        if percent == 0 {
            gettext(app.locale, "Unmute")
        } else {
            gettext(app.locale, "Mute")
        },
        false,
        adjustable,
    )
    .clicked()
    {
        app.actions.push(Action::ToggleMute);
    }
    let mut volume = cell(
        ui,
        Rect::from_min_size(at(53.0, 225.0), vec2(96.0, 16.0)),
        adjustable,
    );
    match widgets::thin_slider(
        &mut volume,
        &app.palette,
        Id::new("mini-player-volume"),
        &gettext(app.locale, "Volume (%)"),
        percent as f32 / 100.0,
        96.0,
        Some(0.05),
    ) {
        SliderEvent::Dragging(value) => {
            app.volume_preview = Some(value);
            if now.is_none_or(|now| now.local) {
                app.actions
                    .push(Action::PreviewVolume((value * 100.0).round() as u8));
            }
        }
        SliderEvent::Committed(value) => {
            app.volume_preview = None;
            app.actions
                .push(Action::SetVolume((value * 100.0).round() as u8));
        }
        SliderEvent::None => {}
    }
    let device = now
        .and_then(|now| now.device_name.as_deref())
        .unwrap_or("Spotifast");
    let mut device_ui = ui.new_child(
        UiBuilder::new().max_rect(Rect::from_min_size(at(177.0, 225.0), vec2(178.0, 18.0))),
    );
    device_ui.add(
        egui::Label::new(
            egui::RichText::new(device)
                .font(theme::regular(11.0))
                .color(app.palette.secondary),
        )
        .truncate(),
    );
    if button(
        app,
        ui,
        at(388.0, 233.0),
        Icon::ListVideo,
        gettext(app.locale, "Queue"),
        false,
        true,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::Queue));
        app.actions.push(Action::ToggleWinampWindow);
    }
}
