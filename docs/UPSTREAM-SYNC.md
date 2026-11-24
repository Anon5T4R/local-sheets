# Sincronização com o upstream (storytold/gridcraft)

Porta/rebrand da suíte Local: o `main` é nosso, o upstream é fonte de mudanças candidatas e nada
entra sem revisão. Regras gerais em `docs/planos/licoes-de-porta.md` (dev-notes).

## Base

- Upstream: `storytold/gridcraft`, tag **v0.16.1** = commit **e581bc95** (`Release 0.16.1`,
  2026-11-04). A árvore foi copiada sem histórico no primeiro commit da porta (`4032f89`, fases A+B).
- Conferido no sync de 2026-11-21: a árvore importada não é exatamente o diff de `e581bc95` — dois
  commits do `main` de 03/nov já vinham nela (`065f05db` MODE por índice/FREQUENCY por busca binária
  e `db05400e` cada dependente listado uma vez), com código e teste. Eles não são reaplicáveis; a
  base para os próximos syncs continua `e581bc95`.
- Fork: `Anon5T4R/local-sheets`, versão própria na linha `0.2.x` (numeração nova evita auto-update
  falso para o upstream).
- Nada é lido de `C:\Dev\storytold` em build, teste ou release; a pasta é só referência local para
  a skill `sync-upstream`.
- `docs/UPSTREAM-MAP.toml` registra os caminhos que diferem, o que pode ser aplicado por script e o
  que exige revisão.

## Vendorizado (fora do sync normal)

| Caminho | Origem | Rev |
|---|---|---|
| `libs/craft-libs` | `storytold/craft-libs` | `c5a1cbf` (com `vendor/`) |
| `libs/craft-libs/crates/update` | **`Anon5T4R/local-libs`** (fonte única da suíte) | conferido pelo `scripts/check-mirror.sh` |
| `libs/egui` | `storytold/egui` | `6d518fc` |
| `libs/winit` | `storytold/winit` | `a517b38` |
| `libs/winprint.rs` | `storytold/winprint.rs` | `417ac3c` |

`6758393` do `winprint.rs` (build-dependencies do download do PDFium opcionais atrás da feature `pdfium`, com o código do
`build.rs` num módulo sob a mesma feature) entrou à parte do rev acima. O app usa o `winprint` com
`default-features = false`; `reqwest`, `sigstore-*` e `aws-lc-sys` deixaram de ser compilados.

`craft-update` é **fork permanente consciente** e fica fora do sync: bugfix vai direto na fonte
(`local-libs`) e é replicado com `sync.ps1 -Mode push` (guarda de hash; a cópia aqui é espelho).
Nunca edite `libs/craft-libs/crates/update` à mão.

## Divergências do rebrand (o que é nosso)

- Identidade: crates `localsheets-*`, binários `localsheets`/`localsheets-cli`, app id, ProgIds e
  UpgradeCode do MSI, chaves de registro e pasta de configuração próprios.
- XLSX continua o formato nativo — o fork não introduz formato próprio, então arquivos e ferramentas
  do upstream seguem compatíveis.
- CI próprio: sem workflows da org `storytold`; release por tag `vX.Y.Z` no Actions.
- Packaging no molde da suíte (MSI per-machine e per-user, AppImage/deb/rpm/tar, PKGBUILD).
- Fontes do `craft-fonts` passam pelo filtro de asset policy (`crates/ui-egui/src/craft_fonts_policy.rs`).
- Aba IA (BYOK) é nossa: `libs/local-ai` vendored + crate `localsheets-ai`, chave no keychain do SO.
- MCP/CLI: servidor `crates/mcp` (stdio + backends headless/`Remote`) e a crate compartilhada
  `craft-mcp` no `--connect`.

## Registro de sync

A base continua `e581bc95`: só pontos escolhidos entram (skill `sync-upstream`), não um bump.

| Data | Alvo (gridcraft) | Pegos | Descartados relevantes |
|---|---|---|---|
| 2026-11-21 | `origin/main` de 2026-11-21 (`11a5711a`); commits de 2026-11-03 a 2026-11-11 | `aa4951fd` → `304690e` (WORKDAY, PIVOTBY, SORT e as funções D param de reescanear a entrada); `bbe08254` → `f8ca170` (TEXTJOIN monta o resultado dentro do limite de texto); `7cb00883`+`9ab297ce` → `47045bf` (edição estrutural reescreve só as fórmulas que muda, com os oráculos `adjust_oracle` e `structural_edit_rewrite`); `35e98156` → `d6f0f32` (um orçamento de trabalho só para todas as chamadas de uma função elevada); `67a0b7f3`+`bc162524` → `293e380` (rasterização, splines e rótulos de categoria com teto) | `065f05db` e `db05400e` já estavam na base; de `bc162524`, o caminho rápido `needs_sanitizing` (sempre sanitizamos e os níveis já têm teto), o remendo de `BarRun` (não temos a fusão de barras) e o da média móvel (somamos a janela direto, sem somas de bloco) |

Nenhum item pediu crate ou API que não temos; o único pré-requisito foi `Formula::with_expr` (do
`b6bbdcd5`, que não temos), trazido junto no commit do item 3.
