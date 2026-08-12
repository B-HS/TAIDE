# W7 번들 테마 원본 취득 매니페스트 (재변환 재현용)

## 대상 파일

- `scripts/convert-vscode-theme.ts` (변환기 — prettier 포맷 출력 포함)
- `src-tauri/resources/themes/*.json` (산출물 36종)

## 리포트

W7 재변환 시 번들 36종의 원본 VS Code 테마 JSON 을 재취득했다. 원본은 레포에 커밋하지 않는
정책(`docs/theme-system.md` §8.2)이라, 다음 재변환을 위해 **확보 경로**를 여기 기록한다.
W5 시점 취득 경로가 미기록이라 이번에 10종의 색이 달라졌다(원인 규명은 `docs/PROCESS.md`
W7-C 항목) — 같은 문제를 반복하지 않기 위한 문서다.

취득 절차: 원본을 한 디렉터리에 아래 파일명으로 모으고(include 체인 로컬 미러 규약 —
`INCLUDE_ROLE_FILENAME_MAP`), 각 테마의 CLI 인자는 커밋된 산출물의 `id/name/type/author/
license/source` 필드에서 그대로 읽어 재사용한다. `--include-dir` 는 그 디렉터리.

## 상세 — 파일명 → 취득 URL

VS Code 내장(브랜치 main, `raw.githubusercontent.com/microsoft/vscode/main/extensions/...`):

| 저장 파일명 | 경로 |
|---|---|
| base-dark-vs.json | theme-defaults/themes/dark_vs.json |
| base-light-vs.json | theme-defaults/themes/light_vs.json |
| vscode-dark-plus.json | theme-defaults/themes/dark_plus.json |
| vscode-light-plus.json | theme-defaults/themes/light_plus.json |
| vscode-dark-modern.json | theme-defaults/themes/dark_modern.json |
| vscode-light-modern.json | theme-defaults/themes/light_modern.json |
| monokai.json | theme-monokai/themes/monokai-color-theme.json |
| solarized-light.json | theme-solarized-light/themes/solarized-light-color-theme.json |
| vscode-solarized-dark.json | theme-solarized-dark/themes/solarized-dark-color-theme.json |
| vscode-abyss.json | theme-abyss/themes/abyss-color-theme.json |
| vscode-monokai-dimmed.json | theme-monokai-dimmed/themes/dimmed-monokai-color-theme.json |
| vscode-tomorrow-night-blue.json | theme-tomorrow-night-blue/themes/tomorrow-night-blue-color-theme.json |
| vscode-kimbie-dark.json | theme-kimbie-dark/themes/kimbie-dark-color-theme.json |
| vscode-red.json | theme-red/themes/Red-color-theme.json |
| vscode-quiet-light.json | theme-quietlight/themes/quietlight-color-theme.json |

커뮤니티(raw.githubusercontent.com, 별도 표기 없으면 기본 브랜치):

| 저장 파일명 | 경로 |
|---|---|
| one-dark-pro.json | Binaryify/OneDark-Pro/master/themes/OneDark-Pro.json |
| tokyo-night.json | enkia/tokyo-night-vscode-theme/master/themes/tokyo-night-color-theme.json |
| nord.json | nordtheme/visual-studio-code/develop/themes/nord-color-theme.json |
| gruvbox-dark.json | jdinhify/vscode-theme-gruvbox/**v1.22.0**/themes/gruvbox-dark-medium.json (이후 태그는 themes/ 미커밋 — 빌드 산출물) |
| intellij-islands-light.json | a-havrysh/vscode-intellij-theme/main/themes/intellij-islands-light-theme.json |
| ayu-dark.json / ayu-light.json | ayu-theme/vscode-ayu/master/ayu-dark.json · ayu-light.json (저장소 루트) |
| palenight.json | whizkydee/vscode-palenight-theme/master/themes/palenight.json |
| night-owl.json / night-owl-light.json | sdras/night-owl-vscode-theme/master/themes/Night Owl-color-theme.json · Night Owl-Light-color-theme.json (URL 인코딩 필요) |
| rose-pine.json / rose-pine-dawn.json | rose-pine/vscode/main/themes/rose-pine-color-theme.json · rose-pine-dawn-color-theme.json |
| everforest-dark.json / everforest-light.json | sainnhe/everforest-vscode/master/themes/everforest-dark.json · everforest-light.json |
| kanagawa-wave.json | paccodes/kanagawa-vscode-theme/main/themes/kanagawa-wave-color-theme.json |
| vitesse-dark.json / vitesse-light.json | antfu/vscode-theme-vitesse/main/themes/vitesse-dark.json · vitesse-light.json |
| one-monokai.json | azemoh/vscode-one-monokai/master/themes/OneMonokai-color-theme.json |
| darcula.json | rokoroku/vscode-theme-darcula/master/themes/darcula.json |

vsix 로만 배포(빌드 산출물이 저장소에 없음 — 압축 해제 후 내부 경로에서 추출):

| 저장 파일명 | vsix 취득처 | vsix 내부 경로 |
|---|---|---|
| dracula.json | github.com/dracula/visual-studio-code/releases (dracula.vsix) | extension/theme/dracula.json |
| catppuccin-mocha.json | github.com/catppuccin/vscode/releases (catppuccin-vsc-*.vsix) | extension/themes/mocha.json |
| github-dark.json / github-light.json | open-vsx.org/api/GitHub/github-vscode-theme/{ver}/file/... (.vsix — 릴리스에 자산 없음, MS Marketplace 미사용) | extension/themes/dark.json · light.json |

주의:
- 상류가 갱신되면 colors/syntax/terminal 절 diff 가 생긴다 — 재변환 시 반드시 tokenColors 제외
  3절 diff 를 확인하고, 상이하면 원인(상류 갱신 vs 변환기 변경)을 격리 실험(구버전 변환기로
  동일 원본 변환)으로 규명한 뒤 채택을 결정한다.
- github-dark/light 는 primer 의 "GitHub Dark/Light"(classic, dark.json/light.json)다.
  dark-default 등 변형과 혼동 금지.
