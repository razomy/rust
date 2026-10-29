use pest::Parser;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};

use razomy_language::analyzer::{analyze_source, AnalysisReport, Severity};
use razomy_language::hir::is_pascal_case;
use razomy_language::lsp::RazomyLanguageServer;
use razomy_language::parser::{RazomyParser, Rule};
#[test]
fn test_lsp_lifecycle_and_features() {
    let mut server = RazomyLanguageServer::new();
    let mut out = Vec::new();

    // 1. Initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    server.handle_message(&init_req, &mut out).unwrap();
    assert!(!out.is_empty());

    // 2. Open specification.rl snippet
    out.clear();
    let sample_code = r#"
User { id String, name String, age Int }
greet(User) [ "Welcome, {user.name}!" ]
"#;
    let open_notif = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.rl",
                "version": 1,
                "text": sample_code
            }
        }
    });
    server.handle_message(&open_notif, &mut out).unwrap();

    // 3. UFCS Completion on `User.` (Law 4 should suggest `id`, `name`, `age`, and `greet`!)
    let comp_res = server.on_completion(&json!({
        "textDocument": { "uri": "file:///test.rl" },
        "position": { "line": 1, "character": 4 }
    }));
    let items = comp_res.get("items").and_then(JsonValue::as_array).unwrap();
    assert!(!items.is_empty());

    // 4. Hover on `User`
    let hover_res = server.on_hover(&json!({
        "textDocument": { "uri": "file:///test.rl" },
        "position": { "line": 1, "character": 2 }
    }));
    assert!(hover_res.get("contents").is_some());

    // 5. Document Symbols
    let sym_res = server.on_document_symbol(&json!({
        "textDocument": { "uri": "file:///test.rl" }
    }));
    assert_eq!(sym_res.as_array().unwrap().len(), 2);
}

fn open_doc(server: &mut RazomyLanguageServer, uri: &str, code: &str) -> Vec<JsonValue> {
    let mut out = Vec::new();
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "version": 1,
                "text": code
            }
        }
    });
    server.handle_message(&notif, &mut out).unwrap();
    let mut cursor = io::Cursor::new(out);
    if let Ok(Some(msg)) = razomy_language::lsp::read_rpc_message(&mut cursor) {
        msg.get("params")
            .and_then(|p| p.get("diagnostics"))
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default()
    } else {
        vec![]
    }
}

fn completion_labels(server: &RazomyLanguageServer, uri: &str, line: usize, character: usize) -> Vec<String> {
    let res = server.on_completion(&json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character }
    }));
    res.get("items")
        .and_then(JsonValue::as_array)
        .unwrap()
        .iter()
        .filter_map(|it| it.get("label").and_then(JsonValue::as_str).map(str::to_string))
        .collect()
}

#[test]
fn test_lsp_dot_completion_while_typing_with_syntax_recovery() {
    let mut server = RazomyLanguageServer::new();
    // Обратите внимание на висячие точки `my_car.`, `self.`, `u.`, `state.err.`, `order_1.total.` прямо в коде!
    let code = r#"Vehicle { brand String, max_speed Int 180 }
Car Vehicle { brand "Generic", max_speed 200 }
SportsCar Car { brand "Porsche", max_speed 300 }
MyGarageCar SportsCar & { garage_slot 1 }

User { id String, name String, age Int }
migrate_user(User) [ user.name ]

CompileError { code Int, log String } & Bypass
AgentState { err CompileError, retries Int }

Invoice {
    price Money"100.00"
    quantity 3.0
    subtotal [ self. ]
    total [ self.subtotal ]
}
order_1 Invoice & { quantity 5.0 }

demo(u User, state AgentState) [
    my_car (MyGarageCar {})
    my_car.
    u.
    state.err.
    order_1.total.
]
"#;
    open_doc(&mut server, "file:///typing.rl", code);

    // 1. Внутри `Invoice`: `self.` (строка 14, символ 20)
    let self_labels = completion_labels(&server, "file:///typing.rl", 14, 20);
    assert!(self_labels.contains(&"price".to_string()), "self. labels: {self_labels:?}");
    assert!(self_labels.contains(&"quantity".to_string()));

    // 2. На локальном экземпляре `my_car.` (строка 21, символ 11) -> поля MyGarageCar, предки и UFCS
    let car_labels = completion_labels(&server, "file:///typing.rl", 21, 11);
    assert!(car_labels.contains(&"brand".to_string()), "my_car. labels: {car_labels:?}");
    assert!(car_labels.contains(&"max_speed".to_string()));
    assert!(car_labels.contains(&"garage_slot".to_string()));
    assert!(car_labels.contains(&"SportsCar".to_string()));
    assert!(car_labels.contains(&"Car".to_string()));

    // 3. На параметре функции `u.` (строка 22, символ 6) -> поля User + UFCS функция `migrate_user`!
    let u_labels = completion_labels(&server, "file:///typing.rl", 22, 6);
    assert!(u_labels.contains(&"id".to_string()), "u. labels: {u_labels:?}");
    assert!(u_labels.contains(&"name".to_string()));
    assert!(u_labels.contains(&"age".to_string()));
    assert!(u_labels.contains(&"migrate_user".to_string()));

    // 4. На вложенной цепочке `state.err.` (строка 23, символ 14) -> поля CompileError (`code`, `log`)
    let err_labels = completion_labels(&server, "file:///typing.rl", 23, 14);
    assert!(err_labels.contains(&"code".to_string()), "state.err. labels: {err_labels:?}");
    assert!(err_labels.contains(&"log".to_string()));

    // 5. На реактивном поле `order_1.total.` (строка 24, символ 18) -> выведен тип `Money`, предлагает `scaled`, `from_scaled`!
    let money_labels = completion_labels(&server, "file:///typing.rl", 24, 18);
    assert!(money_labels.contains(&"scaled".to_string()), "order_1.total. labels: {money_labels:?}");
    assert!(money_labels.contains(&"from_scaled".to_string()));
}

#[test]
fn test_lsp_ufcs_spaced_pipe_and_collection_completions() {
    let mut server = RazomyLanguageServer::new();
    let code = r#"User { id String, name String }
Item { title String, price Int }
greet_user(User) [ "Hi {user.name}" ]

pipeline_demo(u User, items Tuple(Item)) [
    u . 
    items . 
]
"#;
    open_doc(&mut server, "file:///pipe.rl", code);

    // 1. Пробельный пайп `u . ` (строка 5, символ 8) -> предлагает `greet_user` и поля `User`
    let u_pipe = completion_labels(&server, "file:///pipe.rl", 5, 8);
    assert!(u_pipe.contains(&"greet_user".to_string()), "u . labels: {u_pipe:?}");
    assert!(u_pipe.contains(&"name".to_string()));

    // 2. Пробельный пайп на коллекции `items . ` (строка 6, символ 12) -> предлагает `map`, `filter`, `fold`, `zip`, `enumerate`
    let col_pipe = completion_labels(&server, "file:///pipe.rl", 6, 12);
    assert!(col_pipe.contains(&"map".to_string()), "items . labels: {col_pipe:?}");
    assert!(col_pipe.contains(&"filter".to_string()));
    assert!(col_pipe.contains(&"fold".to_string()));
    assert!(col_pipe.contains(&"zip".to_string()));
    assert!(col_pipe.contains(&"enumerate".to_string()));
}

#[test]
fn test_lsp_struct_init_and_router_branch_completion() {
    let mut server = RazomyLanguageServer::new();
    let code = r#"User { id String, name String, age Int }
Circle { radius Float }
Rect { width Float, height Float }
Geometry Circle | Rect

test_ctx(g Geometry) [
    u User {  }
    g . {  }
]
"#;
    open_doc(&mut server, "file:///ctx.rl", code);

    // 1. Внутри `User {  }` (строка 6, символ 13) -> предлагает поля `id`, `name`, `age`
    let struct_fields = completion_labels(&server, "file:///ctx.rl", 6, 13);
    assert!(struct_fields.contains(&"id".to_string()), "User {{}} labels: {struct_fields:?}");
    assert!(struct_fields.contains(&"name".to_string()));
    assert!(struct_fields.contains(&"age".to_string()));

    // 2. Внутри роутера `g . {  }` (строка 7, символ 10) -> предлагает ветки Union `Circle`, `Rect`
    let router_branches = completion_labels(&server, "file:///ctx.rl", 7, 10);
    assert!(router_branches.contains(&"Circle".to_string()), "g . {{}} labels: {router_branches:?}");
    assert!(router_branches.contains(&"Rect".to_string()));
}

#[test]
fn test_lsp_type_and_instance_linking_hover_and_go_to_definition() {
    let mut server = RazomyLanguageServer::new();
    let code = r#"Identifiable { id String }
User Identifiable { name String, age Int }
Circle { radius Float }
Rect { width Float, height Float }
Geometry Circle | Rect

area(g Geometry) [
    my_user (User { id "u1", name "Alice", age 25 })
    my_user.id
    my_user.name
    g . {
        Circle [ 3.14 * circle.radius * circle.radius ]
        Rect   [ rect.width * rect.height ]
    }
]
"#;
    open_doc(&mut server, "file:///linking.rl", code);

    // 1. Hover на локальной переменной-экземпляре `my_user` (строка 7, символ 6)
    let hover_my_user = server.on_hover(&json!({
        "textDocument": { "uri": "file:///linking.rl" },
        "position": { "line": 7, "character": 6 }
    }));
    let hover_text = hover_my_user["contents"]["value"].as_str().unwrap_or("");
    assert!(hover_text.contains("my_user : User"), "hover_my_user: {hover_text}");
    assert!(hover_text.contains("Form User"));

    // 2. Hover на переменной ветки роутера `circle` (строка 11, символ 26)
    let hover_circle = server.on_hover(&json!({
        "textDocument": { "uri": "file:///linking.rl" },
        "position": { "line": 11, "character": 26 }
    }));
    let circle_text = hover_circle["contents"]["value"].as_str().unwrap_or("");
    assert!(circle_text.contains("circle : Circle"), "hover_circle: {circle_text}");

    // 3. Hover на поле `circle.radius` (строка 11, символ 34)
    let hover_radius = server.on_hover(&json!({
        "textDocument": { "uri": "file:///linking.rl" },
        "position": { "line": 11, "character": 34 }
    }));
    let radius_text = hover_radius["contents"]["value"].as_str().unwrap_or("");
    assert!(radius_text.contains("Circle.radius: Float"), "hover_radius: {radius_text}");

    // 4. Go to Definition на `my_user.id` (строка 8, символ 13) -> прыгает в `Identifiable` (строка 0), откуда унаследовано `id`!
    let def_id = server.on_definition(&json!({
        "textDocument": { "uri": "file:///linking.rl" },
        "position": { "line": 8, "character": 13 }
    }));
    assert_eq!(def_id["range"]["start"]["line"].as_u64(), Some(0));

    // 5. Go to Definition на `my_user.name` (строка 9, символ 14) -> прыгает в `User` (строка 1)!
    let def_name = server.on_definition(&json!({
        "textDocument": { "uri": "file:///linking.rl" },
        "position": { "line": 9, "character": 14 }
    }));
    assert_eq!(def_name["range"]["start"]["line"].as_u64(), Some(1));
}

#[test]
fn test_lsp_diagnostics_error_highlighting_and_quickfix_code_actions() {
    let mut server = RazomyLanguageServer::new();
    let code = r#"Circle { radius Float }
User { id String, name String, age Int }
Cloud { id String, name String, region String }
Car { brand String, max_speed Int }
Order { id String, amount Int }
Reserved { reserved_at Int }
Paid { transaction_id String }

area(Circle) [ 3.14 * circle.radius * circle.radius ]
ship_to_customer(order Order & Reserved & Paid) [ order.id ]

test_errors(u User, entity User & Cloud) [
    area (Circle { radius 5.0 })
    Car.max_speed 500
    entity.name
    u.naem
    bad_user User { id 12345, name "Alice", age 25 }
    raw_order Order { id "o1", amount 100 }
    ship_to_customer(raw_order)
]
"#;
    let diags = open_doc(&mut server, "file:///errors.rl", code);
    let codes: Vec<&str> = diags
        .iter()
        .filter_map(|d| d.get("code").and_then(JsonValue::as_str))
        .collect();

    assert!(codes.contains(&"E01_AMBIGUOUS_CALL_OR_BIND"), "codes: {codes:?}");
    assert!(codes.contains(&"E06_IMMUTABLE_TYPE_BINDING"), "codes: {codes:?}");
    assert!(codes.contains(&"E05_AMBIGUOUS_LINEAGE_FIELD"), "codes: {codes:?}");
    assert!(codes.contains(&"E08_UNKNOWN_MEMBER"), "codes: {codes:?}");
    assert!(codes.contains(&"E09_TYPE_MISMATCH"), "codes: {codes:?}");
    assert!(codes.contains(&"E003_TYPESTATE_VIOLATION"), "codes: {codes:?}");

    // Проверяем, что ошибка E06 подсвечивает именно строку с `Car.max_speed 500` (строка 13), а не заголовок функции `test_errors`!
    let e06_diag = diags
        .iter()
        .find(|d| d.get("code").and_then(JsonValue::as_str) == Some("E06_IMMUTABLE_TYPE_BINDING"))
        .unwrap();
    assert_eq!(e06_diag["range"]["start"]["line"].as_u64(), Some(13));

    // Проверяем QuickFix Code Actions (для E01, E05 и опечатки E08 `u.naem` -> `u.name`)
    let actions = server
        .on_code_action(&json!({
            "textDocument": { "uri": "file:///errors.rl" }
        }))
        .as_array()
        .unwrap()
        .clone();
    let titles: Vec<&str> = actions
        .iter()
        .filter_map(|a| a.get("title").and_then(JsonValue::as_str))
        .collect();
    assert!(
        titles.iter().any(|t| t.contains("u.name")),
        "Ожидался QuickFix для опечатки u.naem -> u.name, получено: {titles:?}"
    );
}

#[test]
fn test_lsp_inlay_hints_signature_help_semantic_tokens_and_rename() {
    let mut server = RazomyLanguageServer::new();
    let code = r#"User { id String, name String }
Db {
    find_user(id String, include_deleted Bool) User [
        User { id id, name "Alice" }
    ]
}

process(uid String) [
    found (Db.find_user(uid, false))
    found
]
"#;
    open_doc(&mut server, "file:///extra.rl", code);

    // 1. Inlay Hints (включая выведенный возвращаемый тип `-> User` для `process` и `: User` для `found`)
    let hints = server
        .on_inlay_hints(&json!({
            "textDocument": { "uri": "file:///extra.rl" }
        }))
        .as_array()
        .unwrap()
        .clone();
    let hint_labels: Vec<&str> = hints
        .iter()
        .filter_map(|h| h.get("label").and_then(JsonValue::as_str))
        .collect();
    assert!(
        hint_labels.iter().any(|l| l.contains("User")),
        "Inlay hints: {hint_labels:?}"
    );

    // 2. Signature Help на вызове метода `Db.find_user(uid, ` (строка 8, символ 29 -> активный параметр #1)
    let sig_help = server.on_signature_help(&json!({
        "textDocument": { "uri": "file:///extra.rl" },
        "position": { "line": 8, "character": 29 }
    }));
    assert_eq!(sig_help["activeParameter"].as_u64(), Some(1));
    assert!(sig_help["signatures"][0]["label"]
        .as_str()
        .unwrap_or("")
        .contains("Db.find_user"));

    // 3. Semantic Tokens
    let sem_tokens = server.on_semantic_tokens(&json!({
        "textDocument": { "uri": "file:///extra.rl" }
    }));
    assert!(!sem_tokens["data"].as_array().unwrap().is_empty());

    // 4. References & Rename для `found` (строка 8, символ 6)
    let refs = server
        .on_references(&json!({
            "textDocument": { "uri": "file:///extra.rl" },
            "position": { "line": 8, "character": 6 }
        }))
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(refs.len(), 2);

    let rename_res = server.on_rename(&json!({
        "textDocument": { "uri": "file:///extra.rl" },
        "position": { "line": 8, "character": 6 },
        "newName": "loaded_user"
    }));
    let edits = rename_res["changes"]["file:///extra.rl"]
        .as_array()
        .unwrap();
    assert_eq!(edits.len(), 2);
}

#[test]
fn test_duplicate_fn_error_unknown_type_and_inherited_ufcs_completion() {
    let mut server = RazomyLanguageServer::new();
    let code = r#"User { id String, name String, age Int }
Cat User
Dog { id String }

greet(User) [ "Привет, {user.name}!" ]
greet2(Cat) [ "Привет, {cat.name}!" ]
greet2(Cat) [ "Дубликат!" ]
bark(Dog) [ "Woof!" ]

bad_fn(TypoUser) [ "Oops" ]

main() [
    u User { id "1", name "Алексей", age 30 }
    cat Cat { id "1", name "DD", age 30 }
    cat .
]
"#;
    let diags = open_doc(&mut server, "file:///user_cases.rl", code);
    let codes: Vec<&str> = diags
        .iter()
        .filter_map(|d| d.get("code").and_then(JsonValue::as_str))
        .collect();

    // 1. Дубликат функции greet2 должен выдать E10_DUPLICATE_DEFINITION на строке 6
    assert!(
        codes.contains(&"E10_DUPLICATE_DEFINITION"),
        "Expected E10_DUPLICATE_DEFINITION, got: {codes:?}"
    );

    // 2. Неизвестный тип TypoUser в bad_fn(TypoUser) должен выдать E11_UNKNOWN_TYPE
    assert!(
        codes.contains(&"E11_UNKNOWN_TYPE"),
        "Expected E11_UNKNOWN_TYPE, got: {codes:?}"
    );

    // 3. Автодополнение на `cat .` (строка 14, символ 9 или 10):
    // Должно предлагать поля User (id, name, age), функцию Cat (greet2) и функцию родителя User (greet),
    // но НЕ должно предлагать функцию Dog (bark)!
    let cat_labels = completion_labels(&server, "file:///user_cases.rl", 14, 10);
    assert!(cat_labels.contains(&"id".to_string()), "cat_labels: {cat_labels:?}");
    assert!(cat_labels.contains(&"name".to_string()));
    assert!(cat_labels.contains(&"greet".to_string()), "Must include parent User UFCS method greet");
    assert!(cat_labels.contains(&"greet2".to_string()), "Must include own Cat UFCS method greet2");
    assert!(!cat_labels.contains(&"bark".to_string()), "Must NOT include unrelated Dog method bark");
}

#[test]
fn test_all_dot_and_pipe_completion_styles_with_and_without_spaces() {
    let mut server = RazomyLanguageServer::new();
    let code = r#"User { id String, name String, age Int }
Cat User
Dog { id String }

greet(User) [ "Hello {user.name}" ]
greet_cat(Cat) [ "Meow {cat.name}" ]
bark(Dog) [ "Woof" ]

main() [
    cat Cat { id "1", name "Murzik", age 3 }
    cat.
    cat .
    cat . 
    cat.gr
    cat .gr
    cat . gr
]
"#;
    open_doc(&mut server, "file:///dot_styles.rl", code);

    // 1. Слитная точка `cat.` (строка 10, символ 8)
    let tight_dot = completion_labels(&server, "file:///dot_styles.rl", 10, 8);
    assert!(tight_dot.contains(&"name".to_string()), "tight_dot: {tight_dot:?}");
    assert!(tight_dot.contains(&"greet".to_string()));
    assert!(tight_dot.contains(&"greet_cat".to_string()));
    assert!(!tight_dot.contains(&"bark".to_string()));

    // 2. Пробельный пайп без пробела после точки `cat .` (строка 11, символ 9)
    let spaced_dot_no_trail = completion_labels(&server, "file:///dot_styles.rl", 11, 9);
    assert!(
        spaced_dot_no_trail.contains(&"name".to_string()),
        "spaced_dot_no_trail: {spaced_dot_no_trail:?}"
    );
    assert!(spaced_dot_no_trail.contains(&"greet".to_string()));
    assert!(spaced_dot_no_trail.contains(&"greet_cat".to_string()));
    assert!(!spaced_dot_no_trail.contains(&"bark".to_string()));

    // 3. Пробельный пайп с пробелом после точки `cat . ` (строка 12, символ 10)
    let spaced_dot_with_trail = completion_labels(&server, "file:///dot_styles.rl", 12, 10);
    assert!(spaced_dot_with_trail.contains(&"greet".to_string()));
    assert!(spaced_dot_with_trail.contains(&"greet_cat".to_string()));
    assert!(!spaced_dot_with_trail.contains(&"bark".to_string()));

    // 4. Фильтрация по префиксу: `cat.gr`, `cat .gr`, `cat . gr` (строки 13, 14, 15)
    for (line, col) in [(13, 10), (14, 11), (15, 12)] {
        let filtered = completion_labels(&server, "file:///dot_styles.rl", line, col);
        assert!(
            filtered.contains(&"greet".to_string()),
            "line {line} filtered: {filtered:?}"
        );
        assert!(filtered.contains(&"greet_cat".to_string()));
        assert!(!filtered.contains(&"name".to_string()));
        assert!(!filtered.contains(&"bark".to_string()));
    }
}

#[test]
fn test_cross_file_go_to_definition_hover_completion_and_references() {
    let mut server = RazomyLanguageServer::new();

    let models_uri = "file:///workspace/models.rl";
    let models_code = r#"Identifiable { id String }
User Identifiable { name String, age Int }
Cat User

greet_user(User) [ "Hello, {user.name}!" ]
greet_cat(Cat) [ "Meow, {cat.name}!" ]
"#;

    let main_uri = "file:///workspace/main.rl";
    let main_code = r#"_ models

LocalDog { id String }
bark(LocalDog) [ "Woof!" ]

main() [
    u User { id "u1", name "Alice", age 28 }
    cat Cat { id "c1", name "Barsik", age 4 }
    u.id
    u.name
    cat . greet_user
    cat . greet_cat
    cat .
]
"#;

    let models_diags = open_doc(&mut server, models_uri, models_code);
    assert!(models_diags.is_empty(), "models_diags: {models_diags:?}");

    let main_diags = open_doc(&mut server, main_uri, main_code);
    assert!(main_diags.is_empty(), "main_diags: {main_diags:?}");

    // 1. Go to Definition на импорте `_ models` (строка 0, символ 3 в main.rl) -> прыгает в models.rl!
    let def_import = server.on_definition(&json!({
        "textDocument": { "uri": main_uri },
        "position": { "line": 0, "character": 3 }
    }));
    assert_eq!(def_import["uri"].as_str(), Some(models_uri));

    // 2. Go to Definition на типе `User` в `u User { ... }` (строка 6, символ 7 в main.rl) -> прыгает в models.rl (строка 1)!
    let def_user_type = server.on_definition(&json!({
        "textDocument": { "uri": main_uri },
        "position": { "line": 6, "character": 7 }
    }));
    assert_eq!(def_user_type["uri"].as_str(), Some(models_uri));
    assert_eq!(def_user_type["range"]["start"]["line"].as_u64(), Some(1));

    // 3. Go to Definition на поле `name` внутри инициализации `User { id "u1", name "Alice" }` (строка 6, символ 24) -> models.rl (строка 1)!
    let def_init_field = server.on_definition(&json!({
        "textDocument": { "uri": main_uri },
        "position": { "line": 6, "character": 24 }
    }));
    assert_eq!(def_init_field["uri"].as_str(), Some(models_uri));
    assert_eq!(def_init_field["range"]["start"]["line"].as_u64(), Some(1));

    // 4. Go to Definition на унаследованном поле `u.id` (строка 8, символ 7 в main.rl) -> прыгает в `Identifiable` в models.rl (строка 0)!
    let def_u_id = server.on_definition(&json!({
        "textDocument": { "uri": main_uri },
        "position": { "line": 8, "character": 7 }
    }));
    assert_eq!(def_u_id["uri"].as_str(), Some(models_uri));
    assert_eq!(def_u_id["range"]["start"]["line"].as_u64(), Some(0));

    // 5. Go to Definition на поле `u.name` (строка 9, символ 7 в main.rl) -> прыгает в `User` в models.rl (строка 1)!
    let def_u_name = server.on_definition(&json!({
        "textDocument": { "uri": main_uri },
        "position": { "line": 9, "character": 7 }
    }));
    assert_eq!(def_u_name["uri"].as_str(), Some(models_uri));
    assert_eq!(def_u_name["range"]["start"]["line"].as_u64(), Some(1));

    // 6. Go to Definition на UFCS функции `cat . greet_user` (строка 10, символ 13 в main.rl) -> прыгает в `greet_user` в models.rl (строка 4)!
    let def_ufcs = server.on_definition(&json!({
        "textDocument": { "uri": main_uri },
        "position": { "line": 10, "character": 13 }
    }));
    assert_eq!(def_ufcs["uri"].as_str(), Some(models_uri));
    assert_eq!(def_ufcs["range"]["start"]["line"].as_u64(), Some(4));

    // 7. Межфайловый UFCS Autocomplete на `cat .` (строка 12, символ 9 в main.rl):
    // Должен включать поля из models.rl (`id`, `name`, `age`) и функции `greet_user`, `greet_cat`, но НЕ `bark`!
    let cat_comp = completion_labels(&server, main_uri, 12, 9);
    assert!(cat_comp.contains(&"id".to_string()), "cat_comp: {cat_comp:?}");
    assert!(cat_comp.contains(&"name".to_string()));
    assert!(cat_comp.contains(&"greet_user".to_string()));
    assert!(cat_comp.contains(&"greet_cat".to_string()));
    assert!(!cat_comp.contains(&"bark".to_string()));

    // 8. Межфайловый Find References на `User` (включает вхождения и в models.rl, и в main.rl)
    let refs = server
        .on_references(&json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": 6, "character": 7 }
        }))
        .as_array()
        .unwrap()
        .clone();
    let ref_uris: Vec<&str> = refs
        .iter()
        .filter_map(|r| r.get("uri").and_then(JsonValue::as_str))
        .collect();
    assert!(ref_uris.contains(&models_uri), "ref_uris: {ref_uris:?}");
    assert!(ref_uris.contains(&main_uri), "ref_uris: {ref_uris:?}");
}

#[test]
fn test_cross_file_live_in_memory_edit_updates_dependent_file_diagnostics() {
    let mut server = RazomyLanguageServer::new();

    let models_uri = "file:///live_project/models.rl";
    let main_uri = "file:///live_project/main.rl";

    open_doc(
        &mut server,
        models_uri,
        "User { id String, name String }\n",
    );
    let initial_main_diags = open_doc(
        &mut server,
        main_uri,
        "_ models\nmain() [\n    u User { id \"1\", name \"Alice\" }\n    u.name\n]\n",
    );
    assert!(initial_main_diags.is_empty());

    // Редактируем `models.rl` в памяти IDE (переименовываем `User` -> `User2` без сохранения на диск!)
    let mut out = Vec::new();
    let change_notif = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": models_uri, "version": 2 },
            "contentChanges": [{ "text": "User2 { id String, full_name String }\n" }]
        }
    });
    server.handle_message(&change_notif, &mut out).unwrap();

    // Проверяем, что `main.rl` автоматически пересчитался и теперь видит ошибку E11_UNKNOWN_TYPE для старого `User`!
    let main_doc = server.documents.get(main_uri).unwrap();
    assert!(
        main_doc
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "E11_UNKNOWN_TYPE"),
        "Expected E11_UNKNOWN_TYPE in main.rl after renaming User -> User2 in models.rl, got: {:?}",
        main_doc.report.diagnostics
    );
}