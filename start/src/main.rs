use std::fs;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rl.pest"]
pub struct RazomyParser;

fn main() {
    let source_code =fs::read_to_string("./src/example.rl").unwrap();

    println!("Парсим комплексный код спецификации...");
    match RazomyParser::parse(Rule::Program, &source_code) {
        Ok(parsed_tree) => {
            println!("✅ УСПЕШНО!\n");
            for pair in parsed_tree {
                for inner in pair.into_inner() {
                    if inner.as_rule() == Rule::Decl {
                        for decl in inner.into_inner() {
                            println!("Объявление: {:?}", decl.as_rule());
                        }
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