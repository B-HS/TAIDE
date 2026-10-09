# Third-Party Licenses

TAIDE itself is released under the MIT License — see `LICENSE` at the repository
root. Everything below is about code and data TAIDE bundles or fetches that
belongs to someone else.

This file covers four kinds of third-party code TAIDE distributes notices for:
bundled color themes (shipped inside the app), bundled TextMate grammars used
for syntax highlighting (shipped inside the app via the `shiki` package, and
embedded as the same files in the Rust-native build),
language servers that the in-app LSP installer downloads on demand (not
bundled — fetched from the upstream project's own release infrastructure at
install time, and cached under the user's app-data directory), and a small
number of bundled npm libraries whose non-trivial internal licensing (a
transitive dependency, or non-obvious authorship) warrants its own notice
rather than relying on `package.json`/`bun.lock` alone.

## Bundled Themes

TAIDE ships 47 color themes derived from popular VS Code extensions as built-in
(`builtin: true`) themes under `crates/taide-theme/resources/themes/*.json`. Each source
extension is MIT licensed; this file records the copyright notices required by
the MIT license ("include the copyright notice and this permission notice in
all copies or substantial portions of the Software").

Color/syntax/terminal values were extracted from each project's published VS
Code theme JSON and mechanically converted into TAIDE's own theme schema
(`docs/theme-system.md` §2) by `scripts/convert-vscode-theme.ts`. No source
code from these projects is included in TAIDE — only the resulting color
values, which are re-expressed under TAIDE's own token names.

## One Dark Pro

- Bundled as: `one-dark-pro`
- Source: https://github.com/Binaryify/OneDark-Pro
- License: MIT
- Copyright (c) 2013-2022 Binaryify

## Dracula

- Bundled as: `dracula`
- Source: https://github.com/dracula/visual-studio-code
- License: MIT
- Copyright (c) 2016 Dracula Theme

## GitHub Dark / GitHub Dark Dimmed / GitHub Light

- Bundled as: `github-dark`, `github-dark-dimmed`, `github-light`
- Source: https://github.com/primer/github-vscode-theme
- License: MIT
- Copyright (c) 2020 Primer

## Tokyo Night / Tokyo Night Storm / Tokyo Night Light

- Bundled as: `tokyo-night`, `tokyo-night-storm`, `tokyo-night-light`
- Source: https://github.com/tokyo-night/tokyo-night-vscode-theme
- License: MIT
- Copyright (c) 2018-present Enkia
- Note: the project moved from `enkia/tokyo-night-vscode-theme` to the
  `tokyo-night` organization; the old URL still redirects there.

## Catppuccin (Latte / Frappé / Macchiato / Mocha)

- Bundled as: `catppuccin-latte`, `catppuccin-frappe`, `catppuccin-macchiato`, `catppuccin-mocha`
- Source: https://github.com/catppuccin/vscode
- License: MIT
- Copyright (c) 2021 Catppuccin

## Nord

- Bundled as: `nord`
- Source: https://github.com/nordtheme/visual-studio-code
- License: MIT
- Copyright (C) 2017-present Arctic Ice Studio <development@arcticicestudio.com>
- Copyright (C) 2017-present Sven Greb <development@svengreb.de>

## Gruvbox (Dark / Light)

- Bundled as: `gruvbox-dark`, `gruvbox-light`
- Source: https://github.com/jdinhify/vscode-theme-gruvbox
- License: MIT
- Copyright (c) 2017 JD

## Monokai

- Bundled as: `monokai`
- Source: https://github.com/microsoft/vscode/tree/main/extensions/theme-monokai
- License: MIT (VS Code built-in extension)
- Copyright (c) 2015 - present Microsoft Corporation

## Solarized Light

- Bundled as: `solarized-light`
- Source: https://github.com/microsoft/vscode/tree/main/extensions/theme-solarized-light
- License: MIT (VS Code built-in extension; original Solarized color scheme by Ethan Schoonover)
- Copyright (c) 2015 - present Microsoft Corporation

## Abyss / Monokai Dimmed / Solarized Dark / Tomorrow Night Blue

- Bundled as: `vscode-abyss`, `vscode-monokai-dimmed`, `vscode-solarized-dark`, `vscode-tomorrow-night-blue`
- Source: https://github.com/microsoft/vscode (extensions/theme-abyss, theme-monokai-dimmed,
  theme-solarized-dark, theme-tomorrow-night-blue)
- License: MIT (VS Code built-in extensions)
- Copyright (c) 2015 - present Microsoft Corporation

## IntelliJ Islands Light

- Bundled as: `intellij-islands-light`
- Source: https://github.com/a-havrysh/vscode-intellij-theme
- License: MIT
- Copyright (c) a-havrysh

## Ayu Dark / Ayu Mirage / Ayu Light

- Bundled as: `ayu-dark`, `ayu-mirage`, `ayu-light`
- Source: https://github.com/ayu-theme/vscode-ayu
- License: MIT
- Copyright (c) ayu-theme (dempfi)

## Palenight

- Bundled as: `palenight`
- Source: https://github.com/whizkydee/vscode-palenight-theme
- License: MIT
- Copyright (c) Olaolu Olawuyi (whizkydee)

## Night Owl / Night Owl Light

- Bundled as: `night-owl`, `night-owl-light`
- Source: https://github.com/sdras/night-owl-vscode-theme
- License: MIT
- Copyright (c) Sarah Drasner

## Rosé Pine / Rosé Pine Moon / Rosé Pine Dawn

- Bundled as: `rose-pine`, `rose-pine-moon`, `rose-pine-dawn`
- Source: https://github.com/rose-pine/vscode
- License: MIT
- Copyright (c) Rosé Pine

## Everforest Dark / Everforest Light

- Bundled as: `everforest-dark`, `everforest-light`
- Source: https://github.com/sainnhe/everforest-vscode
- License: MIT
- Copyright (c) sainnhe

## Kanagawa Wave

- Bundled as: `kanagawa-wave`
- Source: https://github.com/paccodes/kanagawa-vscode-theme
- License: MIT
- Copyright (c) paccodes (original color scheme: rebelot/kanagawa.nvim, also MIT)

## Vitesse Dark / Vitesse Light

- Bundled as: `vitesse-dark`, `vitesse-light`
- Source: https://github.com/antfu/vscode-theme-vitesse
- License: MIT
- Copyright (c) Anthony Fu (antfu)

## One Monokai

- Bundled as: `one-monokai`
- Source: https://github.com/azemoh/vscode-one-monokai
- License: MIT
- Copyright (c) Joshua Azemoh (azemoh)

## Dark+ / Light+ / Dark Modern / Light Modern / Kimbie Dark / Red / Quiet Light

- Bundled as: `vscode-dark-plus`, `vscode-light-plus`, `vscode-dark-modern`,
  `vscode-light-modern`, `vscode-kimbie-dark`, `vscode-red`, `vscode-quiet-light`
- Source: https://github.com/microsoft/vscode (extensions/theme-defaults,
  theme-kimbie-dark, theme-red, theme-quietlight)
- License: MIT (VS Code built-in extensions)
- Copyright (c) 2015 - present Microsoft Corporation
- Note: none of these source themes declare `terminal.ansi*` colors. The
  missing ANSI 16-color set was filled with VS Code's own official default
  ANSI palette (see `docs/theme-system.md` §8.2) — the same fallback VS Code
  itself applies at runtime when a theme is silent on terminal colors.

## Darcula

- Bundled as: `darcula`
- Source: https://github.com/rokoroku/vscode-theme-darcula
- License: MIT
- Copyright (c) rokoroku (original color scheme: JetBrains Darcula)
- Note: source theme declares no `terminal.ansi*` colors; filled with VS
  Code's official default dark ANSI palette (see `docs/theme-system.md` §8.2).

---

## Visual Studio 2019 (C++) Dark / Light

- Bundled as: `visual-studio-cpp-dark`, `visual-studio-cpp-light`
- Source: https://github.com/microsoft/vscode-cpptools
- License: MIT
- Copyright (c) Microsoft Corporation
- Note: source themes declare no `terminal.ansi*` colors; filled with VS
  Code's official default dark/light ANSI palettes (see `docs/theme-system.md` §8.2).

## Full MIT License Text

The MIT License (MIT) applies to all themes listed above (copyright holders
as noted per-theme):

```
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
```

---

## Bundled TextMate Grammars

TAIDE renders syntax highlighting via [shiki](https://shiki.style) 4.4.3
(`@shikijs/core`, `@shikijs/engine-javascript`, `@shikijs/langs`,
`@shikijs/monaco`, `@shikijs/vscode-textmate`), which bundles TextMate
grammar files (`.tmLanguage.json`) sourced from upstream editor/extension
projects. **Unlike the color themes above, grammar files are redistributed
as-is** — shiki repackages the upstream TextMate grammar JSON largely
unmodified, so this is closer to "including the file" than "extracting
values from the file." TAIDE imports 30 of these grammars individually
(never the full `@shikijs/langs` barrel — see "Individual subpath imports
only" below).

**The Rust-native build ships the same grammar files.**
`native/taide-native-syntax/grammars/` holds 37 `.tmLanguage.json` files — the
30 grammars below plus the 7 grammar modules they import (see "Embedded
grammar modules" below). They are extracted from `@shikijs/langs` 4.4.3 by
`docs/utils/2026-10-06-extract-shiki-grammars.ts`, which writes out the JSON
text of each module as is, without parsing and re-serializing it, and they are
compiled into the native binary with `include_str!`. Every notice in this
section therefore applies to the native binary as well as to the web bundle.

### shiki packages (MIT)

- `@shikijs/core`, `@shikijs/langs`, `@shikijs/monaco`,
  `@shikijs/engine-javascript`, `@shikijs/vscode-textmate` — MIT.
- Copyright (c) 2021 Pine Wu
- Copyright (c) 2023 Anthony Fu and Shiki contributors
- Full MIT text: see `## Full MIT License Text` above.

### Bundled grammars — 30 languages

| TAIDE language id | shiki language id | Upstream source                           | License                                  |
| ----------------- | ----------------- | ----------------------------------------- | ---------------------------------------- |
| `rust`            | `rust`            | github.com/microsoft/vscode               | MIT                                      |
| `typescript`      | `typescript`      | github.com/microsoft/vscode               | MIT                                      |
| `typescriptreact` | `tsx`             | github.com/microsoft/vscode               | MIT                                      |
| `javascript`      | `javascript`      | github.com/microsoft/vscode               | MIT                                      |
| `javascriptreact` | `jsx`             | github.com/microsoft/vscode               | MIT                                      |
| `json`            | `json`            | github.com/microsoft/vscode               | MIT                                      |
| `jsonc`           | `jsonc`           | github.com/microsoft/vscode               | MIT                                      |
| `markdown`        | `markdown`        | github.com/microsoft/vscode               | MIT                                      |
| `toml`            | `toml`            | github.com/textmate/toml.tmbundle         | _(no upstream LICENSE file — see below)_ |
| `yaml`            | `yaml`            | github.com/textmate/yaml.tmbundle         | _(no upstream LICENSE file — see below)_ |
| `html`            | `html`            | github.com/microsoft/vscode               | MIT                                      |
| `css`             | `css`             | github.com/microsoft/vscode               | MIT                                      |
| `scss`            | `scss`            | github.com/microsoft/vscode               | MIT                                      |
| `python`          | `python`          | github.com/microsoft/vscode               | MIT                                      |
| `go`              | `go`              | github.com/microsoft/vscode               | MIT                                      |
| `shellscript`     | `shellscript`     | github.com/microsoft/vscode               | MIT                                      |
| `java`            | `java`            | github.com/microsoft/vscode               | MIT                                      |
| `ruby`            | `ruby`            | github.com/microsoft/vscode               | MIT                                      |
| `erb`             | `erb`             | github.com/textmate/ruby.tmbundle         | _(no upstream LICENSE file — see below)_ |
| `dart`            | `dart`            | github.com/microsoft/vscode               | MIT                                      |
| `swift`           | `swift`           | github.com/jtbandes/swift-tmlanguage      | MIT                                      |
| `scala`           | `scala`           | github.com/scala/vscode-scala-syntax      | MIT                                      |
| `elixir`          | `elixir`          | github.com/elixir-editors/elixir-tmbundle | NOASSERTION (see below)                  |
| `haskell`         | `haskell`         | github.com/octref/language-haskell        | BSD-3-Clause (see below)                 |
| `c`               | `c`               | github.com/microsoft/vscode               | MIT                                      |
| `cpp`             | `cpp`             | github.com/microsoft/vscode               | MIT                                      |
| `kotlin`          | `kotlin`          | github.com/fwcd/vscode-kotlin             | MIT                                      |
| `lua`             | `lua`             | github.com/microsoft/vscode               | MIT                                      |
| `zig`             | `zig`             | github.com/ziglang/vscode-zig             | MIT                                      |
| `hcl`             | `hcl`             | github.com/hashicorp/syntax               | MPL-2.0 (see below)                      |

`heex` (`.heex` files) has **no bundled grammar** — `@shikijs/langs` 4.4.3 does
not ship a `heex` TextMate grammar. TAIDE maps `.heex` files to the shiki
`html` grammar as a partial fallback (HTML tags/attributes/strings render
correctly; the `<%= %>` EEx expression syntax does not). `plaintext` has no
grammar by design.

MIT-licensed entries above: copyright and permission notices are as
recorded by the individual upstream projects; TAIDE does not modify the
grammar files, satisfying the MIT notice-preservation requirement via
unmodified redistribution. See `## Full MIT License Text` above for the
license text.

### Embedded grammar modules — 7

The `@shikijs/langs` modules of `cpp` and `ruby` statically import further
grammar modules, so loading those two languages also loads the grammars
below. They are part of both the web bundle and the native binary (37 grammar
files in total: the 30 above plus these 7). The module list is produced by
`docs/utils/2026-10-06-extract-shiki-grammars.ts` and recorded in
`native/taide-native-syntax/grammars/manifest.json`.

| shiki language id | Scope name                  | Imported by        | Upstream source                       | License (see note)     |
| ----------------- | --------------------------- | ------------------ | ------------------------------------- | ---------------------- |
| `cpp-macro`       | `source.cpp.embedded.macro` | `cpp`              | github.com/microsoft/vscode           | MIT                    |
| `regexp`          | `source.regexp.python`      | `cpp`, `cpp-macro` | github.com/MagicStack/MagicPython     | MIT                    |
| `glsl`            | `source.glsl`               | `cpp`, `cpp-macro` | github.com/polym0rph/GLSL.tmbundle    | _no license specified_ |
| `sql`             | `source.sql`                | `ruby`             | github.com/microsoft/vscode           | MIT                    |
| `graphql`         | `source.graphql`            | `ruby`             | github.com/prisma-labs/vscode-graphql | MIT                    |
| `haml`            | `text.haml`                 | `ruby`             | github.com/karuna/haml-vscode         | MIT                    |
| `xml`             | `text.xml`                  | `ruby`             | github.com/microsoft/vscode           | MIT                    |

**Note.** The grammar files carry no upstream or license metadata. The
upstream source and license columns were checked on 2026-10-07 against the
grammar table of the `tm-grammars` package README on the `main` branch of
github.com/shikijs/textmate-grammars-themes, not against the exact revision
that `@shikijs/langs` 4.4.3 was built from. That table lists no license for
`glsl`, so it is a gray-area entry like the four below; it already ships in
the web bundle through the `cpp` module. None of the seven is one of the
GPL-3.0 grammars listed under "Individual subpath imports only".

### Haskell grammar — BSD-3-Clause

- Bundled as: shiki `haskell` (TAIDE id `haskell`)
- Source: https://github.com/octref/language-haskell
- License: BSD-3-Clause

```
Copyright (c) the language-haskell contributors

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright
   notice, this list of conditions and the following disclaimer in the
   documentation and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.
```

### HCL grammar — MPL-2.0

- Bundled as: shiki `hcl` (TAIDE id `hcl`)
- Source: https://github.com/hashicorp/syntax
- License: MPL-2.0 (Mozilla Public License 2.0) — full text:
  https://www.mozilla.org/en-US/MPL/2.0/
- Copyright (c) HashiCorp, Inc.
- **MPL-2.0 is file-level copyleft.** TAIDE imports this grammar file
  unmodified via `@shikijs/langs/hcl` and does not patch or re-derive it.
  TAIDE's policy is to **never modify** shiki-provided grammar files — doing
  so would trigger the obligation to publish the modified file's source.
  Unmodified redistribution is satisfied by pointing to the source above.

### Elixir grammar — NOASSERTION

- Bundled as: shiki `elixir` (TAIDE id `elixir`)
- Source: https://github.com/elixir-editors/elixir-tmbundle
- License: NOASSERTION — the upstream repository does not declare an
  SPDX-identifiable license.

### TOML / YAML / ERB grammars — no upstream LICENSE file

- Bundled as: shiki `toml`, `yaml`, `erb` (TAIDE ids `toml`, `yaml`, `erb`)
- Sources: github.com/textmate/toml.tmbundle, github.com/textmate/yaml.tmbundle,
  github.com/textmate/ruby.tmbundle
- These upstream repositories do not include a `LICENSE` file at all.

### Redistribution basis for the gray-area entries above

The four entries with no clear SPDX license (`elixir` NOASSERTION; `toml`,
`yaml`, `erb` with no upstream `LICENSE` file) do not have a legally crisp
redistribution basis. TAIDE bundles them on the following grounds, which
were presented to and approved by the user on 2026-08-12
(`docs/acknowledge/2026-08-12-w7-textmate-contract.md`):

1. These exact grammar files have been redistributed by VS Code, GitHub
   Linguist, and shiki itself (as an MIT-licensed package) for years,
   establishing a widely-relied-upon industry practice.
2. shiki — the package TAIDE depends on — already redistributes them under
   its own MIT license without separately relicensing the grammar content.
3. This is not a legal determination; if any of these upstream projects
   later publish a license that conflicts with redistribution, TAIDE will
   remove or replace the affected grammar.

### Individual subpath imports only

TAIDE imports each grammar via its own `@shikijs/langs/<id>` subpath (e.g.
`@shikijs/langs/rust`) and never imports the `@shikijs/langs` package root
(barrel import). This is a deliberate license-hygiene measure: `@shikijs/langs`
as a whole also ships several GPL-3.0 grammars (`ada`, `gnuplot`, `nginx`,
`org`, `racket`) that TAIDE does not use and must not pull in incidentally.

The native extraction script follows the same rule. It starts from the
grammar modules named in `src/shared/lib/shiki/lang-map.ts`, follows only
their static imports, and aborts if any of the five GPL-3.0 grammar ids
appears in that closure.

---

## Native syntax highlighting engine

The Rust-native build tokenizes the grammars above with two crates that
`native/taide-native-syntax` depends on. Neither is part of the web bundle.

### ferriki-textmate 0.12.0

- Source: https://github.com/sebastian-software/ferriki
- License: MIT OR Apache-2.0 (MIT text: see `## Full MIT License Text` above;
  Apache-2.0 text: https://www.apache.org/licenses/LICENSE-2.0)
- Copyright (c) 2026 Sebastian Software GmbH, Mainz, Germany
- Portions Copyright (c) 2021 Pine Wu
- Portions Copyright (c) 2023 Anthony Fu
- A port of `vscode-textmate` — MIT, Copyright (c) Microsoft Corporation.

### ferroni 1.8.1

- Source: https://github.com/sebastian-software/ferroni
- License: BSD-2-Clause
- Portions of its scanner are derived from `vscode-oniguruma` — MIT,
  Copyright (c) Microsoft Corporation.

```
Copyright (c) 2002-2021 K. Kosako <kkosako0@gmail.com> (Oniguruma C original)
Copyright (c) 2026 Sebastian Software GmbH (Rust port)

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

1. Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright
   notice, this list of conditions and the following disclaimer in the
   documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

### regress 0.12.0 (native find)

- Source: https://github.com/ridiculousfish/regress
- License: MIT OR Apache-2.0 (MIT text: see `## Full MIT License Text` above;
  Apache-2.0 text: https://www.apache.org/licenses/LICENSE-2.0)
- Copyright (c) 2020 ridiculous_fish
- Used only by `native/taide-native-syntax` for the Rust-native find widget.
  The editor and UI crates do not depend on this engine, and it is not part
  of the frozen browser Wasm client's dependency graph.

### Codicons (native find widget)

- Source: https://github.com/microsoft/vscode-codicons
- Creator: Microsoft Corporation and contributors
- License: Creative Commons Attribution 4.0 International — full text:
  https://creativecommons.org/licenses/by/4.0/legalcode
- `native/taide-native-app/assets/codicons/codicon.ttf` is an unchanged copy
  from Monaco Editor 0.56.0, used for the original find and replace icons.
  Attribution, source, license and warranty information are retained in
  `native/taide-native-app/assets/codicons/NOTICE.md`. No endorsement is implied.

### Ported source in `native/taide-native-ui`

- `src/editor-fold-keymap-defaults.json` adapts the original folding keyboard
  bindings from Monaco Editor 0.56.0 (MIT, Copyright (c) Microsoft Corporation),
  `editor/contrib/folding/browser/folding.js`. The same MIT text is retained in
  `native/taide-native-ui/LICENSE-MONACO-FIND`.

- `src/editor-minimap-layout.rs`, `src/editor-minimap.rs` and their native
  editor surface connections adapt layout, glyph blending, selection,
  slider and pointer/touch interactions from Monaco Editor 0.56.0 (MIT,
  Copyright (c) Microsoft Corporation):
  `editor/common/config/editorOptions.js`,
  `editor/browser/viewParts/minimap/minimap.js`, `minimap.css`,
  `editor/browser/viewParts/minimap/minimapCharRenderer.js` and
  `editor/browser/viewParts/minimap/minimapPreBaked.js`.
  `resources/minimap/scale-1.bin` and `scale-2.bin` contain the original
  prebaked glyph intensity data. `tests/fixtures/minimap-reference.txt`
  records results from executing the original layout and glyph renderer;
  `docs/utils/2026-10-09-native-minimap-reference.js` extracts these fixtures
  and assets. The same MIT text is retained in
  `native/taide-native-ui/LICENSE-MONACO-FIND`.

- `src/editor-find.rs`, `src/editor-find-widget.rs` and
  `src/editor-find-keymap-defaults.json` adapt the controller, widget,
  history and keyboard behavior of Monaco Editor 0.56.0 (MIT, Copyright (c)
  Microsoft Corporation), revision `f487add297079a02eb836810185b165e50cadabc`:
  `editor/contrib/find/browser/findController.js`, `findModel.js`,
  `findWidget.js`, `findOptionsWidget.js` and `findWidget.css`.
  The license text is retained in `native/taide-native-ui/LICENSE-MONACO-FIND`.

- `src/editor-display.rs`, `src/editor-caret.rs`, `src/editor-scroll.rs` and
  the native display paths in `src/editor_surface.rs` adapt Monaco Editor
  0.56.0 (MIT, Copyright (c) Microsoft Corporation):
  `editor/browser/viewParts/whitespace/whitespace.js`,
  `editor/browser/viewParts/rulers/rulers.js`,
  `editor/browser/viewParts/viewCursors/viewCursor.js`, `viewCursors.js`,
  `viewCursors.css`, `editor/common/viewLayout/viewLayout.js`,
  `editor/common/core/editorColorRegistry.js` and `base/common/scrollable.js`.
  The same MIT text is retained in `native/taide-native-ui/LICENSE-MONACO-FIND`.

- `src/editor-brackets.rs` and the bracket display paths in
  `src/editor-geometry.rs` adapt the bracket/indent guide display of Monaco
  Editor 0.56.0 (MIT, Copyright (c) Microsoft Corporation):
  `editor/browser/viewParts/indentGuides/indentGuides.js`,
  `indentGuides.css`, `editor/common/viewModel/viewModelLines.js` and
  `editor/common/core/editorColorRegistry.js`. Bracket matching adapts
  `editor/contrib/bracketMatching/browser/bracketMatching.js` and
  `bracketMatching.css`; widget focus and sticky inline decoration boundaries
  follow `editor/common/viewModel/inlineDecorations.js` and
  `editor/contrib/stickyScroll/browser/stickyScrollWidget.js`.
  The same MIT text is retained
  in `native/taide-native-ui/LICENSE-MONACO-FIND`.

### Restored egui/eframe documentation assets

- `native/taide-native-app/vendor/egui-input/assets/ferris.png` and
  `native/taide-native-app/vendor/eframe/data/icon.png` are unchanged
  documentation fixtures from egui/eframe 0.36.2, upstream commit
  `49682f8baa058bf49e011035cfbd6e825f88a5ef` in
  [emilk/egui](https://github.com/emilk/egui). The selected MIT license is
  retained in each vendor directory. The adjacent README files record the
  exact source paths, byte sizes and verified Git blob hashes.

### Ported source in `native/taide-native-syntax`

- `src/style-scopes.rs` ports the color and font style lookup of
  `@shikijs/monaco` 4.4.3 (MIT, Copyright (c) 2021 Pine Wu, Copyright (c) 2023
  Anthony Fu) and `toStandardTokenType` from Monaco Editor 0.56.0 (MIT,
  Copyright (c) Microsoft Corporation). The license texts are kept next to
  the code as `native/taide-native-syntax/LICENSE-SHIKI` and
  `native/taide-native-syntax/LICENSE-MONACO-SNIPPET`.
- `src/token-theme.rs` ports `normalizeTheme` from `@shikijs/primitive` 4.4.3
  and the rule list of `textmateThemeToMonacoTheme` from `@shikijs/monaco`
  4.4.3 (both MIT, same copyright holders as above), and the default token
  rule of `StandaloneTheme.tokenTheme` from Monaco Editor 0.56.0.
- `src/monaco-token-theme.rs` ports the token theme trie from Monaco Editor
  0.56.0 (`parseTokenTheme`, `resolveParsedTokenThemeRules`, `ColorMap`,
  `ThemeTrieElement`, `ThemeTrieElementRule`).
- `src/document-tokens.rs` and `src/token-worker.rs` port the line end state
  store and the tokenization stop rule from Monaco Editor 0.56.0
  (`TokenizationStateStore`, `TrackingTokenizationStateStore`).
- `src/grammar-registrations.rs` ports the language registration order of
  `@shikijs/primitive` 4.4.3 (`Resolver.addLanguage`,
  `Registry.loadLanguages`, `Registry.loadLanguage`, `resolveLangAlias`; MIT,
  same copyright holders as `@shikijs/monaco`) and follows the grammar cache
  rule of `@shikijs/vscode-textmate` 10.0.2 (`SyncRegistry.addGrammar`,
  `SyncRegistry.grammarForScopeName`; MIT, Copyright (c) Microsoft
  Corporation).
- `src/language-configuration.rs` ports the regular expression construction
  and the rule evaluation of Monaco Editor 0.56.0 (MIT, Copyright (c)
  Microsoft Corporation), all from `editor/common/languages` unless noted:
  `OnEnterSupport` of `supports/onEnter.js`, `IndentRulesSupport` of
  `supports/indentRules.js`, `CharacterPairSupport` of
  `supports/characterPair.js`, `groupFuzzyBrackets`,
  `getReversedRegexForBrackets`, `createBracketOrRegExp` and
  `BracketsUtils.findPrevBracketInRange` of `supports/richEditBrackets.js`,
  `LanguageBracketsConfiguration.getBracketRegExp` of
  `supports/languageBracketsConfiguration.js`, `BracketTokens.getRegExpStr`
  of `editor/common/model/bracketPairsTextModelPart/bracketPairsTree/brackets.js`
  and the marker pattern of
  `editor/contrib/folding/browser/indentRangeProvider.js`.

### Monaco language configurations in `native/taide-native-syntax`

- File: `native/taide-native-syntax/language-configurations/monaco.json`,
  generated by `docs/utils/2026-10-07-extract-monaco-language-configurations.ts`
  from `monaco-editor` 0.56.0 (MIT, Copyright (c) Microsoft Corporation).
- It holds the `conf` language configuration of the 22 Monaco language
  definitions that TAIDE's language ids use
  (`esm/vs/languages/definitions/<language>/<language>.js`), the JSON
  configuration of `esm/vs/languages/features/json/jsonMode.js` and the
  plain text configuration of
  `esm/vs/editor/common/languages/languageConfigurationRegistry.js`. Regular
  expressions are stored as their unchanged `source` and `flags`.
- `native/taide-native-syntax/tests/fixtures/language-configurations/reference.json`
  is generated by the same script. It records what Monaco's own classes
  return for those configurations on synthetic lines.
- The license text is kept next to the data as
  `native/taide-native-syntax/LICENSE-MONACO-SNIPPET`.

### Ported source in `native/taide-native-editor`

- `src/sticky-model.rs`, `native/taide-native-ui/src/editor-sticky-scroll.rs`
  and their editor surface connections adapt nested scope selection,
  push-off layout, hidden range filtering, rendering and interactions from
  Monaco Editor 0.56.0 (MIT, Copyright (c) Microsoft Corporation):
  `editor/contrib/stickyScroll/browser/stickyScrollProvider.js`,
  `stickyScrollModelProvider.js`, `stickyScrollController.js`,
  `stickyScrollWidget.js`, `stickyScrollActions.js` and `stickyScroll.css`.
  `native/taide-native-editor/tests/fixtures/sticky-scroll-reference.txt`
  records 357 results obtained by executing the original provider/controller
  methods with synthetic document and viewport data. The MIT text is retained
  in `native/taide-native-editor/LICENSE-MONACO-SNIPPET` and
  `native/taide-native-ui/LICENSE-MONACO-FIND`.

- `src/document-symbols.rs` and `native/taide-native-app/src/editor-symbols.rs`
  adapt the outline selection-range headers, provider preference and coverage
  selection from the same Monaco sticky scroll model provider. The `@` palette
  normalization, preorder breadcrumbs and interaction contract follow TAIDE's
  existing TypeScript implementation. `native/taide-native-app/resources/icons/braces.svg`
  carries the Braces geometry from `lucide-react` 1.28.0 (ISC, Copyright (c)
  Lucide Icons and Contributors); the ISC text is retained in the adjacent
  `resources/icons/LICENSE.txt`.

- `native/taide-native-app/src/breadcrumb-menu.rs` adapts sibling-menu keyboard
  navigation, focus restoration and typeahead from `@radix-ui/react-menu`
  2.1.24 (MIT, Copyright (c) 2022 WorkOS). The MIT text is retained in
  `native/taide-native-app/LICENSE-RADIX-MENU`. The seven additional symbol/tree
  SVG files in `native/taide-native-app/resources/icons` carry geometry from
  `lucide-react` 1.28.0 under the same adjacent ISC license.

- `native/taide-native-editor/src/symbol-locations.rs`,
  `native/taide-native-ui/src/editor-locations.rs`,
  `native/taide-native-ui/src/editor-definition-link.rs` and
  `native/taide-native-app/src/editor-locations.rs` adapt location ordering,
  reference grouping, inline peek layout, navigation, focus and definition
  link interactions from Monaco Editor 0.56.0 (MIT, Copyright (c) Microsoft
  Corporation): `editor/contrib/gotoSymbol/browser/goToCommands.js`,
  `link/goToDefinitionAtPosition.js`, `referencesModel.js`,
  `peek/referencesController.js`, `peek/referencesWidget.js`,
  `peek/referencesTree.js` and their associated styles. The same MIT text
  is retained in `native/taide-native-editor/LICENSE-MONACO-SNIPPET` and
  `native/taide-native-ui/LICENSE-MONACO-FIND`.

- `src/find.rs` and `src/find-replacement.rs` adapt the search driving,
  word boundary, newline, empty match, replacement parsing and case
  preservation rules from Monaco Editor 0.56.0 (MIT, Copyright (c)
  Microsoft Corporation), revision
  `f487add297079a02eb836810185b165e50cadabc`:
  `editor/common/model/textModelSearch.js`,
  `editor/contrib/find/browser/replacePattern.js` and
  `base/common/search.js`. The license text is retained in
  `native/taide-native-editor/LICENSE-MONACO-SNIPPET`.
- `src/line-tokens.rs` ports the invalid line range queue
  (`RangePriorityQueueImpl`, `OffsetRange.addRange`) and the line token
  editing rules (`ContiguousTokensStore.acceptEdit`,
  `ContiguousTokensEditing`) from Monaco Editor 0.56.0 (MIT, Copyright (c)
  Microsoft Corporation). The license text is kept next to the code as
  `native/taide-native-editor/LICENSE-MONACO-SNIPPET`.
- `src/bracket-model.rs` adapts bracket pairing, nesting and indentation
  behavior from Monaco Editor 0.56.0 (MIT, Copyright (c) Microsoft
  Corporation): `editor/common/model/bracketPairsTextModelPart/bracketPairsTree`,
  `editor/common/model/guidesTextModelPart.js` and
  `editor/common/languages/supports/languageBracketsConfiguration.js`.
  `native/taide-native-syntax/tests/fixtures/bracket-display-reference.json`
  records results obtained by directly executing these original model APIs.
  `tests/fixtures/bracket-matching-reference.json` in that same syntax crate
  records near/enclosing results from `BracketPairsTextModelPart` in
  `editor/common/model/bracketPairsTextModelPart/bracketPairsImpl.js`, executed
  by `docs/utils/2026-10-09-native-bracket-matching-reference.js`.
  The same MIT text is retained in
  `native/taide-native-editor/LICENSE-MONACO-SNIPPET`.
- `src/folding.rs` ports the indentation based folding of Monaco Editor
  0.56.0 (MIT, Copyright (c) Microsoft Corporation), all from
  `editor/contrib/folding/browser` unless noted: the region computation of
  `indentRangeProvider.js` (`computeRanges`, `RangesCollector.insertFirst`,
  `RangesCollector.toIndentRanges`) with `computeIndentLevel` from
  `editor/common/model/utils.js`, the region merge and lookup of
  `foldingRanges.js` (`FoldingRegions.sanitizeAndMerge`,
  `FoldingRegions.ensureParentIndices`, `FoldingRegions.findRange`), the
  collapse state rules of `foldingModel.js` (`FoldingModel.update`,
  `FoldingModel.toggleCollapseState`, `FoldingModel.getAllRegionsAtLine`,
  `FoldingModel.getRegionAtLine`, `FoldingModel.getRegionsInside`,
  `toggleCollapseState`, `setCollapseStateLevelsDown`, `setCollapseStateUp`,
  `setCollapseStateForRest`, `getParentFoldLine`, `getPreviousFoldLine`,
  `getNextFoldLine`), the hidden range rules of `hiddenRangeModel.js`
  (`HiddenRangeModel.updateHiddenRanges`,
  `HiddenRangeModel.adjustSelections`) and the click and caret reveal rules
  of `folding.js` (`FoldingController.onEditorMouseUp`,
  `FoldingController.revealCursor`). The marker and off-side handling of
  `computeRanges`, `setCollapseStateForMatchingLines` of `foldingModel.js`
  and the line tests of `FoldAllBlockCommentsAction`, `FoldAllRegionsAction`
  and `UnfoldAllRegionsAction` of `folding.js` are ported in the same file.
  The license text is the same
  `native/taide-native-editor/LICENSE-MONACO-SNIPPET`.
- `src/syntax-folding.rs` adapts provider ordering, range sanitization and
  depth-based range limiting from `syntaxRangeProvider.js` in the same Monaco
  folding directory. The manual range creation/removal and imports toggle in
  `src/folding.rs` adapt the corresponding actions and controller in
  `folding.js`. The same MIT copyright and license above apply.
- `src/auto-indent.rs`, `src/auto-closing.rs`, `src/language-typing.rs` and
  `src/language-configuration.rs` port the typing rules of Monaco Editor
  0.56.0 (MIT, Copyright (c) Microsoft Corporation), all from `editor/common`:
  `EnterOperation._enter`, `AutoIndentOperation`,
  `AutoClosingOvertypeOperation`, `AutoClosingOpenCharTypeOperation`,
  `SurroundSelectionOperation`, `InterceptorElectricCharOperation` and
  `isAutoClosingOvertype` of `cursor/cursorTypeEditOperations.js`,
  `TypeOperations.typeWithInterceptors` and
  `TypeOperations.compositionEndWithInterceptors` of
  `cursor/cursorTypeOperations.js`,
  `DeleteOperations.isAutoClosingPairDelete` of
  `cursor/cursorDeleteOperations.js`, `AutoClosedAction` of
  `cursor/cursor.js`, `SurroundSelectionCommand` of
  `commands/surroundSelectionCommand.js`, `getEnterAction` of
  `languages/enterAction.js`, `getInheritIndentForLine`, `getIndentForEnter`
  and `getIndentActionForType` of `languages/autoIndent.js`,
  `IndentationContextProcessor` and `IndentationLineProcessor` of
  `languages/supports/indentationLineProcessor.js`,
  `StandardAutoClosingPairConditional` of
  `languages/languageConfiguration.js`,
  `BracketElectricCharacterSupport.onElectricCharacter` of
  `languages/supports/electricCharacter.js` and the closing bracket rule of
  the bracket pair parser in
  `model/bracketPairsTextModelPart/bracketPairsTree/parser.js`. The license
  text is the same `native/taide-native-editor/LICENSE-MONACO-SNIPPET`.

---

## Downloaded Language Servers

The entries below use the `download` install strategy in
`crates/taide-lsp/resources/lsp-servers.json` — TAIDE's LSP installer fetches the
listed release artifact from the upstream project directly, verifies it by
SHA-256, and unpacks it into `{appData}/lsp/<server>/<version>/`. No source
or binary code from these projects is committed to the TAIDE repository or
embedded in the TAIDE application bundle; each is retrieved by the end user's
own running copy of TAIDE, on demand, from the upstream project's own release
infrastructure. Language servers installed via a system toolchain (`go
install`, `gem install`, Coursier, GHCup) or detected from an existing SDK
(Dart/Flutter, Xcode) are not listed here — TAIDE never downloads or
redistributes those.

### Eclipse JDT Language Server (jdtls)

- Bundled as: `jdtls`
- Source: https://github.com/eclipse-jdtls/eclipse.jdt.ls
- License: EPL-2.0 (Eclipse Public License 2.0) — full text:
  https://www.eclipse.org/legal/epl-2.0/
- Copyright (c) the Eclipse JDT Language Server contributors

### clangd

- Bundled as: `clangd`
- Source: https://github.com/clangd/clangd (upstream: LLVM/clang-tools-extra)
- License: Apache-2.0 WITH LLVM-exception — full text:
  https://llvm.org/LICENSE.txt
- Copyright (c) the LLVM Project contributors

### Expert (Elixir)

- Bundled as: `expert`
- Source: https://github.com/expert-lsp/expert (formerly `elixir-lang/expert`)
- License: Apache-2.0 — full text: https://www.apache.org/licenses/LICENSE-2.0
- Copyright (c) the Expert contributors
- Note: the project is in alpha (per upstream README). The settings UI should
  surface this to users before they trigger an install.

### lua-language-server

- Bundled as: `luaLanguageServer`
- Source: https://github.com/LuaLS/lua-language-server
- License: MIT (see `## Full MIT License Text` above)
- Copyright (c) the LuaLS / lua-language-server contributors — see the
  archive's own `LICENSE` file for the exact notice

### Taplo (TOML)

- Bundled as: `taplo`
- Source: https://github.com/tamasfe/taplo
- License: MIT (see `## Full MIT License Text` above)
- Copyright (c) the Taplo contributors — see the archive's own `LICENSE`
  file for the exact notice

### zls (Zig)

- Bundled as: `zls`
- Source: https://github.com/zigtools/zls
- License: MIT (see `## Full MIT License Text` above)
- Copyright (c) the zigtools / zls contributors — see the archive's own
  `LICENSE` file for the exact notice

### terraform-ls

- Bundled as: `terraformLs`
- Source: https://github.com/hashicorp/terraform-ls
- License: MPL-2.0 (Mozilla Public License 2.0) — full text:
  https://www.mozilla.org/en-US/MPL/2.0/
- Copyright (c) HashiCorp, Inc.

### Kotlin LSP (JetBrains)

- Bundled as: `kotlinLsp`
- Source: https://github.com/Kotlin/kotlin-lsp
- License: the `kotlin-lsp` repository is Apache-2.0, but the distributed
  release archive bundles a JetBrains Runtime (JBR) and, per the project's
  own README, "proprietary parts of JetBrains Air and Fleet". This is **not**
  a pure Apache-2.0 redistribution — copyright/license notices from
  `license/` inside the release archive should be reviewed before any wider
  redistribution of a cached copy, beyond the on-demand per-user download
  TAIDE performs.
- Copyright (c) JetBrains s.r.o.
- Note: the project is in alpha (per upstream README). The settings UI should
  surface this to users before they trigger an install.

---

## Bundled npm Libraries

### emmet-monaco-es

- Bundled as: an npm runtime dependency (`emmet-monaco-es` in `package.json`),
  compiled into the app bundle — not downloaded on demand like the language
  servers above.
- Source: https://github.com/troy351/emmet-monaco-es
- License: MIT (see `## Full MIT License Text` above)
- Copyright (c) 2017 Troy (`troy351`) — the package's own `LICENSE` carries no
  name (`Copyright (c) 2017` only); attributed here from `package.json`'s
  `author`/`repository` fields.
- Note: wires monaco's editor to
  [Emmet](https://github.com/emmetio/emmet)'s abbreviation-expansion engine
  (`docs/features/editor.md` — Emmet). `emmet-monaco-es`'s own `emmet`
  transitive dependency is also MIT licensed.
- Copyright (c) Sergey Chikuyonok (`emmet`)

---

## Runtime Dependencies (summary)

The full dependency inventory is `bun.lock` (npm) and `Cargo.lock` (Rust); this
section records the license classes found there so the Apache-2.0 notice
requirement is met and any non-permissive terms are visible at a glance.
Surveyed 2026-09-05 from `package.json` `license` fields (production
dependency closure, 171 packages) and `cargo metadata
--filter-platform aarch64-apple-darwin` (392 crates) —
`docs/acknowledge/2026-09-05-license-mit-decision.md`.

### npm (bundled into the app)

- MIT, ISC, 0BSD — the large majority, including `monaco-editor`, `@xterm/*`,
  `react`, `radix-ui`, `@tanstack/*`, `@dnd-kit/*`, `i18next`, `cmdk`,
  `@rhwp/core` (HWP/HWPX rendering, Copyright (c) 2025-2026 Edward Kim),
  `lucide-react` (ISC).
- Apache-2.0 — `pdfjs-dist` (Mozilla), `xlsx` (SheetJS) and its helper packages
  (`cfb`, `ssf`, `codepage`, `crc-32`, `adler-32`, `frac`, `wmf`, `word`),
  `class-variance-authority`. Full text: https://www.apache.org/licenses/LICENSE-2.0
- `dompurify` — MPL-2.0 OR Apache-2.0; TAIDE takes it under Apache-2.0.

### Rust crates (compiled into the app binary)

- MIT and/or Apache-2.0 (dual or either) — the large majority, including
  `tauri` and its plugins, `tokio`, `serde`, `git2`/`libgit2-sys`, `notify`
  (CC0-1.0), `portable-pty`, `reqwest`, `rustls` (Apache-2.0 OR ISC OR MIT),
  `ring` (Apache-2.0 AND ISC), `keyring`, `fontdb`, `sysinfo`.
- Unicode-3.0 — the ICU4X crates (`icu_*`, `zerovec`, `yoke`, `tinystr`, …).
- CDLA-Permissive-2.0 — `webpki-roots` (Mozilla CA bundle).
- Zlib, BSD-2-Clause, BSD-3-Clause, Unlicense, CC0-1.0 — a handful of small
  crates (`foldhash`, `slotmap`, `subtle`, `Inflector`, `walkdir`, `notify`).
- `ferriki-textmate` (MIT OR Apache-2.0) and `ferroni` (BSD-2-Clause) — the
  Rust-native build only; see "Native syntax highlighting engine" above.
- **MPL-2.0 (file-level copyleft)** — `cssparser`, `cssparser-macros`,
  `selectors`, `dtoa-short`, `option-ext`. TAIDE uses them unmodified, as it
  does the HCL grammar above; modifying any of these files would require
  publishing that file's source.

### Vendored native libraries (statically linked)

| Library      | Via                         | License                                                                  |
| ------------ | --------------------------- | ------------------------------------------------------------------------ |
| libgit2 1.9  | `git2` (`vendored-libgit2`) | GPL-2.0 with linking exception (permits linking into an MIT application) |
| OpenSSL 3    | `git2` (`vendored-openssl`) | Apache-2.0                                                               |
| libssh2      | `git2` (`ssh`)              | BSD-3-Clause                                                             |
| liblzma (xz) | `xz2` (`static`)            | 0BSD / public domain                                                     |
| zlib         | `libz-sys`                  | Zlib                                                                     |

Language servers are never bundled (see "Downloaded Language Servers" above),
so their licenses — including the proprietary parts of the JetBrains Kotlin
LSP archive — do not attach to the TAIDE application.
