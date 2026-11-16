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

/// Fills `{name}` placeholders in one pass. Values are never re-expanded, so sheet data that
/// happens to contain braces is safe.
pub fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len() + 64);
    let mut rest = template;
    while let Some(at) = rest.find('{') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let Some(end) = after.find('}') else {
            out.push('{');
            rest = after;
            continue;
        };
        let name = &after[..end];
        match args.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

static EN: Prompts = Prompts {
    system: "You are a spreadsheet assistant. Answer in {answer_in}. Return only the JSON object \
             described: no prose, no markdown, no code fences.",
    answer_in: "English",
    generate: "Build a spreadsheet about: {topic}\n\n\
               Return one JSON object with exactly this shape:\n\
               {{\"title\": \"a short sheet title\", \"columns\": [\"Header 1\", \"Header 2\"], \"rows\": [[\"value\", 1], [\"value\", 2]]}}\n\n\
               Rules:\n\
               - exactly {rows} data rows;\n\
               - every row has one value per column, in the same order as \"columns\";\n\
               - numbers are JSON numbers, text is JSON strings; never a formula;\n\
               - the content is in {answer_in}.",
    fill: "Fill the column \"{header}\" with {count} values, continuing the table.\n\n\
           Existing data (header and some rows):\n{context}\n\n\
           Return one JSON object with exactly this shape:\n\
           {{\"values\": [\"value 1\", \"value 2\"]}}\n\n\
           Rules:\n\
           - exactly {count} values, in the order the rows go;\n\
           - same type, style and language as the existing column;\n\
           - numbers are JSON numbers, text is JSON strings; never a formula.",
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
               {{\"title\": \"um título curto\", \"columns\": [\"Cabeçalho 1\", \"Cabeçalho 2\"], \"rows\": [[\"valor\", 1], [\"valor\", 2]]}}\n\n\
               Regras:\n\
               - exatamente {rows} linhas de dados;\n\
               - cada linha tem um valor por coluna, na mesma ordem de \"columns\";\n\
               - números são números JSON, texto é string JSON; nunca uma fórmula;\n\
               - o conteúdo é em {answer_in}.",
    fill: "Preencha a coluna \"{header}\" com {count} valores, continuando a tabela.\n\n\
           Dados existentes (cabeçalho e algumas linhas):\n{context}\n\n\
           Devolva um objeto JSON exatamente com esta forma:\n\
           {{\"values\": [\"valor 1\", \"valor 2\"]}}\n\n\
           Regras:\n\
           - exatamente {count} valores, na ordem das linhas;\n\
           - o mesmo tipo, estilo e idioma da coluna existente;\n\
           - números são números JSON, texto é string JSON; nunca uma fórmula.",
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
               {{\"title\": \"un título corto\", \"columns\": [\"Encabezado 1\", \"Encabezado 2\"], \"rows\": [[\"valor\", 1], [\"valor\", 2]]}}\n\n\
               Reglas:\n\
               - exactamente {rows} filas de datos;\n\
               - cada fila tiene un valor por columna, en el mismo orden que \"columns\";\n\
               - los números son números JSON, el texto es string JSON; nunca una fórmula;\n\
               - el contenido está en {answer_in}.",
    fill: "Rellena la columna \"{header}\" con {count} valores, continuando la tabla.\n\n\
           Datos existentes (encabezado y algunas filas):\n{context}\n\n\
           Devuelve un objeto JSON exactamente con esta forma:\n\
           {{\"values\": [\"valor 1\", \"valor 2\"]}}\n\n\
           Reglas:\n\
           - exactamente {count} valores, en el orden de las filas;\n\
           - el mismo tipo, estilo e idioma que la columna existente;\n\
           - los números son números JSON, el texto es string JSON; nunca una fórmula.",
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
               {{\"title\": \"短いシート名\", \"columns\": [\"見出し 1\", \"見出し 2\"], \"rows\": [[\"値\", 1], [\"値\", 2]]}}\n\n\
               ルール:\n\
               - データ行はちょうど {rows} 行。\n\
               - 各行は \"columns\" と同じ順で、列ごとに 1 つの値。\n\
               - 数値は JSON の数値、テキストは JSON の文字列。数式は不可。\n\
               - 内容は {answer_in}。",
    fill: "列 \"{header}\" に {count} 個の値を入れ、表を続けてください。\n\n\
           既存のデータ (見出しと数行):\n{context}\n\n\
           次の形の JSON オブジェクトを 1 つだけ返してください:\n\
           {{\"values\": [\"値 1\", \"値 2\"]}}\n\n\
           ルール:\n\
           - 値はちょうど {count} 個、行の順に。\n\
           - 既存の列と同じ型・書式・言語。\n\
           - 数値は JSON の数値、テキストは JSON の文字列。数式は不可。",
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

    /// The JSON object's braces are not placeholders: they survive the fill untouched.
    #[test]
    fn the_json_contract_braces_survive() {
        let text = fill(EN.formula, &[("cell", "B2"), ("question", "total"), ("context", "A1: 10"), ("answer_in", "English")]);
        assert!(text.contains("{\"formula\": \"=SUM(A1:A3)\"}"), "{text}");
        assert!(text.contains("B2") && text.contains("total"));
    }
}
