# eframe documentation fixture

This directory preserves the relative asset path referenced by the vendored
egui documentation examples. It is not a Cargo package. The application still
uses the existing eframe dependency and patch.

`data/icon.png` is unchanged from the installed eframe 0.36.2 package, whose
upstream commit is `49682f8baa058bf49e011035cfbd6e825f88a5ef`.

Source: https://github.com/emilk/egui/blob/49682f8baa058bf49e011035cfbd6e825f88a5ef/crates/eframe/data/icon.png

Git blob SHA-1: `4ce7cc588ecd1b3a0bf81a79dc04677c21e9bf98` (12,052 bytes),
verified against the fixed upstream commit. The original MIT license is
retained in `LICENSE-MIT`. Runtime code does not load this fixture.
