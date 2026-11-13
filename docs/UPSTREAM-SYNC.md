# Sincronização com o upstream (storytold/gridcraft)

## Objetivo
Manter o `local-sheets` próximo do `storytold/gridcraft` sem perder a independência do rebranding:
nenhum merge cego, nenhuma dependência de servidor da org deles, nenhum `git+` no lock. Tudo que
entrar no main deste repo passa por revisão primeiro.

**Baseline:** gridcraft `v0.16.1` = `e581bc95` (o commit importado na Fase A). XLSX continua sendo o
formato nativo — o fork não introduz formato próprio, então arquivos e ferramentas do upstream
seguem compatíveis.

## Princípios
1. **O main é nosso.** O upstream é uma fonte de mudanças candidatas, nunca um destino automático.
2. **Revisão humana obrigatória.** Nada entra no `main` sem passar pela revisão de arquitetura.
3. **Mapeamento de renames é explícito.** Todo caminho que difere fica em `docs/UPSTREAM-MAP.toml`.
4. **Puxamos apenas o que é nosso.** Não puxamos branding, `docs/brand/`, links `getartcraft.com`,
   Discord, CI da org `storytold`, nem o packaging deles.
5. **Contribuímos de volta.** Bug compartilhado vira Issue/PR no `storytold/gridcraft` (em inglês),
   registrado em `docs/UPSTREAM-CONTRIBUTIONS.md` se/quando existir.
6. **Independência de conteúdo.** Feature que baixa conteúdo do servidor deles não entra como está:
   ou o conteúdo é embutido/`assets/`, ou o fetch tem checksum fixo, ou a feature fica de fora.
7. **Vendoring por rev.** `libs/craft-libs` fica no rev `c5a1cbf` (com a árvore `vendor/`);
   `libs/egui` `6d518fc`; `libs/winit` `a517b38`; `libs/winprint.rs` `417ac3c`. Um sync que precise
   de outro rev de craft-libs é decisão própria (lição §43: árvore no rev da maioria + crates
   pinados sobrepostos).
8. **Política de fontes é nossa.** `crates/ui-egui/src/craft_fonts_policy.rs` barra Adobe, Source
   Han/Sans/Serif e Noto CJK do craft-fonts mesmo que o upstream os embuta.

## O que puxar vs o que não puxar

| Categoria | Puxar? | Motivo / exemplo |
|-----------|--------|------------------|
| Correção de segurança (parser XLSX, XLSB, ODS, ZIP) | ✅ Sim | `crates/xlsx/`, `crates/engine/src/io.rs` |
| Correção de corrupção / perda de dados | ✅ Sim | `crates/xlsx/`, `crates/engine/src/io.rs` |
| Crash / travamento | ✅ Sim | engine, UI, calc |
| Performance crítica (recálculo, grade virtual) | ✅ Sim | `crates/calc/`, `crates/ui-egui/src/grid*` |
| Bug de paridade com Excel (fórmula, formato numérico) | ✅ Sim | `crates/formula/`, `crates/functions/`, `crates/numfmt/` |
| Feature de UI / ribbon específica | ⚠️ Caso a caso | avaliar com o rebrand |
| Mudança de nomes / bundle id / marca | ❌ Não | é nosso rebrand; conflito garantido |
| Dependências novas / atualizações | ⚠️ Caso a caso | só via `local-deps/WORKFLOW.md` |
| CI / workflows da org storytold | ❌ Não | nosso CI é próprio (`.github/workflows/`) |
| Packaging | ❌ Não | `packaging/` é do molde da suíte |

## Fluxo de sincronização
1. `git remote add upstream https://github.com/storytold/gridcraft.git` (uma vez) e `git fetch upstream`.
2. `git checkout -b sync/upstream-<YYYYMMDD>` (nunca direto no `main`).
3. Listar os commits novos desde `e581bc95` (ou o último sync registrado) e aplicar por arquivo,
   conforme `docs/UPSTREAM-MAP.toml`.
4. Testar: `cargo build --locked`, `cargo test --locked --no-fail-fast -- --test-threads=1`,
   `cargo clippy --locked --all-targets -- -D warnings`, `cargo xtask ci`.
5. Revisão de arquitetura; merge no `main` só se tudo verde.
6. Registrar aqui: data, faixa de commits, o que entrou, o que ficou de fora e por quê.

## Automação
Pode: `git fetch`, listar commits, tentar `git apply` nos arquivos `strategy = "apply"`, rodar os
gates nos crates afetados, gerar relatório.
Não pode: resolver `Cargo.toml`, `packaging/`, `.github/`; adicionar/atualizar dependência; decidir
se uma feature entra; fazer merge; abrir PR no upstream.

## Registro de syncs

| Data | Upstream | O que entrou | O que ficou de fora |
|------|----------|--------------|---------------------|
| 2026-11-05 | `e581bc95` (v0.16.1) | baseline: árvore inteira importada (Fase A) | `.github/`, `sonar-project.properties`, `.coderabbit.yaml`, `docs/brand/`, packaging de cada SO reescrito no molde da suíte, branding nos README/NOTICE/ATTRIBUTION |

## Decision Ledger

| Data | Decisão | Justificativa |
|------|---------|---------------|
| 2026-11-05 | Sync é manual-revisado, nunca automático no main | Protege o rebrand de regressões silenciosas |
| 2026-11-05 | XLSX continua o formato nativo, sem formato próprio | Compatibilidade com arquivos e ferramentas do upstream e do Excel |
| 2026-11-05 | `docs/UPSTREAM-MAP.toml` é o contrato do que pode ser aplicado por script | Evita que nomes de crate/bundle id do upstream entrem no repo |
| 2026-11-05 | Packaging e CI não vêm do upstream | O molde da suíte (WiX per-machine/per-user, workflows próprios) é a referência |
| 2026-11-05 | Fontes do craft-fonts passam pelo filtro de asset policy | O repo é clean-room: nada de Adobe/Source/Noto CJK embutido |

## craft-update vendored: fork permanente da suíte (fora do sync)

`libs/craft-libs/crates/update` está **fora do escopo** deste sync com o upstream. É um crate nosso, hospedado no `craft-libs` da suíte Local, e um **fork permanente consciente** do `craft-update` upstream — unificado nos 10 apps em 2026-11-13, com um diff de milhares de linhas contra `storytold/craft-libs/crates/update` (MSI context/migrate, pins do launcher, `offers_update`, `desktop_register`, catálogo único de apps). Nenhum diff de sync encaixa nele.

Bugfix e feature vão **direto no vendored de cada app** (e no `craft-libs` quando o fix vale para todos); contribuição ao upstream só via **Issue/PR manual**, em inglês, quando o problema também existir lá — mesma política do `UPSTREAM-CONTRIBUTIONS.md` do local-office. O restante do `craft-libs` segue o fluxo normal deste documento.
