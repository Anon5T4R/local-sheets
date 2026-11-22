# ROADMAP — Local Sheets

Fork do gridcraft (gridcraft v0.16.1), rebrandado para a suíte Local. Rust puro + egui, desktop.
**XLSX é o formato nativo** — o fork não inventa formato próprio, então arquivos e ferramentas do
upstream e do Excel seguem compatíveis. Como é construído: `docs/architecture.md`; decisões e o porquê:
`docs/decisions.md`.

**Estágio: alpha.** Abre, edita e salva planilhas de uso real; o que abaixo falta é o caminho até
"pronto para trabalho de verdade".

## O que falta (por área, sem ordem de dificuldade)

- **Fidelidade do XLSX.** O round-trip é testado com arquivos nossos; falta passar por um corpus de
  arquivos de terceiros e por um Excel real (PivotTables, gráficos, formatação condicional,
  validação, matrizes dinâmicas, tabelas): um prompt de reparo ou perda silenciosa é o que mais
  custa. Conteúdo não modelado (slicers, timelines, modelo de dados, `extLst`) passa intacto pelo
  pass-through, mas nunca foi conferido no Excel.
- **Planilhas grandes.** Leitura sem streaming e recálculo em uma thread só.
- **Gráficos.** Modelo raso: escala de eixo, opções de rótulo, barras de erro e formatação por ponto.
- **Texto não latino.** Sem shaping (árabe, devanagari, thai) nem bidi; a exportação em PDF usa só
  as 14 fontes padrão.
- **`.xls` e arquivos protegidos.** `.xls` (BIFF) é lido só em parte e nunca escrito; `.xlsx`
  criptografado abre com a senha, mas é salvo sem ela.
- **PivotTables.** Sem slicers, itens calculados e PivotCharts; nunca conferidas no Excel.
- **Teclado.** Keytips do ribbon ainda parciais e menos atalhos que o Excel.
- **Localização.** A interface oferece 19 idiomas, mas faltam strings em todos — de 78 no japonês a
  mais de 800 no turco; `cargo xtask i18n-report` lista as que faltam por idioma.

## Formatos

| Formato | Ler | Gravar |
| --- | --- | --- |
| `.xlsx` (nativo) | sim, com estilos, temas, fórmulas, tabelas, formatação condicional, validação, comentários, links, gráficos, imagens, sparklines, pivôs e ajustes de impressão | sim |
| `.ods` | sim: dados, números, texto, booleanos, datas, células/linhas repetidas e mesclagens (fórmulas viram o último resultado salvo) | não |
| `.xlsb` | sim: nomes, visibilidade, números, texto, booleanos, erros e mesclagens (fórmulas viram o último resultado salvo) | não |
| `.xls` (BIFF8) | parcial: valores, fórmulas, formatos numéricos, mesclagens, tamanhos, nomes e formatação de célula; tema exato, gráficos, imagens, comentários, formatação condicional e validação não | não |
| `.csv`/`.tsv`/`.txt` | sim (inclusive `csvAsText`) | sim |
| `.xlsx` com senha | sim (Agile e Standard) | sem senha |

Limites conhecidos de importação: `docs/ods-import.md`, `docs/xlsb-import.md`; imagens em células:
`docs/cell-pictures.md`.

## Referências

`docs/architecture.md` · `docs/decisions.md` · `docs/cli.md` · `docs/control-protocol.md` ·
`docs/mcp.md` · `docs/macros.md` · `docs/releasing.md` · `docs/UPSTREAM-SYNC.md`
