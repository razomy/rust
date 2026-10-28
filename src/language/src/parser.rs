use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct RazomyParser;

// =============================================================================
// 1. ГОМОИКОННОЕ AST: ЛИТЕРАЛЫ И ТОКЕНЫ
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Byte(u8),
    String(String),   // "..."
    Symbol(String),   // '...'
    Money(String),    // $199.99, USD$500.00
    Percent(f64),     // 15.5%
    Date(String),     // 2026-03-30
    Time(String),     // 14:30:00
    Duration(String), // 5m30s, 2s
    Coord(String),    // 1920x1080
    Color(String),    // #2A9D8F
    IpAddr(String),   // 192.168.0.1:8080
    Version(String),  // 1.4.2-beta
    Email(String),    // admin@razomy.org
    Url(String),      // https://razomy.org, https://api.ex/{Id}
    File(String),     // %./data/users.json
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Имена, типы, слитные пути и индексы (`Name`, `User.Name`, `JonnDoyName3.-1`, `_.Age`)
    Path(String),
    /// Самотипизированные литералы и примитивы
    Lit(Literal),
    /// Операторы (`.`, `.>`, `.&`, `.?`, `.|`, `&`, `|`, `/`, `+`, `==`, `!`, и т.д.)
    Op(String),
    /// Диапазоны (`1..10`, `"a".."z"`, `1..Limit`)
    Range(Box<Token>, Box<Token>),
    /// Три гомоиконные скобки:
    /// '{' (Form), '[' (Stream), '(' (Tuple/Expression через пробел),
    /// ')' (маркер слитного списка аргументов `Fn(Args)` без пробела)
    Group(char, Vec<Sentence>),
}

pub type Sentence = Vec<Token>;

// =============================================================================
// 2. КОНВЕРТАЦИЯ PEST PAIRS В ЧИСТОЕ AST
// =============================================================================

pub fn parse_token(pair: pest::iterators::Pair<Rule>) -> Token {
    match pair.as_rule() {
        Rule::brace_group => Token::Group('{', parse_sentences(pair)),
        Rule::bracket_group => Token::Group('[', parse_sentences(pair)),
        Rule::paren_group => Token::Group('(', parse_sentences(pair)),

        Rule::string_lit => {
            let s = pair.as_str();
            Token::Lit(Literal::String(s[1..s.len() - 1].to_string()))
        }
        Rule::symbol_lit => {
            let s = pair.as_str();
            Token::Lit(Literal::Symbol(s[1..s.len() - 1].to_string()))
        }

        Rule::range_expr => {
            let mut inner = pair.into_inner();
            let start = parse_token(inner.next().unwrap());
            let end = parse_token(inner.next().unwrap());
            Token::Range(Box::new(start), Box::new(end))
        }

        Rule::url_lit => Token::Lit(Literal::Url(pair.as_str().to_string())),
        Rule::file_lit => Token::Lit(Literal::File(pair.as_str().to_string())),
        Rule::email_lit => Token::Lit(Literal::Email(pair.as_str().to_string())),
        Rule::money_lit => Token::Lit(Literal::Money(pair.as_str().to_string())),
        Rule::date_lit => Token::Lit(Literal::Date(pair.as_str().to_string())),
        Rule::time_lit => Token::Lit(Literal::Time(pair.as_str().to_string())),
        Rule::ip_lit => Token::Lit(Literal::IpAddr(pair.as_str().to_string())),
        Rule::version_lit => Token::Lit(Literal::Version(pair.as_str().to_string())),
        Rule::coord_lit => Token::Lit(Literal::Coord(pair.as_str().to_string())),
        Rule::duration_lit => Token::Lit(Literal::Duration(pair.as_str().to_string())),
        Rule::percent_lit => {
            let s = pair.as_str();
            let val: f64 = s[..s.len() - 1].parse().unwrap();
            Token::Lit(Literal::Percent(val))
        }
        Rule::color_lit => Token::Lit(Literal::Color(pair.as_str().to_string())),
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

        Rule::operator | Rule::callable_op => Token::Op(pair.as_str().to_string()),

        Rule::path | Rule::callable_path | Rule::path_core => {
            let s = pair.as_str();
            if s == "true" {
                Token::Lit(Literal::Bool(true))
            } else if s == "false" {
                Token::Lit(Literal::Bool(false))
            } else {
                Token::Path(s.to_string())
            }
        }

        _ => unreachable!("Неизвестное правило: {:?}", pair.as_rule()),
    }
}

pub fn parse_sentences(pair: pest::iterators::Pair<Rule>) -> Vec<Sentence> {
    let mut sentences = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::sentence {
            let mut tokens = Vec::new();
            let mut expect_tight_paren = false;

            for tok_pair in inner.into_inner() {
                let rule = tok_pair.as_rule();
                let mut tok = parse_token(tok_pair);

                if expect_tight_paren {
                    if let Token::Group('(', args) = tok {
                        tok = Token::Group(')', args);
                    }
                }

                expect_tight_paren = matches!(rule, Rule::callable_path | Rule::callable_op);
                tokens.push(tok);
            }
            sentences.push(tokens);
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
// 3. СЕМАНТИЧЕСКИЙ КЛАССИФИКАТОР И ДЕКОМПОЗИЦИЯ ПАЙПЛАЙНОВ (V4.7)
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
    /// Axiom 1: `Name Value`, `MoneyVal $199.99`, `Add (A + B)`, `OrderList (...)`
    Binding { key: String, value: Vec<Token> },
    /// Деструктуризация кортежа `(A, B) Source` или параметризованный ключ карты `(K) V`
    TupleBinding {
        tuple: Vec<Sentence>,
        value: Vec<Token>,
    },
    /// Axiom 2 & 3: `Name { ... }` или `Name { ... } & Trait`
    FormDef {
        name: String,
        body: Vec<Sentence>,
        tail: Vec<Token>,
    },
    /// Axiom 4: `User Identifiable { ... } & Extra` или `GlobalCounter Atom(CounterState) { ... }`
    SubtypeFormDef {
        name: String,
        base: String,
        body: Vec<Sentence>,
        tail: Vec<Token>,
    },
    /// Именованная функция: `Name(Args) [Body]` (с возможным `& Private`)
    FunctionDef {
        name: String,
        args: Vec<Sentence>,
        body: Vec<Sentence>,
        tail: Vec<Token>,
    },
    /// Дженерик-тип / форма или контракт метода: `Map(K, V) { (K) V }`, `List(T) None | Cons(T)`
    GenericDef {
        name: String,
        params: Vec<Sentence>,
        body: Vec<Token>,
    },
    /// Реактивное поле / поток / ветка паттерн-матчинга по типу: `Subtotal [ Price * Quantity ]`
    StreamOrBranch { name: String, stream: Vec<Sentence> },
    /// Перегрузка оператора или контракт метода: `+(Other Vec2) [ ... ]`, `<(Other) Bool`
    OperatorMethod {
        op: String,
        args: Vec<Sentence>,
        rest: Vec<Token>,
    },
    /// Анонимная лямбда или ветка кортежа в роутере: `(Args) [Body]`
    LambdaOrTupleBranch {
        args: Vec<Sentence>,
        body: Vec<Sentence>,
    },
    /// Ветка роутера с предикатом `[ _ >= 2000 ] [ 300 ]`, структурой `{ Method 'GET' } [ ... ]` или литералом `true [ ... ]`
    PatternBranch { pattern: Token, body: Vec<Sentence> },
    /// Алгебра типов / уточнений / вычитания: `PublicUser User / User.PasswordHash`, `Adult User & IsAdult`
    AlgebraicDef { name: String, expr: Vec<Token> },
    /// Вызов функции `Connect("db.internal")`, `RunTestSuite()` (слитная скобка без пробела!)
    CallExpr { name: String, args: Vec<Sentence> },
    /// Пайплайн данных (`.`, `.>`, `.&`, `.?`, `.|`)
    Pipeline(Vec<Token>),
    /// Выражение или одиночное значение (`A + B`, `JonnDoyName3.-1`)
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

/// Разбивает плоский пайплайн `A . B .? C` на начальное выражение `A` и шаги `Vec<PipeStep>`
pub fn split_pipeline(tokens: &[Token]) -> (Vec<Token>, Vec<PipeStep>) {
    let mut head = Vec::new();
    let mut steps: Vec<PipeStep> = Vec::new();
    let mut current_op: Option<PipeOp> = None;
    let mut current_expr = Vec::new();

    for token in tokens {
        if let Token::Op(op_str) = token {
            if let Some(pipe_op) = parse_pipe_op(pipe_op_str(op_str)) {
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

#[inline]
fn pipe_op_str(s: &str) -> &str {
    s
}

pub fn classify_sentence(sentence: &Sentence) -> SemanticKind {
    match sentence.as_slice() {
        // 1. Перегрузка оператора или контракт (слитная скобка `)`): `+(Other Vec2) [ ... ]`, `<(Other) Bool`
        [Token::Op(op), Token::Group(')' | '(', args), rest @ ..] if !rest.is_empty() => {
            SemanticKind::OperatorMethod {
                op: op.clone(),
                args: args.clone(),
                rest: rest.to_vec(),
            }
        }

        // 2. Анонимная лямбда или ветка кортежа: `(Args) [Body]`
        [Token::Group('(' | ')', args), Token::Group('[', body)] => {
            SemanticKind::LambdaOrTupleBranch {
                args: args.clone(),
                body: body.clone(),
            }
        }

        // 3. Ветка паттерн-матчинга `[ _ >= 2000 ] [ 300 ]`, `{ Method 'GET' } [ ... ]`, `true [ ... ]` или `1..10 [ ... ]`
        [pat @ Token::Group('[', _), Token::Group('[', body)]
        | [pat @ Token::Group('{', _), Token::Group('[', body)]
        | [pat @ Token::Lit(_), Token::Group('[', body)]
        | [pat @ Token::Range(_, _), Token::Group('[', body)] => SemanticKind::PatternBranch {
            pattern: pat.clone(),
            body: body.clone(),
        },

        // 4. Деструктуризация кортежа `(String, Int) root.base` или ключ карты `("host") "localhost"`
        [Token::Group('(' | ')', tuple), rest @ ..] if !rest.is_empty() && !has_pipe_op(rest) => {
            SemanticKind::TupleBinding {
                tuple: tuple.clone(),
                value: rest.to_vec(),
            }
        }

        // 5. Ключ карты в виде строкового или символьного литерала: `"host" "localhost"`
        [Token::Lit(Literal::String(key) | Literal::Symbol(key)), rest @ ..]
        if !rest.is_empty() && !has_pipe_op(rest) && !matches!(rest, [Token::Op(_), ..]) =>
            {
                SemanticKind::Binding {
                    key: key.clone(),
                    value: rest.to_vec(),
                }
            }

        // 6. Слитная скобка `Name(Args)` (маркер `')'`):
        // Именованная функция `Name(Args) [Body]`, Вызов `Name(Args)`, Выражение `Calc(1) + Calc(2)` или Дженерик `Name(T) ...`
        [Token::Path(name), Token::Group(')', args), rest @ ..] => match rest {
            [Token::Group('[', body), tail @ ..] => SemanticKind::FunctionDef {
                name: name.clone(),
                args: args.clone(),
                body: body.clone(),
                tail: tail.to_vec(),
            },
            [] => SemanticKind::CallExpr {
                name: name.clone(),
                args: args.clone(),
            },
            [Token::Op(_), ..] => {
                if has_pipe_op(rest) {
                    SemanticKind::Pipeline(sentence.clone())
                } else {
                    SemanticKind::Expression(sentence.clone())
                }
            }
            _ => SemanticKind::GenericDef {
                name: name.clone(),
                params: args.clone(),
                body: rest.to_vec(),
            },
        },

        // 7. Начинается с идентификатора `Name ...` (где после `Name` был пробел!)
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

            [Token::Path(base), Token::Group('{', body), tail @ ..] => {
                if has_pipe_op(tail) {
                    SemanticKind::Pipeline(sentence.clone())
                } else {
                    SemanticKind::SubtypeFormDef {
                        name: name.clone(),
                        base: base.clone(),
                        body: body.clone(),
                        tail: tail.to_vec(),
                    }
                }
            }

            // Axiom 4 с Generic-базой: `GlobalCounter Atom(CounterState) { Value ... }`
            [Token::Path(base), Token::Group(')', _), Token::Group('{', body), tail @ ..] => {
                if has_pipe_op(tail) {
                    SemanticKind::Pipeline(sentence.clone())
                } else {
                    SemanticKind::SubtypeFormDef {
                        name: name.clone(),
                        base: base.clone(),
                        body: body.clone(),
                        tail: tail.to_vec(),
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

        // 8. Любые другие пайплайны и выражения
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
