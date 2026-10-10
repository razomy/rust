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

    match RazomyParser::parse(Rule::Program, source_code) {
        Ok(parsed_tree) => {
            println!("✅ УСПЕШНО!\n");
            for pair in parsed_tree {
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::Decl => {
                            for decl_inner in inner.into_inner() {
                                println!("Разобрано объявление: {:?}", decl_inner.as_rule());
                            }
                        }
                        Rule::EOI => {}
                        other => println!("Элемент: {:?}", other),
                    }
                }
            }
        }
        Err(err) => {
            println!("❌ ОШИБКА ПАРСИНГА:\n");
            eprintln!("{}", err);
        }
    }
}