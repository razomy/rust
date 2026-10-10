use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rl.pest"]
pub struct RazomyParser;

/// Рекурсивное форматирование CST-дерева в человекочитаемый вид
fn format_tree(pair: Pair<Rule>, depth: usize, output: &mut String) {
    let rule = pair.as_rule();

    // Пропускаем служебный узел конца файла
    if rule == Rule::EOI {
        return;
    }

    let indent = "  ".repeat(depth);
    let inner: Vec<Pair<Rule>> = pair.clone().into_inner().collect();

    if inner.is_empty() {
        let text = pair.as_str().trim();
        output.push_str(&format!("{}- {:?}: {:?}\n", indent, rule, text));
    } else {
        output.push_str(&format!("{}- {:?}\n", indent, rule));
        for child in inner {
            format_tree(child, depth + 1, output);
        }
    }
}

/// Парсит исходный код и возвращает строку с деревом, либо паникует с описанием ошибки
fn parse_to_tree_str(src: &str) -> String {
    let pairs = RazomyParser::parse(Rule::Program, src)
        .unwrap_or_else(|e| panic!("Не удалось распарсить код:\n{}\nОшибка:\n{}", src, e));

    let mut tree_string = String::new();
    for pair in pairs {
        format_tree(pair, 0, &mut tree_string);
    }
    tree_string
}

// =============================================================================
// МАКРОСЫ ТЕСТИРОВАНИЯ
// =============================================================================

// Парсит код и сравнивает дерево со снапшотом insta
macro_rules! parse_tree {
    ($name:ident, $src:expr) => {
        #[test]
        fn $name() {
            let tree = parse_to_tree_str($src);
            // ВЫВОДИМ ДЕРЕВО В КОНСОЛЬ:
            println!("\n=== [{}] ===\n{}\n{}",  stringify!($name), stringify!($src), tree);
            insta::assert_snapshot!(tree);
        }
    };
}

// Проверяет, что код вызывает ошибку синтаксиса
macro_rules! parse_fail {
    ($name:ident, $src:expr) => {
        #[test]
        fn $name() {
            let res = RazomyParser::parse(Rule::Program, $src);
            assert!(
                res.is_err(),
                "Ожидалась ошибка парсинга для [{}], но код успешно распарсился:\n{}",
                stringify!($name),
                $src
            );
        }
    };
}

// =============================================================================
// 1. КОММЕНТАРИИ (10 тестов)
// =============================================================================
mod comments {
    use super::*;
    parse_tree!(single_empty, "#");
    parse_tree!(single_line_empty, "#\n");
    parse_tree!(single_line_text, "# простой комментарий\n");
    parse_tree!(single_line_no_space_text, "#простой комментарий\n");
    parse_tree!(single_line_with_space_code, "vr a = 1# комментарий в конце\n");
    parse_tree!(single_line_with_code, "vr a = 1 # комментарий в конце\n");
    parse_tree!(multi_line_empty_inline, "####");
    parse_tree!(multi_line_empty, "##\n##\n");
    parse_tree!(multi_line_content, "##\n  Первая строка\n  Вторая строка\n##\n");
    parse_tree!(multi_line_inline, "## комментарий ## vr x = 1\n");
    parse_tree!(multi_line_nested_hashes, "##\n  # внутри многострочного\n##\n");
    parse_tree!(comment_between_code, "vr a = 1\n# пауза\nvr b = 2\n");
    parse_tree!(comment_with_symbols, "# !@#$%^&*()_+-=[]{}|;':\",./<>\n");
    parse_tree!(consecutive_comments, "# один\n# два\n# три\n");
}

// =============================================================================
// 2. ИМПОРТЫ (15 тестов)
// =============================================================================
mod imports {
    use super::*;
    parse_tree!(simple_im, "im console\n");
    parse_tree!(simple_im_dotted, "im std.io\n");
    parse_tree!(im_with_path, "im math = std.math");
    parse_tree!(im_with_path_str, "im math = \"std/math\"\n");
    parse_tree!(im_with_relative_path, "im local = \"./utils/helper\"\n");
    parse_tree!(im_destructure_single, "im { log } = \"std/logger\"\n");
    parse_tree!(im_destructure_multiple, "im { log, warn, error } = \"std/logger\"\n");
    parse_tree!(im_destructure_trailing_comma, "im { a, b, } = \"path\"\n");
    parse_tree!(im_alias_and_destructure, "im { read } = \"fs\"\nim io = \"std/io\"\n");
    parse_tree!(multiple_imports, "im console\nim math = \"std/math\"\nim { log } = \"path\"\n");
    parse_tree!(im_with_deep_path, "im api = \"@company/core/v1/network\"\n");
    parse_tree!(im_destructure_spaces, "im {   x  ,   y   } = \"coords\"\n");
    parse_tree!(im_no_newline, "im console");
    parse_tree!(im_underscore, "im _core = \"core\"\n");
    parse_tree!(im_numbers_in_path, "im v2 = \"api/v2/client\"\n");
    parse_tree!(im_chain, "im a\nim b\nim c\n");
}

// =============================================================================
// 3. ПЕРЕМЕННЫЕ И ДЕСТРУКТУРИЗАЦИЯ (25 тестов)
// =============================================================================
mod variables {
    use super::*;
    parse_tree!(vr_num, "vr x = 1\n");
    parse_tree!(vr_negative_num, "vr x = -42\n");
    parse_tree!(vr_float, "vr pi = 3.1415\n");
    parse_tree!(vr_str, "vr s = \"строка\"\n");
    parse_tree!(vr_bool_true, "vr flag = true\n");
    parse_tree!(vr_bool_false, "vr flag = false\n");
    parse_tree!(vr_typed_int, "vr x Int = 10\n");
    parse_tree!(vr_typed_str, "vr city Str = \"Киев\"\n");
    parse_tree!(vr_typed_custom, "vr user User = alice\n");
    parse_tree!(vr_mut, "vr mut count = 0\n");
    parse_tree!(vr_mut_typed, "vr mut count Int = 0\n");
    parse_tree!(vr_reassign, "vr mut a = 1\na = 2\n");
    parse_tree!(vr_reassign_expr, "vr mut a = 1\na = a + 1\n");
    parse_tree!(vr_record_init, "vr ab = { a 1, b 2 }\n");
    parse_tree!(vr_record_trailing_comma, "vr ab = { a 1, b 2, }\n");
    parse_tree!(vr_record_nested, "vr obj = { a { b 10 } }\n");
    parse_tree!(vr_destructure_pair, "vr { a, b } = ab\n");
    parse_tree!(vr_destructure_pair_array, "vr [ a, b ] = ab\n");
    parse_tree!(vr_destructure_single, "vr { a } = ab\n");
    parse_tree!(vr_destructure_three, "vr { a, b, c } = ab\n");
    parse_tree!(vr_field_access, "vr c = ab.a\n");
    parse_tree!(vr_field_access_deep, "vr v = obj.user.profile.age\n");
    parse_tree!(vr_array_empty, "vr arr = []\n");
    parse_tree!(vr_array_nums, "vr arr = [1, 2, 3]\n");
    parse_tree!(vr_array_trailing, "vr arr = [1, 2, 3,]\n");
    parse_tree!(vr_array_nested, "vr matrix = [[1, 2], [3, 4]]\n");
}

// =============================================================================
// 4. ОБЪЯВЛЕНИЕ ФУНКЦИЙ И ЛЯМБД (30 тестов)
// =============================================================================
mod functions {
    use super::*;
    parse_tree!(fn_empty_params, "fn run() = 1\n");
    parse_tree!(fn_single_arg_eq, "fn square(x Int) = x * x\n");
    parse_tree!(fn_multi_arg_eq, "fn add(a Int, b Int) = a + b\n");
    parse_tree!(fn_multi_arg_typed_ret, "fn add(a Int, b Int) Int = a + b\n");
    parse_tree!(fn_default_param, "fn add(a Int, b Int = 1) Int = a + b\n");
    parse_tree!(fn_no_eq_inline, "fn square(x Int, b Int) x * x\n");
    parse_tree!(fn_no_eq_indent, "fn square(x Int, b Int)\n    x * x\n");
    parse_tree!(fn_body_block, "fn test(x Int) {\n    vr y = x + 1\n    y\n}\n");
    parse_tree!(fn_void_print, "fn log_msg(msg Str)\n    console.print(msg)\n");
    parse_tree!(fn_generic_single, "fn id<T>(val T) T = val\n");
    parse_tree!(fn_generic_constrained, "fn calc<T Number>(x T, b T) = x * x\n");
    parse_tree!(fn_generic_constrained_ret, "fn calc<T Number>(x T, b T) T = x * x\n");
    parse_tree!(fn_generic_multi, "fn pair<T, U>(a T, b U) = { a a, b b }\n");
    parse_tree!(fn_generic_with_defaults, "fn make<T = Int>(val T) T = val\n");
    parse_tree!(lambda_parens, "vr double = (x) = x * 2\n");
    parse_tree!(lambda_short, "vr double = x = x * 2\n");
    parse_tree!(lambda_no_args, "vr get_ten = () = 10\n");
    parse_tree!(lambda_two_args, "vr sum = (a, b) = a + b\n");
    parse_tree!(lambda_block, "vr calc = (x) = {\n    vr y = x * 2\n    y + 1\n}\n");
    parse_tree!(fn_call_no_args, "vr r = run()\n");
    parse_tree!(fn_call_single, "vr r = square(5)\n");
    parse_tree!(fn_call_multi, "vr r = add(5, 10)\n");
    parse_tree!(fn_call_named_field, "vr r = obj.method(1, 2)\n");
    parse_tree!(out_fn_main, "out fn main() = console.print(\"hi\")\n");
    parse_tree!(out_fn_main_block, "out fn main()\n    vr x = 1\n    console.print(x)\n");
    parse_tree!(fn_recursive, "fn fact(n Int) Int = if n <= 1 1 else n * fact(n - 1)\n");
    parse_tree!(fn_nested, "fn outer() = {\n    fn inner() = 42\n    inner()\n}\n");
    parse_tree!(fn_return_record, "fn make_user(name Str) User = { name name, age 0 }\n");
    parse_tree!(fn_complex_signature, "fn handle<T>(id Int, val T, flag Bool = true) Result(T, Str) = Ok(val)\n");
    parse_tree!(fn_currying, "fn add(a Int) = (b) = a + b\n");
}

// =============================================================================
// 5. ТИПЫ, СТРУКТУРЫ, ADT И ДЖЕНЕРИКИ НА ТИПАХ (30 тестов)
// =============================================================================
mod types_and_generics {
    use super::*;
    parse_tree!(record_simple, "type User { name Str, age Int }\n");
    parse_tree!(record_single_field, "type Wrapper { val Int }\n");
    parse_tree!(record_multiline, "type Config {\n    host Str,\n    port Int,\n    debug Bool\n}\n");
    parse_tree!(record_trailing_comma, "type Item { id Int, title Str, }\n");
    parse_tree!(adt_enum_pipe_lead, "type Role =\n    | Admin\n    | User\n    | Guest\n");
    parse_tree!(adt_enum_inline, "type Color = Red | Green | Blue\n");
    parse_tree!(adt_with_data, "type State = | Idle | Loading | Success(User) | Error(Str)\n");
    parse_tree!(adt_payload_multi, "type Event = Click(x Int, y Int) | Key(code Str)\n");
    parse_tree!(adt_payload_record, "type Msg = Data({ id Int, val Str }) | None\n");
    parse_tree!(type_param_simple, "type Container(T) {\n    value T,\n    tag Str\n}\n");
    parse_tree!(type_param_multi, "type Map(K, V) { key K, val V }\n");
    parse_tree!(type_meta_generic, "type Response<TT Real = Number>(T TT = Int) =\n    | Data(T)\n    | Empty\n");
    parse_tree!(type_meta_generic_simple, "type Response<TT>(T TT) = Data(T) | Empty\n");
    parse_tree!(type_alias_simple, "type UserId = Int\n");
    parse_tree!(type_alias_record, "type Point = { x Int, y Int }\n");
    parse_tree!(type_alias_function, "type Callback = (Int) -> Str\n");
    parse_tree!(type_instantiation, "vr c Container(Int) = { value 10, tag \"num\" }\n");
    parse_tree!(type_response_instantiation, "vr r Response(Int) = Data(10)\n");
    parse_tree!(type_union_with_options, "type OptionType = Some(Int) | None\n");
    parse_tree!(record_nested_type_ref, "type Tree { val Int, left Tree?, right Tree? }\n");
    parse_tree!(adt_empty_variants, "type Status = Ready | Done\n");
    parse_tree!(type_with_default_arg, "type Buffer(Size Int = 1024) { data Str }\n");
    parse_tree!(type_record_with_method_field, "type Service { start () -> Unit }\n");
    parse_tree!(type_generic_nested, "type Wrapper(T) { inner Container(T) }\n");
    parse_tree!(record_many_fields, "type Big { a Int, b Int, c Int, d Int, e Int }\n");
    parse_tree!(adt_with_mixed_variants, "type Mixed = Zero | One(Int) | Two(Int, Str) | Full { ok Bool }\n");
    parse_tree!(type_inheritance_like, "type AdminUser { base User, role Str }\n");
    parse_tree!(meta_generic_bounded_only, "type NumberBox<T Number>(item T) { item item }\n");
    parse_tree!(meta_generic_two_bounds, "type Pair<A Number, B Str>(first A, second B) { first A, second B }\n");
    parse_tree!(type_forward_ref, "type A { b B }\ntype B { a A }\n");
}

// =============================================================================
// 6. OPTION И RESULT (NONE, SOME, OK, ERR) (15 тестов)
// =============================================================================
mod option_and_result {
    use super::*;
    parse_tree!(option_type_syntax, "vr u User? = none\n");
    parse_tree!(option_primitive, "vr i Int? = none\n");
    parse_tree!(option_some_val, "vr i Int? = Some(42)\n");
    parse_tree!(option_some_record, "vr u User? = Some({ name \"Алиса\", age 25 })\n");
    parse_tree!(result_type_sig, "fn div(a Int, b Int) Result(Int, Str) = if b == 0 Err(\"err\") else Ok(a / b)\n");
    parse_tree!(result_ok_literal, "vr res = Ok(100)\n");
    parse_tree!(result_err_literal, "vr res = Err(\"ошибка\")\n");
    parse_tree!(result_nested, "vr res Result(Option(Int), Str) = Ok(Some(10))\n");
    parse_tree!(none_in_expr, "vr x = if ready Some(1) else none\n");
    parse_tree!(fn_returns_none, "fn get_none() Int? = none\n");
    parse_tree!(match_on_option, "vr val = match x { Some(v) = v\n none = 0 }\n");
    parse_tree!(match_on_result, "vr val = match res { Ok(v) = v\n Err(e) = -1 }\n");
    parse_tree!(option_chain, "vr name = user?.profile?.name\n");
    parse_tree!(option_coalesce, "vr def = val ?? \"default\"\n");
    parse_tree!(result_try_operator, "vr val = fetch_data()?\n");
}

// =============================================================================
// 7. УПРАВЛЕНИЕ ПОТОКОМ: IF И MATCH (25 тестов)
// =============================================================================
mod control_flow {
    use super::*;
    parse_tree!(if_inline, "vr max = if a > b a else b\n");
    parse_tree!(if_inline_complex_cond, "vr res = if a > 0 && b < 10 \"ok\" else \"fail\"\n");
    parse_tree!(if_block, "vr res = if a > 5 {\n    console.print(\"ok\")\n    \"Ok\"\n} else {\n    \"Fail\"\n}\n");
    parse_tree!(if_else_if_block, "vr res = if a > 0 1 else if a < 0 -1 else 0\n");
    parse_tree!(if_without_else, "if ready console.print(\"go\")\n");
    parse_tree!(if_block_no_else, "if flag {\n    counter = counter + 1\n}\n");
    parse_tree!(match_simple_values, "vr msg = match code {\n    200 = \"OK\"\n    404 = \"Not Found\"\n    _ = \"Unknown\"\n}\n");
    parse_tree!(match_adt_variants, "vr desc = match role {\n    Admin = \"Админ\"\n    User(id) = \"Пользователь\"\n    Guest = \"Гость\"\n}\n");
    parse_tree!(match_guard_condition, "vr desc = match user {\n    User(id) if user.age >= 18 = \"Взрослый\"\n    User(_) = \"Молодой\"\n}\n");
    parse_tree!(match_record_destructure, "vr name = match user {\n    { name, age } if age > 18 = name\n    { name, .. } = name\n}\n");
    parse_tree!(match_rest_fields, "vr r = match obj {\n    { id, .. } = id\n}\n");
    parse_tree!(match_multiline_arms, "vr r = match x {\n    1 = {\n        vr y = 2\n        y\n    }\n    _ = 0\n}\n");
    parse_tree!(match_empty_guards, "vr r = match v { Ok(x) = x\n Err(_) = 0 }\n");
    parse_tree!(if_in_expression, "vr val = 10 + (if flag 5 else 0)\n");
    parse_tree!(nested_if, "vr r = if a if b 1 else 2 else 3\n");
    parse_tree!(nested_match, "vr r = match a {\n    Some(b) = match b { Ok(v) = v\n Err(_) = 0 }\n    none = 0\n}\n");
    parse_tree!(match_boolean, "vr s = match flag { true = \"yes\"\n false = \"no\" }\n");
    parse_tree!(match_strings, "vr s = match action { \"start\" = 1\n \"stop\" = 0\n _ = -1 }\n");
    parse_tree!(match_tuples, "vr s = match (x, y) { (0, 0) = \"origin\"\n (x, 0) = \"x-axis\"\n _ = \"plane\" }\n");
    parse_tree!(match_guard_and_destructure, "vr s = match st { Success({ name, age }) if age > 18 = name\n _ = \"err\" }\n");
    parse_tree!(if_comparison_eq, "vr r = if a == b true else false\n");
    parse_tree!(if_comparison_neq, "vr r = if a != b true else false\n");
    parse_tree!(if_comparison_gte, "vr r = if a >= b true else false\n");
    parse_tree!(if_comparison_lte, "vr r = if a <= b true else false\n");
    parse_tree!(if_negation, "vr r = if !ready \"wait\" else \"go\"\n");
}

// =============================================================================
// 8. ЦИКЛЫ (15 тестов)
// =============================================================================
mod loops {
    use super::*;
    parse_tree!(for_in_collection_indent, "for n in names\n    console.print(n)\n");
    parse_tree!(for_in_collection_block, "for n in names {\n    console.print(n)\n}\n");
    parse_tree!(for_in_range, "for i in 1..5 {\n    console.print(i)\n}\n");
    parse_tree!(for_in_range_inclusive, "for i in 1..=5 console.print(i)\n");
    parse_tree!(for_in_range_vars, "for i in start..end console.print(i)\n");
    parse_tree!(for_destructure_tuple, "for (k, v) in map.entries() console.print(k)\n");
    parse_tree!(for_destructure_record, "for { name, age } in users console.print(name)\n");
    parse_tree!(while_block, "while x > 0 {\n    x = x - 1\n}\n");
    parse_tree!(while_indent, "while x > 0\n    x = x - 1\n");
    parse_tree!(while_compound_condition, "while x > 0 && !done {\n    run()\n}\n");
    parse_tree!(nested_loops, "for x in rows {\n    for y in cols {\n        print(y)\n    }\n}\n");
    parse_tree!(loop_with_break, "while true {\n    if done break\n}\n");
    parse_tree!(loop_with_continue, "for i in 1..10 {\n    if i == 5 continue\n    print(i)\n}\n");
    parse_tree!(for_over_pipeline, "for x in list.filter(pred) {\n    print(x)\n}\n");
    parse_tree!(infinite_loop, "while true { work() }\n");
}

// =============================================================================
// 9. ЧЕЙНИНГ, ПАЙПЛАЙНЫ И ВЫРАЖЕНИЯ (20 тестов)
// =============================================================================
mod pipelines_and_exprs {
    use super::*;
    parse_tree!(pipe_single_map, "vr r = numbers.map(x = x * 2)\n");
    parse_tree!(pipe_map_filter_sum, "vr r = numbers\n    .filter((n) = n % 2 == 0)\n    .map(n = n * 10)\n    .sum()\n");
    parse_tree!(pipe_multiline_indent, "vr r = data\n    .filter(is_valid)\n    .iter(print)\n");
    parse_tree!(pipe_inline, "vr r = numbers.filter(p).map(m).sum()\n");
    parse_tree!(pipe_on_literal_array, "vr r = [1, 2, 3].map(x = x * 2)\n");
    parse_tree!(arithmetic_precedence, "vr r = 1 + 2 * 3 - 4 / 2\n");
    parse_tree!(arithmetic_parens, "vr r = (1 + 2) * (3 - 4)\n");
    parse_tree!(bitwise_operations, "vr r = (a & b) | (c ^ d)\n");
    parse_tree!(string_interpolation_simple, "vr s = \"Привет, ${name}!\"\n");
    parse_tree!(string_interpolation_expr, "vr s = \"Сумма: ${a + b}\"\n");
    parse_tree!(string_interpolation_prop, "vr s = \"Пользователь: ${u.name} (${u.age})\"\n");
    parse_tree!(method_call_chain, "vr s = text.trim().to_lower().replace(\"a\", \"b\")\n");
    parse_tree!(chained_index_access, "vr item = matrix[0][1]\n");
    parse_tree!(record_index_access, "vr item = data[\"key\"]\n");
    parse_tree!(complex_boolean_expr, "vr r = (a && b) || (!c && d)\n");
    parse_tree!(deep_field_mutation, "user.profile.settings.dark_mode = true\n");
    parse_tree!(array_mutation, "arr[0] = 99\n");
    parse_tree!(pipe_with_match_lambda, "vr r = list.map((res) = match res {\n    Ok(v) = v\n    Err(_) = 0\n})\n");
    parse_tree!(pipe_iter_console, "users.iter((u) = console.print(u.name))\n");
    parse_tree!(ternary_pipeline, "vr r = (if ready get_items() else []).filter(pred)\n");
}

// =============================================================================
// 10. ПОЛНЫЕ ПРОГРАММЫ И СЛОЖНЫЕ КЕЙСЫ (10 тестов)
// =============================================================================
mod full_programs {
    use super::*;

    parse_tree!(program_minimal, "out fn main() = 0\n");

    parse_tree!(program_example_rl, r#"
im console
im math = "std/math"
im { log, warn } = "std/logger"

vr name = "Алиса"
vr age = 25
vr city Str = "Киев"
vr ab = { a 1, b 2 }
vr { a, b } = ab
vr c = ab.a
vr mut counter = 0
counter = counter + 1

fn square(x Int) = x * x
fn add(a Int, b Int = 1) Int = a + b
fn identity<T>(val T) T = val
fn calc<T Number>(x T, b T) T = x * x

type User { name Str, age Int }
vr alice User = { name "Алиса", age 25 }

type Role =
    | Admin
    | User(id Int)
    | Guest

type Container(T) {
    value T,
    tag Str
}

type Response<TT Real = Number>(T TT = Int) =
    | Data(T)
    | Empty

fn wrap<T>(val T, tag Str) Container(T) = { value val, tag tag }

out fn main()
    vr users = [
        { name "Алиса", age 25 },
        { name "Боб", age 16 }
    ]
    users.iter((u) = console.print(u.name))
"#);

    parse_tree!(program_generic_math, r#"
type Point<T Number = Float>(x T, y T) {
    x T,
    y T
}

fn distance<T Number>(p1 Point(T), p2 Point(T)) T = {
    vr dx = p1.x - p2.x
    vr dy = p1.y - p2.y
    (dx * dx + dy * dy)
}
"#);

    parse_tree!(program_server_state, r#"
type ServerState =
    | Starting
    | Running(port Int)
    | Stopped(reason Str)

fn describe(st ServerState) Str = match st {
    Starting = "Старт..."
    Running(p) = "Порт: ${p}"
    Stopped(err) = "Ошибка: ${err}"
}
"#);

    parse_tree!(program_data_structures, r#"
type Node<T>(val T) {
    value T,
    next Node(T)?
}

fn create_list<T>(val T) Node(T) = {
    value val,
    next none
}
"#);

    parse_tree!(program_matrix_mult, r#"
fn multiply(a [[Int]], b [[Int]]) [[Int]] = {
    vr rows = a.len()
    vr cols = b[0].len()
    [[0]]
}
"#);

    parse_tree!(program_result_flow, r#"
fn validate_age(age Int) Result(Int, Str) =
    if age < 0 Err("Меньше 0")
    else if age > 150 Err("Слишком стар")
    else Ok(age)
"#);

    parse_tree!(program_builder_pattern, r#"
type Builder { steps [Str] }
fn add_step(b Builder, step Str) Builder = { steps b.steps }
"#);

    parse_tree!(program_closure_factory, r#"
fn make_adder(x Int) = (y) = x + y
out fn main() {
    vr add5 = make_adder(5)
    console.print(add5(10))
}
"#);

    parse_tree!(program_destructuring_heavy, r#"
type Full { a Int, b { c Str, d [Int] } }
fn test(f Full) = {
    vr { a, b { c, d } } = f
    c
}
"#);
}

// =============================================================================
// 11. НЕГАТИВНЫЕ ТЕСТЫ (СИНТАКСИЧЕСКИЕ ОШИБКИ) (20 тестов)
// =============================================================================
mod negative_tests {
    use super::*;

    parse_fail!(err_missing_vr_name, "vr = 10\n");
    parse_fail!(err_unclosed_string, "vr s = \"незакрытая строка\n");
    parse_fail!(err_unclosed_brace_record, "vr x = { a 1, b 2\n");
    parse_fail!(err_unclosed_bracket_array, "vr x = [1, 2, 3\n");
    parse_fail!(err_fn_no_body, "fn test(x Int)\n");
    parse_fail!(err_fn_double_equals, "fn test() == 1\n");
    parse_fail!(err_im_no_target, "im\n");
    parse_fail!(err_im_unclosed_brace, "im { a, b = \"path\"\n");
    parse_fail!(err_mut_without_vr, "mut x = 10\n");
    parse_fail!(err_type_no_name, "type { a Int }\n");
    parse_fail!(err_unclosed_type_generics, "type Box<T { val T }\n");
    parse_fail!(err_invalid_generic_bounds, "fn calc<T === Number>() = 0\n");
    parse_fail!(err_match_no_arms, "vr x = match target {}\n");
    parse_fail!(err_if_no_condition, "vr x = if 10 else 20\n");
    parse_fail!(err_missing_else_in_inline_if, "vr x = if a > b 1\n");
    parse_fail!(err_while_no_body, "while true\n");
    parse_fail!(err_for_missing_in, "for x items print(x)\n");
    parse_fail!(err_invalid_char, "vr x = @@@\n");
    parse_fail!(err_unclosed_comment, "## незакрытый многострочный комментарий\n");
    parse_fail!(err_double_op, "vr x = 10 ++ 20\n");
}