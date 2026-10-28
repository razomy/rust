use razomy_parser::{analyze_ast, parse_code};

fn main() {
    let code = r#"
Identifiable { Id String }
User Identifiable {
  Id   String
  Name String
}
"#;
    let ast = parse_code(code).expect("Ошибка парсинга");
    analyze_ast(&ast, 0);
}
