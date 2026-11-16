//! The prompts, localized, with the JSON contract written out. The contract's keys (`title`,
//! `columns`, `rows`, `values`, `formula`) are never translated: the extractor and the engine read
//! them by name.
//!
//! The answer's *data* is in the user's language; formulas are canonical Excel English (the engine
//! stores and evaluates that syntax), so the formula action says so explicitly.

/// One language's prompt set.
pub struct Prompts {
    pub system: &'static str,
    pub answer_in: &'static str,
    pub generate: &'static str,
    pub fill: &'static str,
    pub formula: &'static str,
    /// The classifier: what each column of an existing table holds (no data is generated).
    pub classify: &'static str,
}

/// The prompts for a language code (`pt`, `pt-br`, `es`, `ja`, ...): Portuguese, Spanish and
/// Japanese when we have them, English otherwise (the source language).
pub fn for_lang(lang: &str) -> &'static Prompts {
    match lang.split(['-', '_']).next().unwrap_or("en").to_ascii_lowercase().as_str() {
        "pt" => &PT,
        "es" => &ES,
        "ja" => &JA,
        _ => &EN,
    }
}

/// Fills `{name}` placeholders in one pass. `{{` and `}}` are literal braces (the JSON contract's
/// own braces), and a substituted value is never expanded again, so sheet data that happens to
/// contain braces is safe.
pub fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len() + 64);
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                let mut name = String::new();
                let mut closed = false;
                for n in chars.by_ref() {
                    if n == '}' {
                        closed = true;
                        break;
                    }
                    name.push(n);
                }
                match args.iter().find(|(key, _)| *key == name) {
                    Some((_, value)) if closed => out.push_str(value),
                    _ => {
                        out.push('{');
                        out.push_str(&name);
                        if closed {
                            out.push('}');
                        }
                    }
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// The engine functions the model may use, by their real names (the test below checks every one
/// against `localsheets_functions::lookup`, so the prompt never teaches a function the engine does
/// not have). Kept short on purpose: the list is a hint, not the library.
pub const ENGINE_FUNCTIONS: &str = "SUM, SUMIF, SUMIFS, AVERAGE, AVERAGEIF, COUNT, COUNTA, COUNTIF, COUNTIFS, MIN, MAX, ROUND, ABS, IF, IFERROR, AND, OR, NOT, VLOOKUP, INDEX, MATCH, CONCATENATE, CONCAT, LEN, TRIM, LEFT, RIGHT, MID, UPPER, LOWER, TODAY, NOW, DATE";

static EN: Prompts = Prompts {
    system: "You are a spreadsheet assistant. Answer in {answer_in}. Return only the JSON object \
             described: no prose, no markdown, no code fences.",
    answer_in: "English",
    generate: "Build a spreadsheet about: {topic}\n\
               Write every text in it (headers, items, notes) in {language}.\n\n\
               Return one JSON object with exactly this shape:\n\
               {{\"title\": \"a short sheet title\", \"columns\": [{{\"title\": \"Item\", \"kind\": \"text\"}}, {{\"title\": \"Cost\", \"kind\": \"currency\"}}], \"rows\": [[\"text\", 1], [\"text\", {{\"f\": \"=B2*2\"}}]], \"last_row\": \"total\"}}\n\n\
               Rules:\n\
               - exactly {rows} data rows (not counting the header; a totals row is one of them);\n\
               - every row has one cell per column, in the same order as \"columns\";\n\
               - every column is an object with its \"title\" and its \"kind\": text, currency, percent, date or month;\n\
               - the header is row 1, so the first data row is row 2 (a formula there references row 2);\n\
               - a cell is a JSON value (text, number, true/false) or an object with one key: \
               {{\"v\": value}} is a plain value written as typed (text is never read as a formula), \
               and {{\"f\": \"=B2*2\"}} is a formula the sheet computes;\n\
               - EVERY computed cell MUST be a formula in \"f\": a line total (=B2*C2), a column total \
               (=SUM(B2:B20)), a share of a total (=E2/E$21, the row absolute). Never write a computed number;\n\
               - a totals row is a data row of formulas, and you say so with \"last_row\": \"total\";\n\
               - amounts are positive numbers; a direction or flow column says whether a row is income or \
               expense, so never write a negative amount unless the column itself is a net or a balance;\n\
               - formulas use English function names and a comma between arguments; the engine has: {functions};\n\
               - the content is in {language}.",
    fill: "Fill the column \"{header}\" with {count} cells, continuing the table.\n\n\
           Existing data (header and some rows):\n{context}\n\n\
           Return one JSON object with exactly this shape:\n\
           {{\"values\": [\"value 1\", {{\"f\": \"=B2*2\"}}]}}\n\n\
           Rules:\n\
           - exactly {count} cells, in the order the rows go;\n\
           - same type, style and language as the existing column;\n\
           - a cell is a JSON value, {{\"v\": value}} (written as typed) or {{\"f\": \"=A2+B2\"}} (a formula);\n\
           - a cell computed from other cells MUST be a formula in \"f\"; never write a computed number by hand;\n\
           - formulas use English function names and a comma between arguments; the engine has: {functions}.",
    formula: "Write the spreadsheet formula for cell {cell}.\n\n\
              Question: {question}\n\n\
              Table context (headers and some rows):\n{context}\n\n\
              Return one JSON object with exactly this shape:\n\
              {{\"formula\": \"=SUM(A1:A3)\"}}\n\n\
              Rules:\n\
              - canonical Excel syntax: English function names, A1 references, a comma between arguments;\n\
              - start with \"=\";\n\
              - no prose, no explanation.",
    classify: "Read this table and say what each column holds. Do not rewrite or invent any value.\n\n\
               Range: {range}\nHeader: {header}\nSample rows:\n{sample}\n\n\
               Return one JSON object with exactly this shape:\n\
               {{\"columns\": [\"currency\", \"percent\", \"text\"], \"total_row\": false}}\n\n\
               Rules:\n\
               - one kind per column, in the header's order: text, currency, percent, date or month;\n\
               - \"total_row\" is true only when the last row sums or aggregates the rows above it;\n\
               - no text, no explanation.",
};

static PT: Prompts = Prompts {
    system: "Você é um assistente de planilhas. Responda em {answer_in}. Devolva apenas o objeto \
             JSON descrito: sem prosa, sem markdown, sem cercas de código.",
    answer_in: "português (Brasil)",
    generate: "Monte uma planilha sobre: {topic}\n\
               Escreva todo o texto dela (cabeçalhos, itens, notas) em {language}.\n\n\
               Devolva um objeto JSON exatamente com esta forma:\n\
               {{\"title\": \"um título curto\", \"columns\": [{{\"title\": \"Item\", \"kind\": \"text\"}}, {{\"title\": \"Custo\", \"kind\": \"currency\"}}], \"rows\": [[\"texto\", 1], [\"texto\", {{\"f\": \"=B2*2\"}}]], \"last_row\": \"total\"}}\n\n\
               Regras:\n\
               - exatamente {rows} linhas de dados (sem contar o cabeçalho; uma linha de total conta);\n\
               - cada linha tem uma célula por coluna, na mesma ordem de \"columns\";\n\
               - cada coluna é um objeto com \"title\" e \"kind\": text, currency, percent, date ou month;\n\
               - o cabeçalho é a linha 1, então a primeira linha de dados é a 2 (uma fórmula ali referencia a linha 2);\n\
               - uma célula é um valor JSON (texto, número, true/false) ou um objeto com uma chave: \
               {{\"v\": valor}} é um valor simples escrito como está (texto nunca é lido como fórmula) e \
               {{\"f\": \"=B2*2\"}} é uma fórmula que a planilha calcula;\n\
               - TODA célula calculada PRECISA ser uma fórmula em \"f\": total da linha (=B2*C2), total da coluna \
               (=SUM(B2:B20)), share de um total (=E2/E$21, com a linha absoluta). Nunca escreva um número calculado;\n\
               - uma linha de total é uma linha de dados com fórmulas, e você diz isso com \"last_row\": \"total\";\n\
               - valores de dinheiro são positivos; uma coluna de direção/fluxo diz se a linha é entrada ou saída, \
               então nunca escreva um valor negativo, a menos que a coluna em si seja líquida ou saldo;\n\
               - fórmulas usam nomes de função em inglês e vírgula entre argumentos; o motor tem: {functions};\n\
               - o conteúdo é em {language}.",
    fill: "Preencha a coluna \"{header}\" com {count} células, continuando a tabela.\n\n\
           Dados existentes (cabeçalho e algumas linhas):\n{context}\n\n\
           Devolva um objeto JSON exatamente com esta forma:\n\
           {{\"values\": [\"valor 1\", {{\"f\": \"=B2*2\"}}]}}\n\n\
           Regras:\n\
           - exatamente {count} células, na ordem das linhas;\n\
           - o mesmo tipo, estilo e idioma da coluna existente;\n\
           - uma célula é um valor JSON, {{\"v\": valor}} (escrito como está) ou {{\"f\": \"=A2+B2\"}} (uma fórmula);\n\
           - uma célula calculada a partir de outras PRECISA ser uma fórmula em \"f\"; nunca escreva um número calculado à mão;\n\
           - fórmulas usam nomes de função em inglês e vírgula entre argumentos; o motor tem: {functions}.",
    formula: "Escreva a fórmula da planilha para a célula {cell}.\n\n\
              Pergunta: {question}\n\n\
              Contexto da tabela (cabeçalhos e algumas linhas):\n{context}\n\n\
              Devolva um objeto JSON exatamente com esta forma:\n\
              {{\"formula\": \"=SUM(A1:A3)\"}}\n\n\
              Regras:\n\
              - sintaxe canônica do Excel: nomes de função em inglês, referências A1, vírgula entre argumentos;\n\
              - comece com \"=\";\n\
              - sem prosa, sem explicação.",
    classify: "Leia esta tabela e diga o que cada coluna contém. Não reescreva nem invente nenhum valor.\n\n\
               Intervalo: {range}\nCabeçalho: {header}\nAmostra de linhas:\n{sample}\n\n\
               Devolva um objeto JSON exatamente com esta forma:\n\
               {{\"columns\": [\"currency\", \"percent\", \"text\"], \"total_row\": false}}\n\n\
               Regras:\n\
               - um tipo por coluna, na ordem do cabeçalho: text, currency, percent, date ou month;\n\
               - \"total_row\" é true só quando a última linha soma ou agrega as linhas de cima;\n\
               - sem texto, sem explicação.",
};

static ES: Prompts = Prompts {
    system: "Eres un asistente de hojas de cálculo. Responde en {answer_in}. Devuelve solo el objeto \
             JSON descrito: sin prosa, sin markdown, sin bloques de código.",
    answer_in: "español",
    generate: "Crea una hoja de cálculo sobre: {topic}\n\
               Escribe todo el texto (encabezados, elementos, notas) en {language}.\n\n\
               Devuelve un objeto JSON exactamente con esta forma:\n\
               {{\"title\": \"un título corto\", \"columns\": [{{\"title\": \"Artículo\", \"kind\": \"text\"}}, {{\"title\": \"Coste\", \"kind\": \"currency\"}}], \"rows\": [[\"texto\", 1], [\"texto\", {{\"f\": \"=B2*2\"}}]], \"last_row\": \"total\"}}\n\n\
               Reglas:\n\
               - exactamente {rows} filas de datos (sin contar el encabezado; una fila de total cuenta);\n\
               - cada fila tiene una celda por columna, en el mismo orden que \"columns\";\n\
               - cada columna es un objeto con \"title\" y \"kind\": text, currency, percent, date o month;\n\
               - el encabezado es la fila 1, así que la primera fila de datos es la 2 (una fórmula allí referencia la fila 2);\n\
               - una celda es un valor JSON (texto, número, true/false) o un objeto con una clave: \
               {{\"v\": valor}} es un valor simple escrito tal cual (el texto nunca se lee como fórmula) y \
               {{\"f\": \"=B2*2\"}} es una fórmula que la hoja calcula;\n\
               - TODA celda calculada DEBE ser una fórmula en \"f\": total de la fila (=B2*C2), total de la columna \
               (=SUM(B2:B20)), cuota de un total (=E2/E$21, con la fila absoluta). Nunca escribas un número calculado;\n\
               - una fila de total es una fila de datos con fórmulas, y lo dices con \"last_row\": \"total\";\n\
               - los importes son positivos; una columna de dirección/flujo dice si la fila es entrada o salida, \
               así que nunca escribas un importe negativo salvo que la columna sea líquida o un saldo;\n\
               - las fórmulas usan nombres de función en inglés y coma entre argumentos; el motor tiene: {functions};\n\
               - el contenido está en {language}.",
    fill: "Rellena la columna \"{header}\" con {count} celdas, continuando la tabla.\n\n\
           Datos existentes (encabezado y algunas filas):\n{context}\n\n\
           Devuelve un objeto JSON exactamente con esta forma:\n\
           {{\"values\": [\"valor 1\", {{\"f\": \"=B2*2\"}}]}}\n\n\
           Reglas:\n\
           - exactamente {count} celdas, en el orden de las filas;\n\
           - el mismo tipo, estilo e idioma que la columna existente;\n\
           - una celda es un valor JSON, {{\"v\": valor}} (escrito tal cual) o {{\"f\": \"=A2+B2\"}} (una fórmula);\n\
           - una celda calculada a partir de otras DEBE ser una fórmula en \"f\"; nunca escribas un número calculado a mano;\n\
           - las fórmulas usan nombres de función en inglés y coma entre argumentos; el motor tiene: {functions}.",
    formula: "Escribe la fórmula de la hoja de cálculo para la celda {cell}.\n\n\
              Pregunta: {question}\n\n\
              Contexto de la tabla (encabezados y algunas filas):\n{context}\n\n\
              Devuelve un objeto JSON exactamente con esta forma:\n\
              {{\"formula\": \"=SUM(A1:A3)\"}}\n\n\
              Reglas:\n\
              - sintaxis canónica de Excel: nombres de función en inglés, referencias A1, coma entre argumentos;\n\
              - empieza por \"=\";\n\
              - sin prosa, sin explicación.",
    classify: "Lee esta tabla y di qué contiene cada columna. No reescribas ni inventes ningún valor.\n\n\
               Rango: {range}\nEncabezado: {header}\nFilas de muestra:\n{sample}\n\n\
               Devuelve un objeto JSON exactamente con esta forma:\n\
               {{\"columns\": [\"currency\", \"percent\", \"text\"], \"total_row\": false}}\n\n\
               Reglas:\n\
               - un tipo por columna, en el orden del encabezado: text, currency, percent, date o month;\n\
               - \"total_row\" es true solo cuando la última fila suma o agrega las de arriba;\n\
               - sin texto, sin explicación.",
};

static JA: Prompts = Prompts {
    system: "あなたは表計算アシスタントです。{answer_in}で答えてください。説明した JSON オブジェクトだけを返してください: \
             文章も markdown もコードフェンスも不要です。",
    answer_in: "日本語",
    generate: "{topic} についての表計算を作成してください。\n\
               {language} で書いてください (見出し・項目・メモのすべてのテキスト)。\n\n\
               次の形の JSON オブジェクトを 1 つだけ返してください:\n\
               {{\"title\": \"短いシート名\", \"columns\": [{{\"title\": \"品目\", \"kind\": \"text\"}}, {{\"title\": \"費用\", \"kind\": \"currency\"}}], \"rows\": [[\"テキスト\", 1], [\"テキスト\", {{\"f\": \"=B2*2\"}}]], \"last_row\": \"total\"}}\n\n\
               ルール:\n\
               - データ行はちょうど {rows} 行 (見出しは含めません。合計行は 1 行と数えます)。\n\
               - 各行は \"columns\" と同じ順で、列ごとに 1 つのセル。\n\
               - 各列は \"title\" と \"kind\" を持つオブジェクト: text、currency、percent、date、month。\n\
               - 見出しは 1 行目なので、最初のデータ行は 2 行目です (そこに書く数式は 2 行目を参照します)。\n\
               - セルは JSON の値 (テキスト・数値・true/false) か、キーを 1 つ持つオブジェクトです: \
               {{\"v\": 値}} はそのまま書く値 (テキストが数式として読まれることはありません)、 \
               {{\"f\": \"=B2*2\"}} はシートが計算する数式。\n\
               - 計算されるセルはすべて \"f\" の数式にしてください: 行の合計 (=B2*C2)、列の合計 \
               (=SUM(B2:B20))、合計に対する割合 (=E2/E$21、行は絶対参照)。計算した数値を書いてはいけません。\n\
               - 合計行もデータ行で、数式で書きます。その場合は \"last_row\": \"total\" を付けます。\n\
               - 金額は正の数です。収入か支出かは方向/フローの列が示すので、列自体が純額や残高でない限り \
               負の金額は書きません。\n\
               - 数式の関数名は英語、引数はカンマ区切り。エンジンにある関数: {functions}。\n\
               - 内容は {language}。",
    fill: "列 \"{header}\" に {count} 個のセルを入れ、表を続けてください。\n\n\
           既存のデータ (見出しと数行):\n{context}\n\n\
           次の形の JSON オブジェクトを 1 つだけ返してください:\n\
           {{\"values\": [\"値 1\", {{\"f\": \"=B2*2\"}}]}}\n\n\
           ルール:\n\
           - セルはちょうど {count} 個、行の順に。\n\
           - 既存の列と同じ型・書式・言語。\n\
           - セルは JSON の値、{{\"v\": 値}} (そのまま書く)、または {{\"f\": \"=A2+B2\"}} (数式)。\n\
           - 他のセルから計算されるセルは必ず \"f\" の数式。計算した数値を手で書いてはいけません。\n\
           - 数式の関数名は英語、引数はカンマ区切り。エンジンにある関数: {functions}。",
    formula: "セル {cell} の表計算の数式を書いてください。\n\n\
              質問: {question}\n\n\
              表の文脈 (見出しと数行):\n{context}\n\n\
              次の形の JSON オブジェクトを 1 つだけ返してください:\n\
              {{\"formula\": \"=SUM(A1:A3)\"}}\n\n\
              ルール:\n\
              - 標準の Excel 構文: 関数名は英語、A1 参照、引数はカンマ区切り。\n\
              - \"=\" で始める。\n\
              - 文章・説明は不要。",
    classify: "この表を読み、各列が何を表すかだけを答えてください。値を書き換えたり作ったりしてはいけません。\n\n\
               範囲: {range}\n見出し: {header}\nサンプル行:\n{sample}\n\n\
               次の形の JSON オブジェクトを 1 つだけ返してください:\n\
               {{\"columns\": [\"currency\", \"percent\", \"text\"], \"total_row\": false}}\n\n\
               ルール:\n\
               - 見出しの順に、列ごとに 1 つの種類: text、currency、percent、date、month。\n\
               - \"total_row\" は最終行が上の行を合計・集計しているときだけ true。\n\
               - 文章・説明は不要。",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_the_json_contract() {
        for prompts in [&EN, &PT, &ES, &JA] {
            assert!(prompts.generate.contains("\"columns\""));
            assert!(prompts.generate.contains("\"rows\""));
            assert!(prompts.fill.contains("\"values\""));
            assert!(prompts.formula.contains("\"formula\""));
        }
    }

    #[test]
    fn the_language_pick_falls_back_to_english() {
        assert_eq!(for_lang("pt-br").answer_in, "português (Brasil)");
        assert_eq!(for_lang("pt").answer_in, "português (Brasil)");
        assert_eq!(for_lang("es").answer_in, "español");
        assert_eq!(for_lang("ja").answer_in, "日本語");
        assert_eq!(for_lang("de").answer_in, "English");
        assert_eq!(for_lang("").answer_in, "English");
    }

    #[test]
    fn placeholders_are_filled_once() {
        let text = fill("a {topic} b {rows} c", &[("topic", "{rows}"), ("rows", "3")]);
        assert_eq!(text, "a {rows} b 3 c", "a substituted value is not expanded again");
    }

    /// `{{` and `}}` are the JSON contract's own braces: they reach the model as single braces.
    #[test]
    fn escaped_braces_become_one() {
        assert_eq!(fill("a {{b}} c", &[]), "a {b} c");
        assert_eq!(fill("{{topic}} {topic}", &[("topic", "x")]), "{topic} x");
        assert_eq!(fill("{\"a\": 1}", &[]), "{\"a\": 1}", "a single-brace group that is not a placeholder stays");
    }

    /// The JSON object's braces are not placeholders: they survive the fill untouched.
    #[test]
    fn the_json_contract_braces_survive() {
        let text = fill(EN.formula, &[("cell", "B2"), ("question", "total"), ("context", "A1: 10"), ("answer_in", "English")]);
        assert!(text.contains("{\"formula\": \"=SUM(A1:A3)\"}"), "{text}");
        assert!(!text.contains("}}"), "no doubled braces reach the model: {text}");
        assert!(text.contains("B2") && text.contains("total"));
    }

    /// The prompt may only teach functions the engine has: an invented name is refused by the
    /// window, but it should never be suggested in the first place.
    #[test]
    fn the_functions_the_prompt_teaches_exist_in_the_engine() {
        let names: Vec<&str> = ENGINE_FUNCTIONS.split(',').map(str::trim).collect();
        assert!(names.len() >= 20, "the list is a real hint: {names:?}");
        let missing: Vec<&str> = names.into_iter().filter(|name| localsheets_functions::lookup(name).is_none()).collect();
        assert!(missing.is_empty(), "not engine functions: {missing:?}");
    }

    /// The two cell channels are in every generate prompt: `v` (written as typed) and `f` (a
    /// formula). That is the fix for a model inventing computed numbers.
    #[test]
    fn every_language_teaches_the_formula_channel() {
        for prompts in [&EN, &PT, &ES, &JA] {
            assert!(prompts.generate.contains("\"f\""), "no formula channel in a generate prompt");
            assert!(prompts.generate.contains("\"v\""));
            assert!(prompts.generate.contains("{functions}"), "the function hint is missing");
            assert!(prompts.generate.contains("\"kind\""), "no column kind in a generate prompt");
            assert!(prompts.generate.contains("\"last_row\""));
            assert!(prompts.fill.contains("\"f\""));
            assert!(prompts.classify.contains("\"columns\"") && prompts.classify.contains("\"total_row\""));
        }
    }
}
