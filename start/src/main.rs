use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rl.pest"]
struct LangParser;

// ---------------------------------------------------------
// 1. AST: Всего 5 вариантов токенов
// ---------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Number(f64),
    String(String),
    Op(String),
    Group(char, Vec<Sentence>), // Вложенные блоки: '(', '[', или '{'
}

pub type Sentence = Vec<Token>;

// ---------------------------------------------------------
// 2. Препроцессор: отступы Python -> фигурные скобки { }
// ---------------------------------------------------------
pub fn desugar_python_indent(input: &str) -> String {
    let mut result = String::new();
    let mut indent_stack = vec![0];

    for line in input.lines() {
        let trimmed = line.trim();
        // Игнорируем пустые строки и комментарии
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Считаем пробелы в начале (1 таб = 4 пробела)
        let indent = line
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum::<usize>();

        let current_indent = *indent_stack.last().unwrap();

        if indent > current_indent {
            indent_stack.push(indent);
            result.push_str(" { \n");
        } else if indent < current_indent {
            while indent < *indent_stack.last().unwrap() {
                indent_stack.pop();
                result.push_str(" } \n");
            }
        }

        result.push_str(trimmed);
        result.push('\n');
    }

    // Закрываем висящие скобки в конце
    while indent_stack.len() > 1 {
        indent_stack.pop();
        result.push_str(" }\n");
    }

    result
}

// ---------------------------------------------------------
// 3. Конвертация Pest Pairs в наше чистое AST
// ---------------------------------------------------------
fn parse_token(pair: pest::iterators::Pair<Rule>) -> Token {
    match pair.as_rule() {
        Rule::word => Token::Word(pair.as_str().to_string()),
        Rule::number => Token::Number(pair.as_str().parse().unwrap()),
        // Rule::string => {
        //     let s = pair.as_str();
        //     Token::String(s[1..s.len() - 1].to_string()) // убираем внешние кавычки
        // }
        Rule::op => Token::Op(pair.as_str().to_string()),
        Rule::paren_group => Token::Group('(', parse_sentences(pair)),
        Rule::bracket_group => Token::Group('[', parse_sentences(pair)),
        Rule::brace_group => Token::Group('{', parse_sentences(pair)),
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
    let desugared = desugar_python_indent(source);
    let pairs = LangParser::parse(Rule::program, &desugared)
        .expect("Ошибка синтаксического анализа")
        .next()
        .unwrap();

    parse_sentences(pairs)
}

// ---------------------------------------------------------
// 4. Пример «нарезки»: интерпретатор/обработчик через match
// ---------------------------------------------------------
fn run_program(sentences: &[Sentence]) {
    for sentence in sentences {
        match sentence.as_slice() {
            // let x = 10
            [Token::Word(w), Token::Word(name), Token::Op(eq), val]
            if w == "let" && eq == "=" =>
                {
                    println!("[ПЕРЕМЕННАЯ] Объявлена: {} со значением {:?}", name, val);
                }

            // def name(args): блок
            // или fn name(args) { блок }
            [Token::Word(kw), Token::Word(name), Token::Group('(', args), Token::Group('{', body)]
            if kw == "def" || kw == "fn" =>
                {
                    println!("[ФУНКЦИЯ] Имя: '{}'", name);
                    println!("          Аргументы: {:?}", args);
                    println!("          Тело:");
                    run_program(body); // рекурсивно выполняем тело
                }

            // if cond: блок
            [Token::Word(kw), cond @ .., Token::Group('{', body)] if kw == "if" => {
                println!("[IF ВЕТКА] Условие: {:?}", cond);
                run_program(body);
            }

            // print "строка" / log x
            [Token::Word(cmd), args @ ..] => {
                println!("[ВЫЗОВ КОМАНДЫ] {} с аргументами: {:?}", cmd, args);
            }

            _ => println!("[ДРУГОЕ] {:?}", sentence),
        }
    }
}

// ---------------------------------------------------------
// 5. Тестирование
// ---------------------------------------------------------
fn main() {
    // Входной код в стиле ПИТОНА (с отступами и двоеточиями)
    let python_code = r#"
# Это глобальный скрипт
let global_var = 100

def test_handler(x, y):
    let local = 42
    if x > 10:
        print "x is greater!"
        log local

print "finished"
"#;

    println!("--- 1. ИСХОДНЫЙ КОД ---");
    println!("{}", python_code);

    println!("--- 2. ПАРСИНГ В УНИВЕРСАЛЬНОЕ AST ---");
    let ast = parse_code(python_code);
    println!("{:#?}", ast);

    println!("\n--- 3. НАРЕЗКА И ВЫПОЛНЕНИЕ ---");
    run_program(&ast);
}