# AGENTS.md — Local Sheets

Fork independente do gridcraft (GridCraft, ArtCraft), rebrandado para a suíte Local do João.
Rust puro + egui. Sem conta, sem telemetria, sem dependência de servidores de terceiros.

## Regras
- **Dependências:** toda dep que entra/sai/atualiza exige `Cargo.lock` commitado +
  linha no inventário do repo `Anon5T4R/local-deps` (regra completa em
  `local-deps/WORKFLOW.md`). PR sem isso não mergea.
- **Sem fontes git:** nenhuma dependência pode apontar para `git+` (o CI valida).
  Tudo vem do crates.io (pinado pelo lock) ou de `libs/`/`vendor/` (vendored nos
  revs exatos que o upstream pina).
- **Commits:** português, sem Co-Authored-By.
- **Release:** tag `vX.Y.Z` → o Actions builda Windows + Linux e publica. Mínimo local.
- **Disco local:** nunca deixe `target/` para trás — `cargo clean` após uso local;
  builds pesados são do CI.

## Estrutura
- `crates/` — engine e módulos (`localsheets-*`), em camadas L0–L7 (`cargo xtask layers`)
- `apps/localsheets` — app desktop + control channel; `apps/localsheets-cli` — CLI/MCP;
  `apps/localsheets-web` — wasm
- `libs/` — forks vendored: `craft-libs` (c5a1cbf, com `vendor/`), `egui` (6d518fc),
  `winit` (a517b38) e `winprint.rs`
- `packaging/` — Windows (WiX/MSI per-machine e per-user), Linux (AppImage/deb/rpm/tar/Flatpak),
  macOS, FreeBSD, web
- `xtask/` — tarefas de build/CI (`cargo xtask ci`, `layers`, `wasm`, `parity`, `keytips`, `assets`)

## Build
```
cargo build --locked          # debug
cargo test --locked           # testes (no Windows: -- --test-threads=1, testes de UI)
packaging/windows/package.ps1 # MSI + portable (Windows)
packaging/linux/package.sh    # AppImage/deb/rpm/tar (Linux)
```

Regras técnicas do código (never-crash, camadas, tudo é comando, wasm, política de assets):
`docs/architecture.md`; o formato nativo e a compatibilidade com o upstream estão em
`docs/file-format-parity.md`. Os nós cegos do rebrand, se algum, ficam em `docs/gaps.md`.
