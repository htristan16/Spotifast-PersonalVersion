# Sleek native theme package, version 1

Original theme: [Sleek by harbassan](https://github.com/spicetify/spicetify-themes/tree/c432012478e1f2e2ffccca8e943e188af40d6190/Sleek).
The bundled color.ini is an unchanged snapshot at that commit. Its MIT license
is included in LICENSE. No upstream CSS or JavaScript is executed.

The native Rust port reads all 19 schemes and their extra CSS color roles.
It recreates Sleek's sidebar/player diagonal gradient, colored selected
navigation with contrasting text, 8 px cards and cover artwork, smaller
cover shadows, 6 px scrollbars, rounded controls, outlined 40 x 20 toggles,
track selection and hover colors, neutral unselected titles, accent headings,
transparent play/pause controls, and square progress rails. The main player's
seek bar spans the window at the player bar's top edge, with times visible on
hover and during a seek. Mini-player controls and progress share the styling.

Spotifast keeps its own navigation, library, transport actions, fonts,
keyboard support and accessible controls. Spotify-only selectors for ads,
upgrade buttons and buddy feeds have no native equivalents. This is a native
appearance port, not a pixel-identical copy of Spotify or a CSS interpreter.

The catalog is bundled. Install enables this package in Settings; Apply
selects a scheme; Remove disables it and returns an active selection to the
previous built-in appearance. Neither operation deletes imported JSON themes.
Package updates ship with app updates. No marketplace requests are made.
