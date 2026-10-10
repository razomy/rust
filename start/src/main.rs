use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rl.pest"]
pub struct RazomyParser;

fn main() {
    let source_code = r#"
# Объявляем тип как в Python (без скобок и запятых)
type User
    name Str
    age Int

# Объявляем функцию (знак = опционален, тело идет отступами)
out fn main()
    let mut counter = 0
    counter = counter + 1

    if counter > 0
        print("Счетчик больше нуля!")
    else
        print("Ноль")
    "#;

    println!("Парсим код:\n{}", source_code);

    match RazomyParser::parse(Rule::Program, source_code) {
        Ok(parsed_tree) => {
            println!("✅ УСПЕШНО!\n");
            // Выведем только верхний уровень, чтобы не захламлять консоль
            for pair in parsed_tree {
                println!("Разобрано: {:?}", pair.as_rule());
                for inner in pair.into_inner() {
                    println!("  -> {:?}", inner.as_rule());
                }
            }
        }
        Err(err) => {
            println!("❌ ОШИБКА ПАРСИНГА:\n");
            eprintln!("{}", err);
        }
    }
}