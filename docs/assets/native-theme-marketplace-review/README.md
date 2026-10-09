# Native marketplace and Sleek review

Open index.html locally. It offers theme and size selectors plus Before/After
controls. Home and Marketplace cover light/dark at 1280x800 and 800x600;
Sleek covers both sizes and its mini player. Additional captures show installed
controls, empty search, the scheme chooser, a track list and seek hover.

The first four Home baselines were captured before the implementation.
Sleek baselines use the retained color.ini importer and the Wealthy palette,
without the new native appearance. Marketplace baselines show the previous
Appearance settings, where no marketplace was available.

Captures are from the actual Linux/X11 application in demo mode. Public demo
artwork requests returned HTTP 403 in this cloud environment, so placeholder
artwork is visible. The screenshots were not edited or mocked up.

Example captures:

```sh
cargo run --locked --features demo -- --demo --demo-show sleek \
  --demo-size 1280x800 --demo-shot after-sleek.png
cargo run --locked --features demo -- --demo --demo-show marketplace,light \
  --demo-size 800x600 --demo-shot marketplace-light-narrow.png
cargo run --locked --features demo -- --demo --demo-show sleek,mini-player \
  --demo-shot sleek-mini.png
```

Validation on Linux: launcher tests, formatting, default/all-feature Clippy
with warnings denied, default/all-feature all-target test suites, doc-test
command, rustdoc with warnings denied, gettext checks and Jekyll build passed.
The main suites ran 935 and 958 tests respectively, with three existing ignored
tests in each. The doc-test target currently contains no examples to run.
Native macOS and Windows builds were not run here.

Regression coverage checks the package's 19 complete schemes, settings from
older versions, unknown and uninstalled selections, install/apply/remove,
restoring base geometry, retained installation across theme switches, and
accessible install/apply/remove controls above the preview in a narrow window.
