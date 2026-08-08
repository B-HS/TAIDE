# Third-Party Licenses — Bundled Themes

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
