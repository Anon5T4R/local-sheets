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
    generate: "Build a spreadsheet about: {topic}\n\n\
               Return one JSON object with exactly this shape:\n\
               {{\"title\": \"a short sheet title\", \"columns\": [\"Header 1\", \"Header 2\"], \"rows\": [[\"text\", 1], [\"text\", {{\"f\": \"=B2*2\"}}]]}}\n\n\
               Rules:\n\
               - exactly {rows} data rows;\n\
               - every row has one cell per column, in the same order as \"columns\";\n\
               - the header is row 1, so the first data row is row 2 (a formula there references row 2);\n\
               - a cell is a JSON value (text, number, true/false) or an object with one key: \
               {{\"v\": value}} is a plain value written as typed (text is never read as a formula), \
               and {{\"f\": \"=B2*2\"}} is a formula the sheet computes;\n\
               - a cell computed from other cells MUST be a formula in \"f\" (for example =B2*C2, =SUM(B2:B30), \
               =SUMIF(A2:A30,\"Rent\",B2:B30)); never write a computed number by hand;\n\
               - formulas use English function names and a comma between arguments; the engine has: {functions};\n\
               - the content is in {answer_in}.",
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
};

static PT: Prompts = Prompts {
    system: "Você é um assistente de planilhas. Responda em {answer_in}. Devolva apenas o objeto \
             JSON descrito: sem prosa, sem markdown, sem cercas de código.",
    answer_in: "português (Brasil)",
    generate: "Monte uma planilha sobre: {topic}\n\n\
               Devolva um objeto JSON exatamente com esta forma:\n\
               {{\"title\": \"um título curto\", \"columns\": [\"Cabeçalho 1\", \"Cabeçalho 2\"], \"rows\": [[\"texto\", 1], [\"texto\", {{\"f\": \"=B2*2\"}}]]}}\n\n\
               Regras:\n\
               - exatamente {rows} linhas de dados;\n\
               - cada linha tem uma célula por coluna, na mesma ordem de \"columns\";\n\
               - o cabeçalho é a linha 1, então a primeira linha de dados é a 2 (uma fórmula ali referencia a linha 2);\n\
               - uma célula é um valor JSON (texto, número, true/false) ou um objeto com uma chave: \
               {{\"v\": valor}} é um valor simples escrito como está (texto nunca é lido como fórmula) e \
               {{\"f\": \"=B2*2\"}} é uma fórmula que a planilha calcula;\n\
               - uma célula calculada a partir de outras células PRECISA ser uma fórmula em \"f\" (por exemplo =B2*C2, \
               =SUM(B2:B30), =SUMIF(A2:A30,\"Aluguel\",B2:B30)); nunca escreva um número calculado à mão;\n\
               - fórmulas usam nomes de função em inglês e vírgula entre argumentos; o motor tem: {functions};\n\
               - o conteúdo é em {answer_in}.",
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
};

static ES: Prompts = Prompts {
    system: "Eres un asistente de hojas de cálculo. Responde en {answer_in}. Devuelve solo el objeto \
             JSON descrito: sin prosa, sin markdown, sin bloques de código.",
    answer_in: "español",
    generate: "Crea una hoja de cálculo sobre: {topic}\n\n\
               Devuelve un objeto JSON exactamente con esta forma:\n\
               {{\"title\": \"un título corto\", \"columns\": [\"Encabezado 1\", \"Encabezado 2\"], \"rows\": [[\"texto\", 1], [\"texto\", {{\"f\": \"=B2*2\"}}]]}}\n\n\
               Reglas:\n\
               - exactamente {rows} filas de datos;\n\
               - cada fila tiene una celda por columna, en el mismo orden que \"columns\";\n\
               - el encabezado es la fila 1, así que la primera fila de datos es la 2 (una fórmula allí referencia la fila 2);\n\
               - una celda es un valor JSON (texto, número, true/false) o un objeto con una clave: \
               {{\"v\": valor}} es un valor simple escrito tal cual (el texto nunca se lee como fórmula) y \
               {{\"f\": \"=B2*2\"}} es una fórmula que la hoja calcula;\n\
               - una celda calculada a partir de otras celdas DEBE ser una fórmula en \"f\" (por ejemplo =B2*C2, \
               =SUM(B2:B30), =SUMIF(A2:A30,\"Alquiler\",B2:B30)); nunca escribas un número calculado a mano;\n\
               - las fórmulas usan nombres de función en inglés y coma entre argumentos; el motor tiene: {functions};\n\
               - el contenido está en {answer_in}.",
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
};

static JA: Prompts = Prompts {
    system: "あなたは表計算アシスタントです。{answer_in}で答えてください。説明した JSON オブジェクトだけを返してください: \
             文章も markdown もコードフェンスも不要です。",
    answer_in: "日本語",
    generate: "{topic} についての表計算を作成してください。\n\n\
               次の形の JSON オブジェクトを 1 つだけ返してください:\n\
               {{\"title\": \"短いシート名\", \"columns\": [\"見出し 1\", \"見出し 2\"], \"rows\": [[\"テキスト\", 1], [\"テキスト\", {{\"f\": \"=B2*2\"}}]]}}\n\n\
               ルール:\n\
               - データ行はちょうど {rows} 行。\n\
               - 各行は \"columns\" と同じ順で、列ごとに 1 つのセル。\n\
               - 見出しは 1 行目なので、最初のデータ行は 2 行目です (そこに書く数式は 2 行目を参照します)。\n\
               - セルは JSON の値 (テキスト・数値・true/false) か、キーを 1 つ持つオブジェクトです: \
               {{\"v\": 値}} はそのまま書く値 (テキストが数式として読まれることはありません)、 \
               {{\"f\": \"=B2*2\"}} はシートが計算する数式。\n\
               - 他のセルから計算されるセルは必ず \"f\" の数式にしてください (例: =B2*C2、=SUM(B2:B30)、\
               =SUMIF(A2:A30,\"家賃\",B2:B30))。計算した数値を手で書いてはいけません。\n\
               - 数式の関数名は英語、引数はカンマ区切り。エンジンにある関数: {functions}。\n\
               - 内容は {answer_in}。",
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
            assert!(prompts.fill.contains("\"f\""));
        }
    }
}
