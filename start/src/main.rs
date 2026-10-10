mod parser_tests;

use std::fs;
use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pair;

#[derive(Parser)]
#[grammar = "rl.pest"]
pub struct RazomyParser;

// Рекурсивная функция для обхода дерева
fn format_tree(pair: Pair<Rule>, depth: usize, output: &mut String) {
    let indent = "  ".repeat(depth);
    let rule = pair.as_rule();

    // Получаем дочерние элементы
    let inner: Vec<Pair<Rule>> = pair.clone().into_inner().collect();

    if inner.is_empty() {
        // Листовой узел (терминал): выводим правило и само значение текста
        let text = pair.as_str().trim();
        output.push_str(&format!("{}- {:?}: {:?}\n", indent, rule, text));
    } else {
        // Промежуточный узел: выводим название правила и рекурсивно идем внутрь
        output.push_str(&format!("{}- {:?}\n", indent, rule));
        for child in inner {
            format_tree(child, depth + 1, output);
        }
    }
}

fn main() {
    let source_path = "./src/example.rl";
    let output_path = "rl-tree.txt";

    let source_code = fs::read_to_string(source_path)
        .expect("Не удалось прочитать файл исходного кода");

    println!("Парсим комплексный код спецификации...");
    match RazomyParser::parse(Rule::Program, &source_code) {
        Ok(parsed_tree) => {
            println!("✅ УСПЕШНО!\n");

            let mut tree_string = String::new();

            // Обходим корневые элементы
            for pair in parsed_tree {
                format_tree(pair, 0, &mut tree_string);
            }

            // 1. Сохраняем дерево в файл
            fs::write(output_path, &tree_string)
                .expect("Не удалось записать дерево в файл");
            println!("Дерево успешно сохранено в файл: {}", output_path);

            // 2. (Опционально) Если хотите увидеть превью в консоли:
            // println!("\nПредпросмотр дерева:\n{}", tree_string);
        }
        Err(err) => {
            println!("❌ ОШИБКА ПАРСИНГА:\n");
            eprintln!("{}", err);
        }
    }
}