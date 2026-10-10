use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rl.pest"]
pub struct RazomyParser;

fn main() {
    let source_code = r#"
import console

type Role = Admin | User(id Int) | Guest
type Person { name Str, age Int, role Role }

fn check_access(person Person) Result(Str, Str) =
    match person.role {
        Admin = Ok("Полный доступ")
        User(id) if person.age >= 18 = Ok("Доступ для ID: ${id}")
        User(_) = Err("Пользователь слишком молод")
        Guest = Err("Гостям вход запрещен")
    }

out fn main() =
    let users = [
        { name "Алиса", age 25, role Admin },
        { name "Боб",  age 16, role User(101) },
        { name "Ева",  age 22, role Guest }
    ]

    # Обработка данных через чейнинг
    users
        .map((u) = check_access(u))
        .map((res) = match res {
            Ok(msg)  = "✅ ${msg}"
            Err(err) = "❌ Отказ: ${err}"
        })
        .iter((log) = console.print(log))

    # Пример работы с отсутствующим значением (none)
    let config_path Str? = none

    let path = match config_path {
        Some(p) = p
        none = "/default/path.conf"
    }

    console.print("Используется путь: ${path}")
    "#;

    println!("Парсим комплексный код спецификации...");

    match RazomyParser::parse(Rule::Program, source_code) {
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