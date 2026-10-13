use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rl.pest"]
struct LangParser;

// ---------------------------------------------------------
// 1. AST: 5 вариантов токенов
// ---------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Path(String),               // Имена, типы, пути (Name, Process.Log)
    Number(f64),                // Числа
    String(String),             // Строки (без кавычек)
    Op(String),                 // Операторы (&, |, =, и т.д.)
    Group(char, Vec<Sentence>), // Вложенные блоки: '(', '[', '{', '<'
}

pub type Sentence = Vec<Token>;

// ---------------------------------------------------------
// 2. Конвертация Pest Pairs в чистое AST
// ---------------------------------------------------------
fn parse_token(pair: pest::iterators::Pair<Rule>) -> Token {
    match pair.as_rule() {
        Rule::path => Token::Path(pair.as_str().to_string()),
        Rule::number => Token::Number(pair.as_str().parse().unwrap()),
        Rule::double_quote_group | Rule::single_quote_group => {
            let s = pair.as_str();
            // Убираем внешние кавычки
            Token::String(s[1..s.len() - 1].to_string())
        }
        Rule::op => Token::Op(pair.as_str().to_string()),
        Rule::paren_group => Token::Group('(', parse_sentences(pair)),
        Rule::bracket_group => Token::Group('[', parse_sentences(pair)),
        Rule::brace_group => Token::Group('{', parse_sentences(pair)),
        Rule::angle_group => Token::Group('<', parse_sentences(pair)), // Добавили < >
        _ => unreachable!("Неизвестный токен: {:?}", pair.as_rule()),
    }
}

fn parse_sentences(pair: pest::iterators::Pair<Rule>) -> Vec<Sentence> {
    let mut sentences = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::sentence {
            let tokens = inner.into_inner().map(parse_token).collect();
            sentences.push(tokens);
        }
    }
    sentences
}

pub fn parse_code(source: &str) -> Vec<Sentence> {
    // Нам больше не нужен desugar_python_indent, так как целевой язык
    // уже использует явные скобки {}
    let pairs = LangParser::parse(Rule::Program, source)
        .expect("Ошибка синтаксического анализа")
        .next()
        .unwrap();

    parse_sentences(pairs)
}

// ---------------------------------------------------------
// 3. Анализатор (Интерпретатор / Компилятор)
// ---------------------------------------------------------
fn analyze_ast(sentences: &[Sentence], depth: usize) {
    let indent = "  ".repeat(depth);

    for sentence in sentences {
        match sentence.as_slice() {
            // 1. Свойства ключа со строковым значением (Default "")
            [Token::Path(key), Token::String(val)] => {
                println!("{indent}[СВОЙСТВО] {} = \"{}\"", key, val);
            }

            // 2. Поле с типом (text Text)
            [Token::Path(name), Token::Path(typ)] => {
                println!("{indent}[ПОЛЕ] {} : {}", name, typ);
            }

            // 3. Объединение типов (AnyMessage TextMessage | ToolMessage)
            [Token::Path(name), Token::Path(t1), Token::Op(op), Token::Path(t2)] if op == "|" => {
                println!("{indent}[UNION ТИП] {} = {} | {}", name, t1, t2);
            }

            // 4. Дженерик типы (AnyMessages Array<AnyMessage>)
            [Token::Path(name), Token::Path(base_type), Token::Group('<', generics)] => {
                println!("{indent}[ДЖЕНЕРИК] {} = {}<...>", name, base_type);
                analyze_ast(generics, depth + 1);
            }

            // 5. Структура с модификатором & (SourceDir DefaultStr & { Default "." })
            [Token::Path(name), Token::Path(base), Token::Op(op), Token::Group('{', body)] if op == "&" => {
                println!("{indent}[СТРУКТУРА + ТРЕЙТ] {} расширяет {}", name, base);
                analyze_ast(body, depth + 1);
            }

            // 6. Обычная структура или блок (Shell { ... })
            [Token::Path(name), Token::Group('{', body)] => {
                println!("{indent}[БЛОК/СТРУКТУРА] {}", name);
                analyze_ast(body, depth + 1);
            }

            // 7. Конструкция match <Query> { ... }
            [Token::Path(kw), Token::Group('<', target), Token::Group('{', body)] if kw == "match" => {
                println!("{indent}[MATCH] по слоту:");
                analyze_ast(target, depth + 1);
                println!("{indent}Ветки match:");
                analyze_ast(body, depth + 1);
            }

            // 8. Ветка match (IsError(T <Query>) { <Yes> })
            [Token::Path(name), Token::Group('(', args), Token::Group('{', body)] => {
                println!("{indent}[ВЫЗОВ/ВЕТКА] {}(...)", name);
                analyze_ast(args, depth + 1);
                println!("{indent}Тело ветки:");
                analyze_ast(body, depth + 1);
            }

            // ==========================================
            // --- НОВЫЕ ПРАВИЛА ---
            // ==========================================

            // 9. Вызов функции или команды ( Process.Log(<T>) )
            [Token::Path(name), Token::Group('(', args)] => {
                println!("{indent}[ВЫЗОВ КОМАНДЫ] {}(...)", name);
                analyze_ast(args, depth + 1);
            }

            // 10. Передача значения в именованный слот ( T <Query> )
            [Token::Path(name), Token::Group('<', val)] => {
                println!("{indent}[АРГУМЕНТ СЛОТА] {} = <...>", name);
                analyze_ast(val, depth + 1);
            }

            // 11. Одиночное имя.
            // Это может быть включение типа (SourceDir), тип внутри дженерика (AnyMessage)
            // или просто имя слота (Query, Yes, No)
            [Token::Path(name)] => {
                println!("{indent}[ИДЕНТИФИКАТОР] {}", name);
            }

            // ==========================================

            // 12. Слот вида <T> или <Yes>
            [Token::Group('<', content)] => {
                println!("{indent}[СЛОТ/ВОЗВРАТ]");
                analyze_ast(content, depth + 1);
            }

            // 13. Бинарное выражение с instanceof ( <T> instanceof Error )
            [Token::Group('<', slot), Token::Path(op), Token::Path(typ)] if op == "instanceof" => {
                println!("{indent}[ПРОВЕРКА ТИПА] <слот> instanceof {}", typ);
            }

            // Fallback (если что-то забыли)
            _ => println!("{indent}[НЕИЗВЕСТНО] {:?}", sentence),
        }
    }
}
// ---------------------------------------------------------
// 4. Тестирование
// ---------------------------------------------------------
fn main() {
    let code = r#"
// --- 1. Определение типов ---
Default {}
DefaultStr str & { Default "" }
SourceDir DefaultStr & { Default "." }

Text DefaultStr
Name DefaultStr
Message {}

TextMessage Message & { text Text }
ToolMessage Message & { name Name }

AnyMessage TextMessage | ToolMessage
AnyMessages Array<AnyMessage>

AiContext {
  SourceDir
  AnyMessages
}

// --- 2. Системные модули ---
Shell {
  Log { Process.Log(<T>) }
  Command { Process.Execute(<T>) }
}

// --- 3. Предикаты и паттерн-матчинг ---
IsError { <T> instanceof Error }

MatchYesNo {
  match <Query> {
    IsError(T <Query>) { <Yes> }
    Default { <No> }
  }
}
"#;

    println!("--- 1. ПАРСИНГ ---");
    let ast = parse_code(code);

    // Если хочешь посмотреть сырое AST раскомментируй:
    // println!("{:#?}", ast);

    println!("\n--- 2. АНАЛИЗ И СМАНТИКА ---");
    analyze_ast(&ast, 0);
}