# Third-Party Licenses

This file covers three kinds of third-party code TAIDE distributes notices for:
bundled color themes (shipped inside the app), bundled TextMate grammars used
for syntax highlighting (shipped inside the app via the `shiki` package), and
language servers that the in-app LSP installer downloads on demand (not
bundled — fetched from the upstream project's own release infrastructure at
install time, and cached under the user's app-data directory).

## Bundled Themes

TAIDE ships 36 color themes derived from popular VS Code extensions as built-in
(`builtin: true`) themes under `src-tauri/resources/themes/*.json`. Each source
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

## GitHub Dark / GitHub Light

- Bundled as: `github-dark`, `github-light`
- Source: https://github.com/primer/github-vscode-theme
- License: MIT
- Copyright (c) 2020 Primer

## Tokyo Night

- Bundled as: `tokyo-night`
- Source: https://github.com/enkia/tokyo-night-vscode-theme
- License: MIT
- Copyright (c) 2018-present Enkia

## Catppuccin (Mocha)

- Bundled as: `catppuccin-mocha`
- Source: https://github.com/catppuccin/vscode
- License: MIT
- Copyright (c) 2021 Catppuccin

## Nord

- Bundled as: `nord`
- Source: https://github.com/nordtheme/visual-studio-code
- License: MIT
- Copyright (C) 2017-present Arctic Ice Studio <development@arcticicestudio.com>
- Copyright (C) 2017-present Sven Greb <development@svengreb.de>

## Gruvbox (Dark)

- Bundled as: `gruvbox-dark`
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

## Ayu Dark / Ayu Light

- Bundled as: `ayu-dark`, `ayu-light`
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

## Rosé Pine / Rosé Pine Dawn

- Bundled as: `rose-pine`, `rose-pine-dawn`
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

---

## Downloaded Language Servers

The entries below use the `download` install strategy in
`src-tauri/resources/lsp-servers.json` — TAIDE's LSP installer fetches the
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
