use pest::Parser;
use pest_derive::Parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct RazomyParser;

// =============================================================================
// 1. ГОМОИКОННОЕ AST: ЛИТЕРАЛЫ И ТОКЕНЫ (V6.0)
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Byte(u8),
    String(String),    // "..." (multiline with {expr} interpolation and \ escapes)
    RawString(String), // r"...", r1"..."1, r2"..."2 (multiline, zero escapes, zero interpolation)
    Symbol(String),    // '...' (exact constant tag without interpolation)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Имена, типы, слитные пути и индексы (`name`, `User.name`, `MyGarageCar.-1`, `_.age`, `_1.0`)
    Path(String),
    /// Примитивные и строковые литералы
    Lit(Literal),
    /// Операторы (`.`, `.>`, `.&`, `.?`, `.|`, `&`, `|`, `/`, `+`, `==`, `!`, и т.д.)
    Op(String),
    /// Диапазоны (`1..10`, `"a".."z"`, `1..limit`, `(a + 1)..(b - 1)`)
    Range(Box<Token>, Box<Token>),
    /// Три гомоиконные скобки:
    /// '{' (Form), '[' (Stream), '(' (Tuple/Expression через пробел),
    /// ')' (маркер слитного списка аргументов `fn(args)` без пробела или перегрузки оператора `+ (other)`)
    Group(char, Vec<Sentence>),
}

pub type Sentence = Vec<Token>;

// =============================================================================
// 2. КОНВЕРТАЦИЯ PEST PAIRS В ЧИСТОЕ AST
// =============================================================================

fn strip_raw_string(s: &str) -> String {
    let mut s = s.to_string();

    if s.starts_with('r') {
        // Собираем все цифры сразу после 'r' (может быть пусто, если просто r"...")
        let digits: String = s[1..].chars().take_while(|c| c.is_ascii_digit()).collect();

        let prefix = format!("r{digits}\"");
        let suffix = format!("\"{digits}");

        // Проверяем, что строка действительно обрамлена этим префиксом и суффиксом
        // и что они не пересекаются (например, на строке вроде r1"1)
        if s.len() >= prefix.len() + suffix.len() && s.starts_with(&prefix) && s.ends_with(&suffix)
        {
            // Удаляем с конца через pop (цифры + кавычка)
            for _ in 0..suffix.len() {
                s.pop();
            }
            // Удаляем с начала ('r' + цифры + кавычка)
            s.drain(..prefix.len());
        }
    }

    s
}

fn extract_raw_string_content(s: &str) -> String {
    strip_raw_string(s)
}

pub fn parse_token(pair: pest::iterators::Pair<Rule>) -> Token {
    match pair.as_rule() {
        Rule::brace_group => Token::Group('{', parse_sentences(pair)),
        Rule::bracket_group => Token::Group('[', parse_sentences(pair)),
        Rule::paren_group => Token::Group('(', parse_sentences(pair)),

        Rule::string_lit => {
            let s = pair.as_str();
            Token::Lit(Literal::String(s[1..s.len() - 1].to_string()))
        }
        Rule::raw_string_lit => {
            let s = pair.as_str();
            Token::Lit(Literal::RawString(extract_raw_string_content(s)))
        }
        Rule::symbol_lit => {
            let s = pair.as_str();
            Token::Lit(Literal::Symbol(s[1..s.len() - 1].to_string()))
        }

        Rule::hex_byte_lit => {
            let s = &pair.as_str()[2..];
            let val = u8::from_str_radix(s, 16).unwrap_or(0);
            Token::Lit(Literal::Byte(val))
        }

        Rule::number_lit => {
            let s = pair.as_str();
            if s.contains('.') {
                Token::Lit(Literal::Float(s.parse().unwrap()))
            } else {
                Token::Lit(Literal::Int(s.parse().unwrap()))
            }
        }

        Rule::operator => Token::Op(pair.as_str().to_string()),

        Rule::callable_op => Token::Path(pair.as_str().to_string()),

        Rule::path | Rule::type_ident => {
            let s = pair.as_str();
            match s {
                "true" => Token::Lit(Literal::Bool(true)),
                "false" => Token::Lit(Literal::Bool(false)),
                _ => Token::Path(s.to_string()),
            }
        }

        _ => unreachable!("Неизвестное правило: {:?}", pair.as_rule()),
    }
}

pub fn parse_sentences(pair: pest::iterators::Pair<Rule>) -> Vec<Sentence> {
    let mut sentences = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::sentence {
            let mut raw_tokens: Vec<Token> = Vec::new();
            let mut prev_end: Option<usize> = None;
            let mut prev_was_callable_op = false;

            for tok_pair in inner.into_inner() {
                let rule = tok_pair.as_rule();
                let span = tok_pair.as_span();
                let is_tight = prev_end == Some(span.start());

                // 1. Type"..." — 100% синтаксический сахар для вызова конструктора Type("...")!
                if rule == Rule::tagged_string_lit {
                    let mut inner_tagged = tok_pair.into_inner();
                    let type_pair = inner_tagged.next().unwrap();
                    let str_pair = inner_tagged.next().unwrap();
                    let type_name = type_pair.as_str().to_string();
                    let str_tok = parse_token(str_pair);

                    raw_tokens.push(Token::Path(type_name));
                    raw_tokens.push(Token::Group(')', vec![vec![str_tok]]));
                    prev_end = Some(span.end());
                    prev_was_callable_op = false;
                    continue;
                }

                let mut tok = parse_token(tok_pair);

                // 2. Слитная цепочка с пустыми скобками: `a.b().c` -> склеиваем в единый `Path("a.b().c")`,
                // если сразу после `()` слитно идет `.c`!
                if is_tight && rule == Rule::path {
                    if let Token::Path(ref p) = tok {
                        if p.starts_with('.') {
                            if let Some(Token::Group(')', args)) = raw_tokens.last() {
                                if args.is_empty()
                                    && matches!(
                                        raw_tokens.get(raw_tokens.len().wrapping_sub(2)),
                                        Some(Token::Path(_))
                                    )
                                {
                                    raw_tokens.pop(); // убираем пустую `)`
                                    if let Some(Token::Path(prev_path)) = raw_tokens.last_mut() {
                                        prev_path.push_str("()");
                                        prev_path.push_str(p);
                                        prev_end = Some(span.end());
                                        prev_was_callable_op = false;
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }

                // 3. Если скобка `(` идет слитно после `Path` (`fn(args)`) или после `callable_op` (`+ (other)`),
                // помечаем её маркером `')'`
                if let Token::Group(delim, _) = &mut tok {
                    if *delim == '(' {
                        let after_tight_path =
                            is_tight && matches!(raw_tokens.last(), Some(Token::Path(_)));
                        if after_tight_path || prev_was_callable_op {
                            *delim = ')';
                        }
                    }
                }

                prev_end = Some(span.end());
                prev_was_callable_op = rule == Rule::callable_op;
                raw_tokens.push(tok);
            }

            // 4. Сворачиваем оператор диапазона `start .. end` в `Token::Range(start, end)`,
            //    а префиксный `..x` сохраняем как `Token::Op("..")` + `x`.
            let mut final_tokens: Vec<Token> = Vec::with_capacity(raw_tokens.len());
            let mut iter = raw_tokens.into_iter();
            while let Some(tok) = iter.next() {
                if matches!(&tok, Token::Op(op) if op == "..") {
                    let prev_is_range_start = matches!(
                        final_tokens.last(),
                        Some(Token::Lit(_) | Token::Path(_) | Token::Group('(', _))
                    );
                    if prev_is_range_start {
                        if let (Some(start), Some(end)) = (final_tokens.pop(), iter.next()) {
                            final_tokens.push(Token::Range(Box::new(start), Box::new(end)));
                            continue;
                        }
                    }
                }
                final_tokens.push(tok);
            }

            let final_tokens = desugar_sentence_tokens(final_tokens);
            sentences.push(final_tokens);
        }
    }
    sentences
}

fn is_rel_op_str(op: &str) -> bool {
    matches!(
        op,
        ".+" | ".-" | ".*" | ".==" | ".!=" | ".>=" | ".<=" | ".>" | ".<" | ".%" | ".**"
    )
}

fn is_param_hole(s: &str) -> bool {
    s.starts_with('_')
        && s.len() >= 2
        && s.chars().nth(1).map(|c| c.is_ascii_lowercase()).unwrap_or(false)
}

fn collect_param_holes_in_sentence(tokens: &[Token], out: &mut Vec<String>) {
    for tok in tokens {
        match tok {
            Token::Path(p) => {
                let root = p.split('.').next().unwrap_or(p);
                if is_param_hole(root) && !out.contains(&root.to_string()) {
                    out.push(root.to_string());
                }
            }
            Token::Group('(', sents) | Token::Group(')', sents) => {
                for s in sents {
                    collect_param_holes_in_sentence(s, out);
                }
            }
            _ => {}
        }
    }
}

/// Нормализует синтаксический сахар Razomy v6.4 на уровне токенов предложения:
/// 1. Leading-dot в выражениях и потоках: `.field` -> `_.field`, `.0` -> `_.0`
/// 2. Relative Operator Step в начале потока/выражения: `[ .+ 1 ]` -> `[ _ + 1 ]`
/// 3. Point-Free Relative Sugar в аргументах вызова: `map(.price)` -> `map([ _.price ])`, `filter(.age >= 18)` -> `filter([ _.age >= 18 ])`
/// 4. Negative Type Guard: `Order & !Cancelled` -> `Order / Cancelled`
/// 5. Explicit In-Place Update: `a .= val` -> `a val`, `a .+ 1` -> `a (a + 1)`
/// 6. Named Function with Form Slots: `name { a Int, b Int 0 } [Body]` -> `name(a Int, b Int 0) [Body]`
fn desugar_sentence_tokens(tokens: Vec<Token>) -> Vec<Token> {
    if tokens.is_empty() {
        return tokens;
    }

    // 1. Named Function with Form Slots: `name { a Int, b Int 0 } [Body]` -> `name(a Int, b Int 0) [Body]`
    if let [Token::Path(fn_name), Token::Group('{', slot_sents), rest @ ..] = tokens.as_slice() {
        let is_snake = fn_name
            .chars()
            .next()
            .map(|c| c.is_ascii_lowercase())
            .unwrap_or(false);
        if is_snake && rest.iter().any(|t| matches!(t, Token::Group('[', _))) {
            let mut rewritten = Vec::with_capacity(tokens.len());
            rewritten.push(Token::Path(fn_name.clone()));
            rewritten.push(Token::Group(')', slot_sents.clone()));
            rewritten.extend_from_slice(rest);
            return desugar_sentence_tokens(rewritten);
        }
    }

    // 2. Ветка роутера с относительным оператором: `.>= 2000 [ 300 ]` или `.== target [ "Matched" ]`
    if tokens.len() >= 3 {
        if let (Some(Token::Op(op)), Some(Token::Group('[', body))) =
            (tokens.first(), tokens.last())
        {
            if is_rel_op_str(op) || op == "./" {
                let raw_op = op.trim_start_matches('.').to_string();
                let mut pred_sent = vec![Token::Path("_".into()), Token::Op(raw_op)];
                for mid in &tokens[1..tokens.len() - 1] {
                    pred_sent.push(desugar_token_inner(mid.clone()));
                }
                return vec![
                    Token::Group('[', vec![pred_sent]),
                    Token::Group('[', body.clone()),
                ];
            }
        }
    }

    // 3. Relative Operator Step в начале предложения внутри `[...]`: `.+ 10` -> `_ + 10`
    if let Some(Token::Op(op)) = tokens.first() {
        if is_rel_op_str(op) && tokens.len() >= 2 {
            let raw_op = op.trim_start_matches('.').to_string();
            let mut out = vec![Token::Path("_".into()), Token::Op(raw_op)];
            for t in tokens.into_iter().skip(1) {
                out.push(desugar_token_inner(t));
            }
            return out;
        }
    }

    // 4. Layer 4 — Ergonomic Perceus Value Rebind Operators (`a .= b`, `a += b`, `a -= b`, `a *= b`, `a /= b`, `a &= b`)
    //    plus legacy `a .+ b`, `a .- b`, `a .* b`:
    if tokens.len() >= 3 {
        if let (Token::Path(lhs), Token::Op(op)) = (&tokens[0], &tokens[1]) {
            if op == ".=" {
                let rhs = desugar_sentence_tokens(tokens[2..].to_vec());
                let wrapped = if rhs.len() == 1 {
                    rhs
                } else {
                    vec![Token::Group('(', vec![rhs])]
                };
                let mut out = vec![Token::Path(lhs.clone())];
                out.extend(wrapped);
                return out;
            }
            let compound_math_op = match op.as_str() {
                "+=" | ".+" => Some("+"),
                "-=" | ".-" => Some("-"),
                "*=" | ".*" => Some("*"),
                "/=" => Some("/"),
                "&=" => Some("&"),
                _ => None,
            };
            if let Some(math_op) = compound_math_op {
                let rhs_desugared = desugar_sentence_tokens(tokens[2..].to_vec());
                let rhs_wrapped = if rhs_desugared.len() == 1 {
                    rhs_desugared
                } else {
                    vec![Token::Group('(', vec![rhs_desugared])]
                };
                let mut rhs_expr = vec![Token::Path(lhs.clone()), Token::Op(math_op.to_string())];
                rhs_expr.extend(rhs_wrapped);
                return vec![
                    Token::Path(lhs.clone()),
                    Token::Group('(', vec![rhs_expr]),
                ];
            }
        }
    }

    // 5. Общий проход по токенам предложения:
    //    • `& !Cancelled` -> `/ Cancelled`
    //    • `a .? .is_healthy` -> `a . filter([ _.is_healthy ])`
    //    • `.field` -> `_.field`
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        // Negative Type Guard: `& !Type` -> `/ Type`
        if i + 2 < tokens.len() {
            if let (Token::Op(amp), Token::Op(bang), Token::Path(neg_ty)) =
                (&tokens[i], &tokens[i + 1], &tokens[i + 2])
            {
                if amp == "&"
                    && bang == "!"
                    && neg_ty
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_uppercase())
                        .unwrap_or(false)
                {
                    out.push(Token::Op("/".into()));
                    out.push(Token::Path(neg_ty.clone()));
                    i += 3;
                    continue;
                }
            }
        }

        // Point-Free Filter Pipe: `nodes .? .is_healthy` -> `nodes . filter([ _.is_healthy ])`
        if i + 1 < tokens.len() {
            if let (Token::Op(qop), Token::Path(rel_p)) = (&tokens[i], &tokens[i + 1]) {
                if qop == ".?" && rel_p.starts_with('.') && rel_p.len() > 1 {
                    let prop = format!("_{rel_p}");
                    out.push(Token::Op(".".into()));
                    out.push(Token::Path("filter".into()));
                    out.push(Token::Group(
                        ')',
                        vec![vec![Token::Group('[', vec![vec![Token::Path(prop)]])]],
                    ));
                    i += 2;
                    continue;
                }
            }
        }

        let tok = tokens[i].clone();
        match tok {
            Token::Path(ref p) if p == "." => {
                out.push(Token::Path("_".into()));
            }
            Token::Path(ref p) if p.starts_with('.') && p.len() > 1 => {
                // Если это метод после закрывающей скобки `(a * b).round_to_int` — оставляем `.round_to_int`!
                let after_group = matches!(out.last(), Some(Token::Group('(' | ')', _)));
                if after_group {
                    out.push(tok);
                } else {
                    // В позиции шага после `.` (`x . .Regex.matches(...)`): убираем ведущую точку
                    let after_pipe = matches!(
                        out.last(),
                        Some(Token::Op(op)) if matches!(op.as_str(), "." | ".>" | ".&" | ".?" | ".!" | "./")
                    );
                    if after_pipe {
                        out.push(Token::Path(p.trim_start_matches('.').to_string()));
                    } else {
                        out.push(Token::Path(format!("_{p}")));
                    }
                }
            }
            // Constrained hole inside parens: `(_a & Int)` or `(_T & Comparable(_T))` -> `_a` or `_T` in runtime expression position
            Token::Group('(', ref inner) if inner.len() == 1 => {
                if let [Token::Path(hole), Token::Op(amp), ..] = inner[0].as_slice() {
                    if amp == "&" && hole.starts_with('_') {
                        out.push(Token::Path(hole.clone()));
                        i += 1;
                        continue;
                    }
                }
                out.push(desugar_token_inner(tok));
            }
            other => out.push(desugar_token_inner(other)),
        }
        i += 1;
    }

    out
}

fn desugar_token_inner(tok: Token) -> Token {
    match tok {
        Token::Group(')', args) => {
            let new_args = args
                .into_iter()
                .map(|arg_sent| {
                    // Point-Free Relative Sugar внутри аргументов вызова:
                    // `map(.price)` -> `map([ _.price ])`
                    // `filter(.age >= 18)` -> `filter([ _.age >= 18 ])`
                    if let Some(Token::Path(first_p)) = arg_sent.first() {
                        if first_p.starts_with('.') && first_p.len() > 1 {
                            let desugared = desugar_sentence_tokens(arg_sent);
                            return vec![Token::Group('[', vec![desugared])];
                        }
                    }
                    if let Some(Token::Op(first_op)) = arg_sent.first() {
                        if is_rel_op_str(first_op) {
                            let desugared = desugar_sentence_tokens(arg_sent);
                            return vec![Token::Group('[', vec![desugared])];
                        }
                    }
                    desugar_sentence_tokens(arg_sent)
                })
                .collect();
            Token::Group(')', new_args)
        }
        Token::Group('[', sents) => {
            let new_sents: Vec<Sentence> = sents.into_iter().map(desugar_sentence_tokens).collect();
            let mut param_holes = Vec::new();
            for s in &new_sents {
                collect_param_holes_in_sentence(s, &mut param_holes);
            }
            if param_holes.len() >= 2 {
                // Sugar #5: 2+ Anonymous Parameter Holes `[ _a + _b ]` -> explicit lambda `(_a, _b) [ _a + _b ]`
                let args: Vec<Sentence> = param_holes
                    .into_iter()
                    .map(|h| vec![Token::Path(h)])
                    .collect();
                Token::Group(
                    '(',
                    vec![vec![
                        Token::Group('(', args),
                        Token::Group('[', new_sents),
                    ]],
                )
            } else if param_holes.len() == 1 {
                let hole_name = &param_holes[0];
                let replaced: Vec<Sentence> = new_sents
                    .into_iter()
                    .map(|s| replace_param_hole_with_underscore(s, hole_name))
                    .collect();
                Token::Group('[', replaced)
            } else {
                Token::Group('[', new_sents)
            }
        }
        Token::Group(delim, sents) => {
            let new_sents = sents.into_iter().map(desugar_sentence_tokens).collect();
            Token::Group(delim, new_sents)
        }
        Token::Range(a, b) => Token::Range(
            Box::new(desugar_token_inner(*a)),
            Box::new(desugar_token_inner(*b)),
        ),
        Token::Path(ref p) if p == "." => Token::Path("_".into()),
        Token::Path(ref p) if p.starts_with('.') && p.len() > 1 => {
            Token::Path(format!("_{p}"))
        }
        other => other,
    }
}

fn replace_param_hole_with_underscore(tokens: Sentence, hole_name: &str) -> Sentence {
    tokens
        .into_iter()
        .map(|t| match t {
            Token::Path(p) if p == hole_name => Token::Path("_".into()),
            Token::Path(p) if p.starts_with(&format!("{hole_name}.")) => {
                Token::Path(format!("_.{}", &p[hole_name.len() + 1..]))
            }
            Token::Group(d, sents) if d != '[' => Token::Group(
                d,
                sents
                    .into_iter()
                    .map(|s| replace_param_hole_with_underscore(s, hole_name))
                    .collect(),
            ),
            other => other,
        })
        .collect()
}

pub fn parse_code(source: &str) -> Result<Vec<Sentence>, pest::error::Error<Rule>> {
    let mut pairs = RazomyParser::parse(Rule::Program, source)?;
    let root = pairs.next().unwrap();
    Ok(parse_sentences(root))
}

/// Восстанавливает неполный исходный код во время набора в IDE (например, висячую точку `User.`, `u.`, `self.`
/// или незавершенную инструкцию) с сохранением 100% байтовых смещений и номеров строк.
pub fn recover_source_for_ide(source: &str) -> String {
    let mut bytes = source.as_bytes().to_vec();
    let len = bytes.len();

    // Проход 1: Заменяем висячие точки `ident.` (где после `.` нет идентификатора/индекса/точки)
    // и висячие пайп-операторы перед закрывающей скобкой или концом файла на пробелы `' '`.
    let mut i = 0;
    while i < len {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'#' && matches!(bytes.get(i + 1), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            i += 2;
            while i < len && bytes[i] != b'#' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'.' {
            let prev_is_ident_or_paren = i > 0
                && (bytes[i - 1].is_ascii_alphanumeric()
                    || bytes[i - 1] == b'_'
                    || bytes[i - 1] == b')');
            let next_b = bytes.get(i + 1).copied();
            let next_is_valid_seg = match next_b {
                Some(b) => {
                    b.is_ascii_alphanumeric()
                        || b == b'_'
                        || b == b'.'
                        || (b == b'-' && bytes.get(i + 2).map_or(false, |b2| b2.is_ascii_digit()))
                }
                None => false,
            };

            if prev_is_ident_or_paren && !next_is_valid_seg {
                bytes[i] = b' ';
                i += 1;
                continue;
            }

            let op_len = if matches!(next_b, Some(b'>' | b'&' | b'?' | b'|')) {
                2
            } else if next_b != Some(b'.') {
                1
            } else {
                0
            };
            if op_len > 0 && !prev_is_ident_or_paren {
                let mut lookahead = i + op_len;
                while lookahead < len && matches!(bytes[lookahead], b' ' | b'\t' | b'\r') {
                    lookahead += 1;
                }
                if lookahead >= len || matches!(bytes[lookahead], b'\n' | b']' | b')' | b'}' | b',')
                {
                    for k in 0..op_len {
                        bytes[i + k] = b' ';
                    }
                    i += op_len;
                    continue;
                }
            }
        }
        i += 1;
    }

    let mut current = String::from_utf8_lossy(&bytes).into_owned();
    if RazomyParser::parse(Rule::Program, &current).is_ok() {
        return current;
    }

    // Проход 2: Построчное восстановление при незавершенных выражениях (сохраняя скобки и переводы строк)
    for _ in 0..10 {
        let Err(err) = RazomyParser::parse(Rule::Program, &current) else {
            break;
        };
        let err_pos = match err.location {
            pest::error::InputLocation::Pos(p) => p,
            pest::error::InputLocation::Span((s, _)) => s,
        };
        let clamped = err_pos.min(bytes.len());
        let mut line_start = clamped;
        while line_start > 0 && bytes[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        let mut line_end = clamped;
        while line_end < bytes.len() && bytes[line_end] != b'\n' {
            line_end += 1;
        }

        let mut changed = false;
        for b in &mut bytes[line_start..line_end] {
            if !matches!(
                *b,
                b'{' | b'}' | b'[' | b']' | b'(' | b')' | b'\r' | b'\n' | b' '
            ) {
                *b = b' ';
                changed = true;
            }
        }
        if !changed && line_start > 0 {
            let prev_end = line_start - 1;
            let mut prev_start = prev_end;
            while prev_start > 0 && bytes[prev_start - 1] != b'\n' {
                prev_start -= 1;
            }
            for b in &mut bytes[prev_start..prev_end] {
                if !matches!(
                    *b,
                    b'{' | b'}' | b'[' | b']' | b'(' | b')' | b'\r' | b'\n' | b' '
                ) {
                    *b = b' ';
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
        current = String::from_utf8_lossy(&bytes).into_owned();
    }

    current
}

// =============================================================================
// 3. СЕМАНТИЧЕСКИЙ КЛАССИФИКАТОР И ДЕКОМПОЗИЦИЯ ПАЙПЛАЙНОВ (V6.0)
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum PipeOp {
    Dot,      // `.`  Standard Dataflow Pipe
    Tap,      // `.>` Side-effect Tap Pipe (and `.~` Finalize / Defer Pipe)
    Enrich,   // `.&` Context Enrich / Merge Pipe
    Subtract, // `./` Subtract / Strip Pipe
    Filter,   // `.?` Positive Filter / Pattern Test / Conditional Exit Pipe
    Guard,    // `.!` Negative Guard / Assert Pipe
    Parallel, // `.|` Concurrent Fan-out Pipe
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipeStep {
    pub op: PipeOp,
    pub expr: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticKind {
    /// Axiom 1: `name value`, `money_val Money"199.99"`, `add (a + b)`, `order_list (...)`
    Binding { key: String, value: Vec<Token> },
    /// Деструктуризация кортежа `(a, b) source` или параметризованный ключ карты `(K) V`
    TupleBinding {
        tuple: Vec<Sentence>,
        value: Vec<Token>,
    },
    /// Axiom 2: `Name { ... }` или `Name { ... } & Trait`
    FormDef {
        name: String,
        body: Vec<Sentence>,
        tail: Vec<Token>,
    },
    /// Именованная функция или перегруженный оператор: `name(args) ReturnType [Body]`, `+ (other Vec2) [Body]`
    FunctionDef {
        name: String,
        args: Vec<Sentence>,
        body: Vec<Sentence>,
        tail: Vec<Token>,
    },
    /// Дженерик-тип / параметризованная форма или контракт метода/оператора: `Map(K, V) { (K) V }`, `format() String`, `< (other Other) Bool`
    GenericDef {
        name: String,
        params: Vec<Sentence>,
        body: Vec<Token>,
    },
    /// Реактивное поле / нулевой геттер / ветка паттерн-матчинга по типу: `subtotal [ self.price * self.quantity ]`
    StreamOrBranch { name: String, stream: Vec<Sentence> },
    /// Анонимная лямбда или ветка кортежа/выражения в роутере: `(args) [Body]`, `(calc(_)) [ ... ]`
    LambdaOrTupleBranch {
        args: Vec<Sentence>,
        body: Vec<Sentence>,
    },
    /// Ветка роутера с предикатом `[ _ >= 2000 ] [ 300 ]`, структурой `{ method 'GET' } [ ... ]` или литералом `true [ ... ]`
    PatternBranch { pattern: Token, body: Vec<Sentence> },
    /// Алгебра типов / уточнений / вычитания: `PublicUser User / User.password_hash`, `Adult User & is_adult`
    AlgebraicDef { name: String, expr: Vec<Token> },
    /// Вызов функции или конструктора `connect("db.internal")`, `run_test_suite()`, `Money"199.99"`
    CallExpr { name: String, args: Vec<Sentence> },
    /// Пайплайн данных (`.`, `.>`, `.&`, `.?`, `.|`)
    Pipeline(Vec<Token>),
    /// Выражение или одиночное значение (`a + b`, `MyGarageCar.-1`)
    Expression(Vec<Token>),
}

pub fn parse_pipe_op(op: &str) -> Option<PipeOp> {
    match op {
        "." => Some(PipeOp::Dot),
        ".>" | ".~" => Some(PipeOp::Tap),
        ".&" => Some(PipeOp::Enrich),
        "./" => Some(PipeOp::Subtract),
        ".?" => Some(PipeOp::Filter),
        ".!" => Some(PipeOp::Guard),
        ".|" => Some(PipeOp::Parallel),
        _ => None,
    }
}

pub fn is_pipe_op(op: &str) -> bool {
    parse_pipe_op(op).is_some()
}

pub fn has_pipe_op(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t, Token::Op(op) if is_pipe_op(op)))
}

pub fn has_algebra_op(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t, Token::Op(op) if matches!(op.as_str(), "&" | "|" | "/")))
}

/// Разбивает плоский пайплайн `a . b .? c` на начальное выражение `a` и шаги `Vec<PipeStep>`
pub fn split_pipeline(tokens: &[Token]) -> (Vec<Token>, Vec<PipeStep>) {
    let mut head = Vec::new();
    let mut steps: Vec<PipeStep> = Vec::new();
    let mut current_op: Option<PipeOp> = None;
    let mut current_expr = Vec::new();

    for token in tokens {
        if let Token::Op(op_str) = token {
            if let Some(pipe_op) = parse_pipe_op(op_str) {
                if let Some(prev_op) = current_op.replace(pipe_op) {
                    steps.push(PipeStep {
                        op: prev_op,
                        expr: std::mem::take(&mut current_expr),
                    });
                } else {
                    head = std::mem::take(&mut current_expr);
                }
                continue;
            }
        }
        current_expr.push(token.clone());
    }

    if let Some(last_op) = current_op {
        steps.push(PipeStep {
            op: last_op,
            expr: current_expr,
        });
    } else {
        head = current_expr;
    }

    (head, steps)
}

/// Проверяет, содержит ли хвост определения функции `rest` блок тела `[Body]`
/// (включая функции с явным возвращаемым типом `name(args) ReturnType [Body]` и суффиксом `& Compile` / `& Private`)
fn split_function_body_and_tail(rest: &[Token]) -> Option<(Vec<Sentence>, Vec<Token>)> {
    for (idx, tok) in rest.iter().enumerate() {
        if let Token::Group('[', body) = tok {
            let prefix = &rest[..idx];
            let suffix = &rest[idx + 1..];
            let prefix_has_brace = prefix.iter().any(|t| matches!(t, Token::Group('{', _)));
            let suffix_valid =
                suffix.is_empty() || matches!(suffix, [Token::Op(op), ..] if op == "&");
            if !prefix_has_brace && suffix_valid {
                let mut combined_tail = prefix.to_vec();
                combined_tail.extend_from_slice(suffix);
                return Some((body.clone(), combined_tail));
            }
        }
    }
    None
}

pub fn classify_sentence(sentence: &Sentence) -> SemanticKind {
    match sentence.as_slice() {
        // 0. Guarded Pattern Branch in Router: `(a, b) & [ a > b ] [ Body ]`
        [Token::Group('(' | ')', args), Token::Op(amp), Token::Group('[', _guard), Token::Group('[', body)]
            if amp == "&" =>
        {
            SemanticKind::LambdaOrTupleBranch {
                args: args.clone(),
                body: body.clone(),
            }
        }

        // 1. Анонимная лямбда или ветка кортежа/выражения в роутере: `(args) [Body]`, `(calc(_)) [ ... ]`
        [Token::Group('(' | ')', args), Token::Group('[', body)] => {
            SemanticKind::LambdaOrTupleBranch {
                args: args.clone(),
                body: body.clone(),
            }
        }

        // 2. Ветка паттерн-матчинга `[ _ >= 2000 ] [ 300 ]`, `{ method 'GET' } [ ... ]`, `true [ ... ]` или `1..10 [ ... ]`
        [pat @ (Token::Group('[' | '{', _) | Token::Lit(_) | Token::Range(_, _)), Token::Group('[', body)] => {
            SemanticKind::PatternBranch {
                pattern: pat.clone(),
                body: body.clone(),
            }
        }

        // 2b. Slotless stream field with directive inside Form `{}`: `auto_cool [ ... ] & Watch`
        [Token::Path(name), Token::Group('[', stream), Token::Op(amp), Token::Path(_directive)]
            if amp == "&" =>
        {
            SemanticKind::StreamOrBranch {
                name: name.clone(),
                stream: stream.clone(),
            }
        }

        // 3. Деструктуризация кортежа `(String, Int) root.base` или параметризованный ключ карты `(K) V`
        [Token::Group('(' | ')', tuple), rest @ ..] if !rest.is_empty() && !has_pipe_op(rest) => {
            SemanticKind::TupleBinding {
                tuple: tuple.clone(),
                value: rest.to_vec(),
            }
        }

        // 4. Ключ карты в виде строкового или символьного литерала: `"host" "localhost"`
        [Token::Lit(Literal::String(key) | Literal::RawString(key) | Literal::Symbol(key)), rest @ ..]
            if !rest.is_empty() && !has_pipe_op(rest) && !matches!(rest, [Token::Op(_), ..]) =>
        {
            SemanticKind::Binding {
                key: key.clone(),
                value: rest.to_vec(),
            }
        }

        // 5. Слитная скобка `Name(Args)` (маркер `')'`):
        // Именованная функция `name(args) [Body]` / `name(args) RetType [Body]`,
        // Вызов `name(args)` / `Type"..."`, Выражение `calc(1) + calc(2)` или Дженерик `Name(T) ...`
        [Token::Path(name), Token::Group(')', args), rest @ ..] => {
            if rest.is_empty() {
                SemanticKind::CallExpr {
                    name: name.clone(),
                    args: args.clone(),
                }
            } else if let Some((body, tail)) = split_function_body_and_tail(rest) {
                SemanticKind::FunctionDef {
                    name: name.clone(),
                    args: args.clone(),
                    body,
                    tail,
                }
            } else if matches!(rest, [Token::Op(op), ..] if op != "&") {
                if has_pipe_op(rest) {
                    SemanticKind::Pipeline(sentence.clone())
                } else {
                    SemanticKind::Expression(sentence.clone())
                }
            } else if has_pipe_op(rest) {
                SemanticKind::Pipeline(sentence.clone())
            } else {
                SemanticKind::GenericDef {
                    name: name.clone(),
                    params: args.clone(),
                    body: rest.to_vec(),
                }
            }
        }

        // 6. Начинается с идентификатора `Name ...` (где после `Name` был пробел!)
        [Token::Path(name), rest @ ..] => match rest {
            [] => SemanticKind::Expression(sentence.clone()),

            [Token::Op(_), ..] => {
                if has_pipe_op(sentence) {
                    SemanticKind::Pipeline(sentence.clone())
                } else {
                    SemanticKind::Expression(sentence.clone())
                }
            }

            [Token::Group('{', body), tail @ ..] => {
                if has_pipe_op(tail) {
                    SemanticKind::Pipeline(sentence.clone())
                } else {
                    SemanticKind::FormDef {
                        name: name.clone(),
                        body: body.clone(),
                        tail: tail.to_vec(),
                    }
                }
            }

            [Token::Group('[', stream)] => SemanticKind::StreamOrBranch {
                name: name.clone(),
                stream: stream.clone(),
            },

            // Axiom 3 & 4: `Name Base { ... }` является каноническим сахаром для `Name (Base & { ... })`
            [Token::Path(base), Token::Group('{', body), tail @ ..] => {
                if has_pipe_op(tail) {
                    SemanticKind::Pipeline(sentence.clone())
                } else {
                    let mut expr = vec![
                        Token::Path(base.clone()),
                        Token::Op("&".into()),
                        Token::Group('{', body.clone()),
                    ];
                    expr.extend_from_slice(tail);
                    SemanticKind::AlgebraicDef {
                        name: name.clone(),
                        expr,
                    }
                }
            }

            // Axiom 4 с Generic-базой: `global_counter Atom(CounterState) { value ... }` -> `Atom(CounterState) & { ... }`
            [Token::Path(base), Token::Group(')', targs), Token::Group('{', body), tail @ ..] => {
                if has_pipe_op(tail) {
                    SemanticKind::Pipeline(sentence.clone())
                } else {
                    let mut expr = vec![
                        Token::Path(base.clone()),
                        Token::Group(')', targs.clone()),
                        Token::Op("&".into()),
                        Token::Group('{', body.clone()),
                    ];
                    expr.extend_from_slice(tail);
                    SemanticKind::AlgebraicDef {
                        name: name.clone(),
                        expr,
                    }
                }
            }

            _ if has_algebra_op(rest) && !has_pipe_op(rest) => SemanticKind::AlgebraicDef {
                name: name.clone(),
                expr: rest.to_vec(),
            },

            _ => SemanticKind::Binding {
                key: name.clone(),
                value: rest.to_vec(),
            },
        },

        // 7. Любые другие пайплайны и выражения
        _ => {
            if has_pipe_op(sentence) {
                SemanticKind::Pipeline(sentence.clone())
            } else {
                SemanticKind::Expression(sentence.clone())
            }
        }
    }
}

pub fn analyze_ast(sentences: &[Sentence], depth: usize) {
    let indent = "  ".repeat(depth);
    for sentence in sentences {
        let kind = classify_sentence(sentence);
        println!("{indent}{:?}", kind);
    }
}

// =============================================================================
// РАЗРЕШЕНИЕ ИМПОРТОВ СОГЛАСНО РАЗДЕЛУ 21 СПЕЦИФИКАЦИИ V5.7
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct LocatedSentence {
    pub sentence: Sentence,
    /// Индекс предложения внутри собственного файла (всегда совпадает с `sentence_spans` этого файла)
    pub local_index: usize,
    /// `None` для главного анализируемого файла, `Some(path)` для импортированного модуля
    pub file_path: Option<PathBuf>,
}

/// Нормализует путь (`.` и `..`) лексически без платформенных префиксов `\\?\` или `/private`
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push(comp);
                }
            }
            other => out.push(other),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportDirective {
    /// Именованный модуль: `Base root.base`, `Http std.net.http`, `Models Path"./models"`
    Namespaced { alias: String, source: ImportSource },
    /// Деструктурирующий импорт: `(String, Int) root.base`
    Destructured {
        names: Vec<String>,
        source: ImportSource,
    },
    /// Wildcard импорт: `_ root.base`, `_ base`, `_ Path"../../../base"`
    Wildcard { source: ImportSource },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportSource {
    /// Точечный путь или идентификатор: `root.base`, `std.net.http`, `base`
    ModulePath(String),
    /// Файловый путь через типизированную строку `Path"..."` / `Path("...")`
    FilePath(String),
}

/// Распознает источник импорта (либо литерал из `Path"..."`, либо путь модуля `root.base`)
fn extract_import_source(tok: &Token, next_tok: Option<&Token>) -> Option<(ImportSource, usize)> {
    match (tok, next_tok) {
        // Path"..." преобразуется лексером в Path + Group(')', [["..."]])
        (Token::Path(p), Some(Token::Group(')', args))) if p == "Path" => {
            if let Some(Token::Lit(Literal::String(path_str) | Literal::RawString(path_str))) =
                args.first().and_then(|s| s.first())
            {
                return Some((ImportSource::FilePath(path_str.clone()), 2));
            }
            None
        }
        // Обычный точечный путь или идентификатор модуля: `root.base`, `base`, `std.net.http`
        (Token::Path(path_str), _) => Some((ImportSource::ModulePath(path_str.clone()), 1)),
        _ => None,
    }
}

/// Сопоставляет предложение Sentence с грамматикой импортов Razomy v6.4 (с поддержкой синтаксиса v5.7):
/// 1) `..<source>` или `_ <source>`           -> Wildcard / Lineage Spread
/// 2) `{ A, B, C } <source>` или `(A, B) <src>` -> Destructured
/// 3) `Alias <source>`                        -> Namespaced
pub fn extract_import(sent: &Sentence) -> Option<ImportDirective> {
    match sent.as_slice() {
        // 1a. V6.4 Prefix Lineage Spread Import: `..root.prelude`, `..Path"../shared/ext_mod.rl"`, `..(std.net.http / 'legacy_request')`
        [Token::Op(op), rest @ ..] if op == ".." && !rest.is_empty() => {
            if let [Token::Group('(', inner)] = rest {
                if let Some(first_sent) = inner.first() {
                    if let Some((source, _)) =
                        extract_import_source(first_sent.first()?, first_sent.get(1))
                    {
                        return Some(ImportDirective::Wildcard { source });
                    }
                }
            }
            let next = rest.get(1);
            let (source, _) = extract_import_source(&rest[0], next)?;
            Some(ImportDirective::Wildcard { source })
        }

        // 1b. Legacy Wildcard-импорт: `_ root.base` или `_ Path"../base"`
        [Token::Path(target), rest @ ..] if target == "_" && !rest.is_empty() => {
            let next = rest.get(1);
            let (source, _) = extract_import_source(&rest[0], next)?;
            Some(ImportDirective::Wildcard { source })
        }

        // 2. Деструктурирующий импорт: `{ String, Int, Money } root.base` или `(String, Int) root.base`
        [Token::Group('{' | '(', items), rest @ ..] if !rest.is_empty() => {
            let next = rest.get(1);
            let (source, _) = extract_import_source(&rest[0], next)?;

            let mut names = Vec::new();
            for item_sentence in items {
                for t in item_sentence {
                    if let Token::Path(name) = t {
                        if name != ".." {
                            names.push(name.clone());
                        }
                    }
                }
            }
            if names.is_empty() {
                return None;
            }
            Some(ImportDirective::Destructured { names, source })
        }

        // 3. Именованный импорт: `Base root.base`, `Http std.net.http`, `Models Path"./models"`
        // Одиночный PascalCase идентификатор справа без точки (`Cat User`, `Age Int`) — это алиас типа, а не модуль!
        [Token::Path(alias), rest @ ..]
            if !rest.is_empty()
                && alias
                    .chars()
                    .next()
                    .map_or(false, |c| c.is_ascii_uppercase()) =>
        {
            let next = rest.get(1);
            let (source, _) = extract_import_source(&rest[0], next)?;
            if let ImportSource::ModulePath(ref m) = source {
                let first_char = m.chars().next().unwrap_or('_');
                if first_char.is_ascii_uppercase() && !m.contains('.') {
                    return None;
                }
            }
            Some(ImportDirective::Namespaced {
                alias: alias.clone(),
                source,
            })
        }

        _ => None,
    }
}

/// Преобразует источник импорта в относительный путь к `.rl` файлу
pub fn source_to_fs_path(source: &ImportSource) -> PathBuf {
    match source {
        ImportSource::FilePath(p) => {
            let mut pb = PathBuf::from(p);
            if pb.extension().is_none() {
                pb.set_extension("rl");
            }
            pb
        }
        ImportSource::ModulePath(m) => {
            // "root.base" -> "root/base.rl"
            let relative = m.replace('.', "/");
            let mut pb = PathBuf::from(relative);
            if pb.extension().is_none() {
                pb.set_extension("rl");
            }
            pb
        }
    }
}

pub fn resolve_import_fs_path(
    base_dir: &Path,
    source: &ImportSource,
    in_memory_files: &std::collections::HashMap<PathBuf, String>,
) -> PathBuf {
    let raw_rel = source_to_fs_path(source);
    let candidate = normalize_path(&base_dir.join(&raw_rel));
    if candidate.exists() || in_memory_files.contains_key(&candidate) {
        return candidate;
    }
    if let ImportSource::ModulePath(m) = source {
        if let Some(stripped) = m.strip_prefix("root.") {
            let mut pb = PathBuf::from(stripped.replace('.', "/"));
            if pb.extension().is_none() {
                pb.set_extension("rl");
            }
            let alt = normalize_path(&base_dir.join(pb));
            if alt.exists() || in_memory_files.contains_key(&alt) {
                return alt;
            }
        }
    }
    candidate
}

/// Рекурсивный резолвер и сборщик модулей
pub fn parse_code_with_imports(
    source: &str,
    base_dir: Option<&Path>,
) -> Result<Vec<Sentence>, String> {
    parse_code_with_imports_and_memory(source, base_dir, &std::collections::HashMap::new())
}

pub fn parse_code_with_imports_and_memory(
    source: &str,
    base_dir: Option<&Path>,
    in_memory_files: &std::collections::HashMap<PathBuf, String>,
) -> Result<Vec<Sentence>, String> {
    let located = parse_located_with_imports_and_memory(source, base_dir, in_memory_files)?;
    Ok(located.into_iter().map(|ls| ls.sentence).collect())
}

pub fn parse_located_with_imports_and_memory(
    source: &str,
    base_dir: Option<&Path>,
    in_memory_files: &std::collections::HashMap<PathBuf, String>,
) -> Result<Vec<LocatedSentence>, String> {
    let mut visited = HashSet::new();
    parse_recursive_located(source, base_dir, None, &mut visited, in_memory_files)
}

fn parse_recursive_located(
    source: &str,
    base_dir: Option<&Path>,
    current_file: Option<&Path>,
    visited: &mut HashSet<PathBuf>,
    in_memory_files: &std::collections::HashMap<PathBuf, String>,
) -> Result<Vec<LocatedSentence>, String> {
    let raw_sentences = parse_code(source)
        .or_else(|_| parse_code(&recover_source_for_ide(source)))
        .map_err(|e| format!("Синтаксическая ошибка:\n{e}"))?;
    let mut expanded = Vec::with_capacity(raw_sentences.len());
    let current_dir = base_dir.unwrap_or_else(|| Path::new("."));
    let current_file_buf = current_file.map(normalize_path);

    for (local_idx, sent) in raw_sentences.into_iter().enumerate() {
        if let Some(import_dir) = extract_import(&sent) {
            match import_dir {
                ImportDirective::Wildcard { source: imp_src } => {
                    let target_path =
                        resolve_import_fs_path(current_dir, &imp_src, in_memory_files);
                    if target_path.exists() || in_memory_files.contains_key(&target_path) {
                        let imported_sents =
                            load_imported_file_located(&target_path, visited, in_memory_files)?;
                        expanded.extend(imported_sents);
                    }
                    expanded.push(LocatedSentence {
                        sentence: sent,
                        local_index: local_idx,
                        file_path: current_file_buf.clone(),
                    });
                    continue;
                }

                ImportDirective::Namespaced {
                    alias,
                    source: imp_src,
                } => {
                    let target_path =
                        resolve_import_fs_path(current_dir, &imp_src, in_memory_files);
                    if target_path.exists() || in_memory_files.contains_key(&target_path) {
                        let imported_sents =
                            load_imported_file_located(&target_path, visited, in_memory_files)?;
                        let raw_inner: Vec<Sentence> =
                            imported_sents.into_iter().map(|ls| ls.sentence).collect();
                        expanded.push(LocatedSentence {
                            sentence: sent,
                            local_index: local_idx,
                            file_path: current_file_buf.clone(),
                        });
                        expanded.push(LocatedSentence {
                            sentence: vec![Token::Path(alias), Token::Group('{', raw_inner)],
                            local_index: local_idx,
                            file_path: Some(target_path),
                        });
                        continue;
                    }
                }

                ImportDirective::Destructured {
                    names,
                    source: imp_src,
                } => {
                    let target_path =
                        resolve_import_fs_path(current_dir, &imp_src, in_memory_files);
                    if target_path.exists() || in_memory_files.contains_key(&target_path) {
                        let imported_sents =
                            load_imported_file_located(&target_path, visited, in_memory_files)?;
                        let filtered =
                            imported_sents
                                .into_iter()
                                .filter(|ls| match ls.sentence.first() {
                                    Some(Token::Path(n)) => names.contains(n),
                                    _ => false,
                                });
                        expanded.extend(filtered);
                    }
                    expanded.push(LocatedSentence {
                        sentence: sent,
                        local_index: local_idx,
                        file_path: current_file_buf.clone(),
                    });
                    continue;
                }
            }
        }

        expanded.push(LocatedSentence {
            sentence: sent,
            local_index: local_idx,
            file_path: current_file_buf.clone(),
        });
    }

    Ok(expanded)
}

fn load_imported_file_located(
    candidate: &Path,
    visited: &mut HashSet<PathBuf>,
    in_memory_files: &std::collections::HashMap<PathBuf, String>,
) -> Result<Vec<LocatedSentence>, String> {
    let norm = normalize_path(candidate);
    let visit_key = candidate.canonicalize().unwrap_or_else(|_| norm.clone());

    if !visited.insert(visit_key) {
        return Ok(Vec::new());
    }

    let content = if let Some(mem_text) = in_memory_files
        .get(&norm)
        .or_else(|| in_memory_files.get(candidate))
        .or_else(|| {
            candidate
                .canonicalize()
                .ok()
                .and_then(|c| in_memory_files.get(&c))
        }) {
        mem_text.clone()
    } else {
        std::fs::read_to_string(&norm)
            .map_err(|e| format!("Ошибка чтения модуля '{}': {e}", norm.display()))?
    };

    parse_recursive_located(
        &content,
        norm.parent(),
        Some(&norm),
        visited,
        in_memory_files,
    )
}
