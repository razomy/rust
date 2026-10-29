use pest::Parser;
use pest_derive::Parser;

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

fn extract_raw_string_content(s: &str) -> String {
    if (s.starts_with("r3\"") && s.ends_with("\"3"))
        || (s.starts_with("r2\"") && s.ends_with("\"2"))
        || (s.starts_with("r1\"") && s.ends_with("\"1"))
    {
        s[3..s.len() - 2].to_string()
    } else if s.starts_with("r\"") && s.ends_with('"') {
        s[2..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
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
                                if args.is_empty() && matches!(raw_tokens.get(raw_tokens.len().wrapping_sub(2)), Some(Token::Path(_))) {
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
                        let after_tight_path = is_tight && matches!(raw_tokens.last(), Some(Token::Path(_)));
                        if after_tight_path || prev_was_callable_op {
                            *delim = ')';
                        }
                    }
                }

                prev_end = Some(span.end());
                prev_was_callable_op = rule == Rule::callable_op;
                raw_tokens.push(tok);
            }

            // 4. Сворачиваем оператор диапазона `start .. end` в `Token::Range(start, end)`
            let mut final_tokens: Vec<Token> = Vec::with_capacity(raw_tokens.len());
            let mut iter = raw_tokens.into_iter();
            while let Some(tok) = iter.next() {
                if matches!(&tok, Token::Op(op) if op == "..") {
                    if let (Some(start), Some(end)) = (final_tokens.pop(), iter.next()) {
                        final_tokens.push(Token::Range(Box::new(start), Box::new(end)));
                        continue;
                    }
                }
                final_tokens.push(tok);
            }

            sentences.push(final_tokens);
        }
    }
    sentences
}

pub fn parse_code(source: &str) -> Result<Vec<Sentence>, pest::error::Error<Rule>> {
    let mut pairs = RazomyParser::parse(Rule::Program, source)?;
    let root = pairs.next().unwrap();
    Ok(parse_sentences(root))
}

// =============================================================================
// 3. СЕМАНТИЧЕСКИЙ КЛАССИФИКАТОР И ДЕКОМПОЗИЦИЯ ПАЙПЛАЙНОВ (V6.0)
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum PipeOp {
    Dot,      // `.`  Standard Dataflow Pipe
    Tap,      // `.>` Side-effect Tap Pipe
    Enrich,   // `.&` Context Enrich / Merge Pipe
    Filter,   // `.?` Safe / Predicate Filter Pipe
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
        ".>" => Some(PipeOp::Tap),
        ".&" => Some(PipeOp::Enrich),
        ".?" => Some(PipeOp::Filter),
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
            let suffix_valid = suffix.is_empty()
                || matches!(suffix, [Token::Op(op), ..] if op == "&");
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
