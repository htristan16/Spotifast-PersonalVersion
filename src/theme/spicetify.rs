//! Import Spicetify color schemes as ordinary native palette files.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;

use egui::Color32;

use super::{CustomTheme, Palette};

const MAX_BYTES: usize = 64 * 1024;
const MAX_SCHEMES: usize = 64;

#[derive(Debug)]
pub struct Scheme {
    pub name: String,
    pub palette: Palette,
    pub colors: BTreeMap<String, Color32>,
}

/// Literal colors only. References need the user's Spicetify environment and
/// must never be silently replaced with a different color.
fn color(value: &str) -> Option<Color32> {
    let value = value.trim();
    if value.contains(',') {
        let rgb: Vec<_> = value
            .split(',')
            .map(|part| part.trim().parse::<u8>())
            .collect();
        if let [Ok(r), Ok(g), Ok(b)] = rgb.as_slice() {
            return Some(Color32::from_rgb(*r, *g, *b));
        }
        return None;
    }
    let hex = value.strip_prefix('#').unwrap_or(value);
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        3 => {
            let mut rgb = [0; 3];
            for (index, digit) in hex.bytes().enumerate() {
                rgb[index] = (digit as char).to_digit(16)? as u8 * 17;
            }
            Some(Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
        }
        6 => fastframe_theme::parse_color(&format!("#{hex}")),
        _ => None,
    }
}

fn luminance(color: Color32) -> f32 {
    let linear = |byte: u8| {
        let value = byte as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

fn palette(colors: &BTreeMap<String, Color32>) -> Palette {
    let main = colors
        .get("main")
        .copied()
        .unwrap_or(Color32::from_rgb(18, 18, 18));
    let text = colors.get("text").copied().unwrap_or(Color32::WHITE);
    let dark = luminance(main) < luminance(text);
    let mut palette = if dark {
        Palette::dark()
    } else {
        Palette::light()
    };
    let pick = |names: &[&str], fallback| {
        names
            .iter()
            .find_map(|name| colors.get(*name).copied())
            .unwrap_or(fallback)
    };
    palette.window = main;
    palette.text = text;
    palette.panel = pick(&["player", "sidebar"], palette.panel);
    palette.surface = pick(&["card", "sidebar"], palette.surface);
    palette.surface_hover = pick(&["selected-row"], palette.surface.lerp_to_gamma(text, 0.08));
    palette.surface_active = pick(
        &["tab-active", "selected-row"],
        palette.surface.lerp_to_gamma(text, 0.15),
    );
    palette.outline = pick(&["misc"], palette.surface.lerp_to_gamma(text, 0.18));
    palette.secondary = pick(&["subtext"], text.lerp_to_gamma(main, 0.3));
    palette.dim = pick(&["button-disabled"], text.lerp_to_gamma(main, 0.5));
    palette.accent = pick(&["button", "play-button"], palette.accent);
    palette.accent_hover = pick(&["button-active"], palette.accent.lerp_to_gamma(text, 0.12));
    // Choose the better contrasting text for transport buttons in either theme.
    palette.on_accent = if luminance(palette.accent) > 0.179 {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    // Spicetify's notification color can be a background. Native errors
    // use colored text, so retain their readable fallback when necessary.
    if let Some(error) = colors.get("notification-error") {
        let foreground = luminance(*error);
        let background = luminance(palette.panel);
        if (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05) >= 3.0 {
            palette.danger = *error;
        }
    }
    palette.overlay = pick(&["notification"], palette.panel);
    if let Some(shadow) = colors.get("shadow") {
        palette.shadow = Color32::from_rgba_unmultiplied(
            shadow.r(),
            shadow.g(),
            shadow.b(),
            if dark { 140 } else { 50 },
        );
    }
    palette
}

/// Reads every named section, retaining source order for the default scheme.
/// Invalid input rejects the entire import before any files are written.
pub fn parse(text: &str) -> Result<Vec<Scheme>, String> {
    if text.len() > MAX_BYTES {
        return Err("color.ini is larger than 64 KiB".into());
    }
    let mut sections: Vec<(String, BTreeMap<String, Color32>)> = Vec::new();
    for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        let bad_line = || format!("Invalid color.ini at line {}", index + 1);
        if line.starts_with('[') {
            let end = line.find(']').ok_or_else(bad_line)?;
            let name = line[1..end].trim();
            let tail = line[end + 1..].trim();
            if name.is_empty()
                || name.chars().count() > 100
                || (!tail.is_empty() && !tail.starts_with([';', '#']))
            {
                return Err(bad_line());
            }
            if sections.iter().any(|(existing, _)| existing == name) {
                return Err(format!("Duplicate color scheme: {name}"));
            }
            if sections.len() == MAX_SCHEMES {
                return Err("color.ini has more than 64 schemes".into());
            }
            sections.push((name.into(), BTreeMap::new()));
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(bad_line)?;
        let (scheme, colors) = sections.last_mut().ok_or_else(bad_line)?;
        let key = key.trim().to_ascii_lowercase();
        let value = value.split(';').next().unwrap_or_default().trim();
        // Some themes put inline comments after hex values.
        let value = value.split(" #").next().unwrap_or(value).trim();
        if key.is_empty() {
            return Err(bad_line());
        }
        let parsed = color(value).ok_or_else(|| {
            format!("{scheme}: {key} needs a literal hex or RGB color; resolve environment/XResources references before importing")
        })?;
        colors.insert(key, parsed);
    }
    if sections.is_empty() || sections.iter().any(|(_, colors)| colors.is_empty()) {
        return Err("color.ini must contain named, nonempty color schemes".into());
    }
    Ok(sections
        .into_iter()
        .map(|(name, colors)| Scheme {
            name,
            palette: palette(&colors),
            colors,
        })
        .collect())
}

fn file_label(label: &str) -> String {
    let label: String = label
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '-'
            } else {
                ch
            }
        })
        .scan(0, |bytes, ch| {
            *bytes += ch.len_utf8();
            (*bytes <= 60).then_some(ch)
        })
        .collect();
    let label = label.trim_matches([' ', '.']);
    if label.is_empty() {
        "Theme".into()
    } else {
        label.into()
    }
}

fn encode(palette: Palette) -> Result<String, String> {
    let fields = [
        ("window", palette.window),
        ("panel", palette.panel),
        ("surface", palette.surface),
        ("surface_hover", palette.surface_hover),
        ("surface_active", palette.surface_active),
        ("outline", palette.outline),
        ("text", palette.text),
        ("secondary", palette.secondary),
        ("dim", palette.dim),
        ("accent", palette.accent),
        ("accent_hover", palette.accent_hover),
        ("on_accent", palette.on_accent),
        ("danger", palette.danger),
        ("warning", palette.warning),
        ("overlay", palette.overlay),
        ("shadow", palette.shadow),
    ];
    let colors: BTreeMap<_, _> = fields
        .into_iter()
        .map(|(name, color)| {
            let hex: String = color
                .to_srgba_unmultiplied()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            (name, format!("#{hex}"))
        })
        .collect();
    serde_json::to_string_pretty(
        &serde_json::json!({"base": if palette.dark { "dark" } else { "light" }, "colors": colors}),
    )
    .map_err(|error| error.to_string())
}

/// Commits complete JSON files without replacing existing user palettes.
/// Reimporting unchanged schemes reuses their files; changed schemes get a suffix.
fn store(directory: &Path, label: &str, text: &str) -> Result<(String, bool), String> {
    let temporary = directory.join(format!(".spicetify-{:016x}.tmp", rand::random::<u64>()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    let result = (|| {
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| error.to_string())?;
        drop(file);
        for suffix in 1..=128 {
            let filename = if suffix == 1 {
                format!("{label}.json")
            } else {
                format!("{label} ({suffix}).json")
            };
            let target = directory.join(&filename);
            match std::fs::hard_link(&temporary, &target) {
                Ok(()) => return Ok((filename, true)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if std::fs::symlink_metadata(&target)
                        .is_ok_and(|meta| meta.is_file() && meta.len() == text.len() as u64)
                        && std::fs::read_to_string(&target).is_ok_and(|existing| existing == text)
                    {
                        return Ok((filename, false));
                    }
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("Too many files with the same theme name".into())
    })();
    let _ = std::fs::remove_file(temporary);
    result
}

/// Runs on a blocking worker, never on the drawing thread. Only the selected
/// color.ini is read; CSS, JavaScript, and assets are neither read nor executed.
pub fn import(source: &Path, directory: &Path) -> Result<Vec<CustomTheme>, String> {
    if !std::fs::metadata(source)
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("Select a regular color.ini file".into());
    }
    let mut text = String::new();
    let mut file = std::fs::File::open(source)
        .map_err(|error| error.to_string())?
        .take((MAX_BYTES + 1) as u64);
    file.read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    let schemes = parse(&text)?;
    let theme = source
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("Theme");
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let entries = std::fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .take(fastframe_theme::MAX_DIRECTORY_ENTRIES + 1)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    if entries.len() > fastframe_theme::MAX_DIRECTORY_ENTRIES {
        return Err("The themes folder has more than 512 entries".into());
    }
    let json = entries
        .iter()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
        })
        .count();
    let mut imported = Vec::new();
    let mut created = Vec::new();
    for scheme in schemes {
        let label = format!(
            "Spicetify {} - {}",
            file_label(theme),
            file_label(&scheme.name)
        );
        let result = encode(scheme.palette).and_then(|text| store(directory, &label, &text));
        match result {
            Ok((filename, new)) => {
                if new {
                    created.push(directory.join(&filename));
                }
                if entries.len() + created.len() > fastframe_theme::MAX_DIRECTORY_ENTRIES
                    || json + created.len() > fastframe_theme::MAX_THEMES
                {
                    for path in created {
                        let _ = std::fs::remove_file(path);
                    }
                    return Err(
                        "The themes folder is full; keep at most 128 palettes and 512 entries"
                            .into(),
                    );
                }
                imported.push(CustomTheme {
                    filename,
                    palette: scheme.palette,
                });
            }
            Err(error) => {
                for path in created {
                    let _ = std::fs::remove_file(path);
                }
                return Err(error);
            }
        }
    }
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\u{feff}[Dark]\r\nmain=121212\r\ntext=#fff\r\nsubtext=179, 179, 179\r\nbutton=1db954 ; green\r\nplayer=181818\r\ncard=282828\r\nshadow=334455\r\n\r\n[Light] # another scheme\r\nmain=#ffffff\r\ntext=121212\r\nbutton=005500\r\n";

    #[test]
    fn schemes_preserve_order_formats_and_light_dark_appearance() {
        let schemes = parse(SAMPLE).unwrap();
        assert_eq!(
            schemes
                .iter()
                .map(|scheme| scheme.name.as_str())
                .collect::<Vec<_>>(),
            ["Dark", "Light"]
        );
        let dark = schemes[0].palette;
        assert!(dark.dark);
        assert_eq!(dark.accent, Color32::from_rgb(29, 185, 84));
        assert_eq!(dark.secondary, Color32::from_gray(179));
        assert_eq!(dark.panel, Color32::from_gray(24));
        assert_eq!(dark.surface, Color32::from_gray(40));
        assert_eq!(dark.on_accent, Color32::BLACK);
        let light = schemes[1].palette;
        assert!(!light.dark);
        assert_eq!(light.window, Color32::WHITE);
        assert_eq!(light.on_accent, Color32::WHITE);
        for scheme in schemes {
            let json = encode(scheme.palette).unwrap();
            assert_eq!(
                fastframe_theme::parse_palette::<Palette>(&json).unwrap(),
                scheme.palette
            );
        }
    }

    #[test]
    fn notification_background_does_not_hide_native_error_text() {
        let schemes =
            parse("[Dark]\nmain=202020\nplayer=242424\nnotification-error=242424").unwrap();
        assert_ne!(schemes[0].palette.danger, schemes[0].palette.panel);
        assert_eq!(file_label(&"界".repeat(60)).len(), 60);
    }

    #[test]
    fn malformed_and_unresolved_schemes_are_rejected() {
        for input in [
            "",
            "main=123456",
            "[Empty]",
            "[Bad]\nmain=256,0,0",
            "[Bad]\nmain=1234",
            "[Bad]\nmain=${TOKEN}",
            "[Bad]\nmain=${xrdb:background}",
            "[Bad]\nmain=fff\n[Bad]\ntext=000",
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
        assert!(parse(&"x".repeat(MAX_BYTES + 1)).is_err());
        let many: String = (0..=MAX_SCHEMES)
            .map(|index| format!("[{index}]\nmain=000000\n"))
            .collect();
        assert!(parse(&many).is_err());
    }

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "spotifast-spicetify-{:016x}",
                rand::random::<u64>()
            ));
            std::fs::create_dir_all(root.join("Sleek")).unwrap();
            Self(root)
        }
        fn source(&self) -> std::path::PathBuf {
            self.0.join("Sleek/color.ini")
        }
        fn themes(&self) -> std::path::PathBuf {
            self.0.join("themes")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn importing_all_schemes_is_repeatable_and_never_replaces_user_edits() {
        let fixture = Fixture::new();
        std::fs::write(fixture.source(), SAMPLE).unwrap();
        // Theme code is unrelated to palette import and must not be touched.
        std::fs::write(
            fixture.0.join("Sleek/theme.js"),
            "throw new Error('must not execute')",
        )
        .unwrap();
        let themes = import(&fixture.source(), &fixture.themes()).unwrap();
        assert_eq!(themes.len(), 2);
        assert_eq!(
            themes,
            import(&fixture.source(), &fixture.themes()).unwrap()
        );
        for theme in &themes {
            let json = std::fs::read_to_string(fixture.themes().join(&theme.filename)).unwrap();
            assert_eq!(
                fastframe_theme::parse_palette::<Palette>(&json).unwrap(),
                theme.palette
            );
        }
        let edited = fixture.themes().join(&themes[0].filename);
        std::fs::write(&edited, "user edits").unwrap();
        let new = import(&fixture.source(), &fixture.themes()).unwrap();
        assert_ne!(new[0].filename, themes[0].filename);
        assert_eq!(new[1], themes[1]);
        assert_eq!(std::fs::read_to_string(edited).unwrap(), "user edits");
        assert_eq!(std::fs::read_dir(fixture.themes()).unwrap().count(), 3);
        assert_eq!(std::fs::read_to_string(fixture.source()).unwrap(), SAMPLE);
    }

    #[test]
    fn invalid_later_schemes_do_not_partially_import() {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.source(),
            "[Valid]\nmain=121212\n[Broken]\ntext=oops",
        )
        .unwrap();
        assert!(import(&fixture.source(), &fixture.themes()).is_err());
        assert!(!fixture.themes().exists());
        std::fs::write(fixture.source(), "[../outside:*?]\nmain=121212").unwrap();
        let themes = import(&fixture.source(), &fixture.themes()).unwrap();
        assert!(!themes[0].filename.contains(['/', '\\', ':', '*', '?']));
        assert!(fixture.themes().join(&themes[0].filename).is_file());
    }
    #[test]
    fn full_theme_folder_allows_reimports_and_rolls_back_new_palettes() {
        let fixture = Fixture::new();
        std::fs::write(fixture.source(), SAMPLE).unwrap();
        let original = import(&fixture.source(), &fixture.themes()).unwrap();
        for index in 2..fastframe_theme::MAX_THEMES {
            std::fs::write(
                fixture.themes().join(format!("Existing-{index}.json")),
                "{}",
            )
            .unwrap();
        }
        assert_eq!(
            import(&fixture.source(), &fixture.themes()).unwrap(),
            original
        );
        std::fs::write(fixture.source(), SAMPLE.replace("121212", "131313")).unwrap();
        assert!(import(&fixture.source(), &fixture.themes()).is_err());
        assert_eq!(
            std::fs::read_dir(fixture.themes()).unwrap().count(),
            fastframe_theme::MAX_THEMES
        );
        assert!(fixture.themes().join(&original[0].filename).is_file());
    }
}
