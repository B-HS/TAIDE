# Third-Party Licenses

This file covers two kinds of third-party code TAIDE distributes notices for:
bundled color themes (shipped inside the app), and language servers that the
in-app LSP installer downloads on demand (not bundled — fetched from the
upstream project's own release infrastructure at install time, and cached
under the user's app-data directory).

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
