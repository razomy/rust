// tests/runtime.rs
// =============================================================================
// RAZOMY V5.7 — RUNTIME EXECUTION, DIALECTS & CONCURRENCY TEST SUITE
// Каждый тест логирует компактный дамп структуры результата (`compact_dump`)
// и детальную причину ошибки в случае падения.
// =============================================================================

use razomy_language::analyzer::analyze_source;
use razomy_language::runtime::{run_program, Value};

/// Запускает программу Razomy V5.7, печатая компактный лог структуры и диагностики
/// (видно при `cargo test -- --nocapture` и при любом падении теста).
fn run_and_log(test_label: &str, code: &str) -> Value {
    let report = analyze_source(code);
    let errors: Vec<String> = report
        .diagnostics
        .iter()
        .map(|d| format!("[{:?}:{}] {}", d.severity, d.code, d.message))
        .collect();

    match run_program(code) {
        Ok(val) => {
            println!(
                "[TEST LOG | {test_label}] OK -> {}",
                val.compact_dump()
            );
            val
        }
        Err(err) => {
            eprintln!(
                "\n[TEST FAIL | {test_label}]\n  Runtime Error: {err}\n  HIR Diagnostics: {errors:?}\n  Forms in HIR: {:?}\n  Functions in HIR: {:?}\n",
                report.hir.form_order, report.hir.function_order
            );
            panic!("{test_label} failed: {err}");
        }
    }
}

#[test]
fn test_runtime_lineage_index_access_and_perceus_in_place_mutation() {
    let code = r#"
        BrandName String

        Vehicle {
            brand     BrandName
            max_speed Int 180
            tag       "standard"
        }

        Car Vehicle {
            brand     "GenericMotor"
            max_speed 200
        }

        SportsCar Car {
            brand     "Porsche"
            max_speed 300
        }

        MyCar SportsCar {
            brand "CustomPorsche"
        }

        MyGarageCar MyCar & { garage_slot 1 }

        main() [
            my_car (MyGarageCar {})
            my_car.brand "Tesla"
            my_car.0     "Rivian"
            my_car.-1    42

            (
                MyCar.brand,
                MyCar.SportsCar.brand,
                MyCar.SportsCar.Car.brand,
                MyGarageCar.0,
                MyGarageCar.-1,
                my_car.brand,
                my_car.garage_slot
            )
        ]
    "#;

    let res = run_and_log("Lineage & Perceus mutation", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("CustomPorsche".into()),
            Value::String("Porsche".into()),
            Value::String("GenericMotor".into()),
            Value::String("CustomPorsche".into()),
            Value::Int(1),
            Value::String("Rivian".into()),
            Value::Int(42),
        ])
    );
}

#[test]
fn test_runtime_exact_money_and_reactive_forms_excel_dataflow() {
    let code = r#"
        Money(raw String "0.00") {
            scaled (raw . Decimal.parse_scale4)
            from_scaled(val Int) [ Money { scaled val } ]
            +  (other Money)  [ Money.from_scaled(self.scaled + other.scaled) ]
            -  (other Money)  [ Money.from_scaled(self.scaled - other.scaled) ]
            *  (factor Float) [ Money.from_scaled((self.scaled.to_float * factor).round_to_int) ]
            == (other Money)  [ self.scaled == other.scaled ]
        }

        Invoice {
            price    Money"100.00"
            quantity 3.0
            tax_rate 0.20

            subtotal [ self.price * self.quantity ]
            tax      [ self.subtotal * self.tax_rate ]
            total    [ self.subtotal + self.tax ]
        }

        order_1 Invoice & { quantity 5.0 }

        main() [
            (
                Invoice.total.scaled,
                order_1.subtotal.scaled,
                order_1.tax.scaled,
                order_1.total.scaled,
                order_1.total == Money"600.00"
            )
        ]
    "#;

    let res = run_and_log("Reactive Invoice with exact Money", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::Int(3600000),
            Value::Int(5000000),
            Value::Int(1000000),
            Value::Int(6000000),
            Value::Bool(true),
        ])
    );
}

#[test]
fn test_runtime_law1_scoping_and_depth_indexed_subjects() {
    let code = r#"
        WithName { name String }

        read_via_type(WithName)          [ with_name.name ]
        read_unpacked({ name } WithName) [ name ]
        extract_name_demo(w WithName)    [ w . [ _1.name ] ]
        sum_pair_pipe(pair (Int, Int))   [ pair . [ _1.0 + _1.1 ] ]

        main() [
            item WithName { name "Razomy" }
            (
                item . read_via_type,
                item . read_unpacked,
                item . extract_name_demo,
                (15, 27) . sum_pair_pipe
            )
        ]
    "#;

    let res = run_and_log("Law 1 & depth-indexed subjects", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("Razomy".into()),
            Value::String("Razomy".into()),
            Value::String("Razomy".into()),
            Value::Int(42),
        ])
    );
}

#[test]
fn test_runtime_operator_overloading_and_pattern_inclusion_eq() {
    let code = r#"
        Vec2 {
            x Float
            y Float

            + (other Vec2)  [ Vec2 { x (self.x + other.x), y (self.y + other.y) } ]
            * (scale Float) [ Vec2 { x (self.x * scale),   y (self.y * scale) } ]
        }

        main() [
            v1 Vec2 { x 2.0, y 3.0 }
            v2 Vec2 { x 4.0, y 1.0 }
            v3 ((v1 + v2) * 2.0)
            in_range (42 == (1..100))
            is_mgr   ('manager' == ('admin' | 'manager'))
            (v3.x, v3.y, in_range, is_mgr)
        ]
    "#;

    let res = run_and_log("Vec2 operator overloading", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::Float(12.0),
            Value::Float(8.0),
            Value::Bool(true),
            Value::Bool(true),
        ])
    );
}

#[test]
fn test_runtime_type_subtraction_and_map_manipulation() {
    let code = r#"
        User  { id String, name String }
        Cloud { id String, name String, region String }

        UserAndCloud      User & Cloud
        UserWithCloudName UserAndCloud / User.name

        config_map {
            "host" "localhost"
            "port" "8080"
        }
        updated_config config_map & { "port" "9090", "tls" "true" } / "host"

        main() [
            inst (User { id "u1", name "Alice" } & Cloud { id "c1", name "AWS", region "eu" })
            clean (inst / User.name)
            port_opt (updated_config . Map.get("port"))
            host_opt (updated_config . Map.get("host"))
            (clean.name, port_opt.value, host_opt)
        ]
    "#;

    let res = run_and_log("Type subtraction and Map", code);
    if let Value::Tuple(items) = res {
        assert_eq!(items[0], Value::String("AWS".into()));
        assert_eq!(items[1], Value::String("9090".into()));
    } else {
        panic!("Expected Tuple");
    }
}

#[test]
fn test_runtime_defaults_partial_application_and_di_overrides() {
    let code = r#"
        connect(host String "localhost", port Int 8080, tls Bool false) [
            "{host}:{port}:{tls}"
        ]

        default_conn   connect()
        secure_connect (connect & { port 9090, tls true })
        active_conn    secure_connect()

        add_two(a Int, b Int) [ a + b ]
        add_ten add_two(10)

        main() [
            (default_conn, active_conn, add_ten(25))
        ]
    "#;

    let res = run_and_log("Defaults and Partial Application", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("localhost:8080:false".into()),
            Value::String("localhost:9090:true".into()),
            Value::Int(35),
        ])
    );
}

#[test]
fn test_runtime_pipelines_refinements_and_law3_bypass_outcome() {
    let code = r#"
        Order      { id String, amount Int }
        Reserved   { reserved_at Int }
        Paid       { transaction_id String }
        OutOfStock { reason String } & Bypass
        NoMoney    { reason String } & Bypass

        reserve_stock(o Order) [
            o.amount . {
                [ _ > 0 ] [ o & Reserved { reserved_at 100 } ]
                _         [ OutOfStock { reason "Empty" } ]
            }
        ]

        charge_payment(o Order & Reserved) [
            o.amount . {
                [ _ < 50000 ] [ o & Paid { transaction_id "tx_777" } ]
                _             [ NoMoney { reason "Too expensive" } ]
            }
        ]

        execute(o Order) [
            o . reserve_stock . charge_payment
        ]

        main() [
            good  (Order { id "1", amount 500 } . execute)
            empty (Order { id "2", amount 0 } . execute)
            (good.transaction_id, empty.reason)
        ]
    "#;

    let res = run_and_log("Law 3 pipeline", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("tx_777".into()),
            Value::String("Empty".into()),
        ])
    );
}

#[test]
fn test_runtime_collections_early_exit_break_zip_enumerate_and_catamorphism() {
    let code = r##"
        Break(T) { value T, label Symbol 'default' }

        sum_until_limit(numbers, max Int) [
            numbers . fold(0, (acc Int, n Int) [
                (acc + n) . {
                    [ _ > max ] [ Break { value _ } ]
                    _           [ _ ]
                }
            ])
        ]

        Num(T)    { value T }
        Add(L, R) { left L, right R }
        Mul(L, R) { left L, right R }

        cata(expression, algebra) [
            expression . {
                Num [ algebra.on_num(num.value) ]
                Add [ algebra.on_add(add.left . cata(algebra), add.right . cata(algebra)) ]
                Mul [ algebra.on_mul(mul.left . cata(algebra), mul.right . cata(algebra)) ]
            }
        ]

        CalcAlgebra {
            on_num(v Int)        [ v ]
            on_add(a Int, b Int) [ a + b ]
            on_mul(a Int, b Int) [ a * b ]
        }

        main() [
            early_sum ((1..10) . sum_until_limit(12))
            ast (Mul { left (Add { left (Num { value 2 }), right (Num { value 3 }) }), right (Num { value 4 }) })
            eval_res (ast . cata(CalcAlgebra))
            zipped (("a", "b") . zip((10, 20)) . enumerate)
            (early_sum, eval_res, zipped.length)
        ]
    "##;

    let res = run_and_log("Break, zip/enumerate & Catamorphism", code);
    assert_eq!(
        res,
        Value::Tuple(vec![Value::Int(15), Value::Int(20), Value::Int(2)])
    );
}

#[test]
fn test_runtime_parallel_pipe_atoms_signals_and_fsm_tuple_type_routing() {
    let code = r#"
        CounterState { count Int }
        Increment    { by Int }

        update_counter(state CounterState, msg Increment) [
            CounterState { count (state.count + msg.by) }
        ]

        global_counter Atom(CounterState { count 10 })

        Disconnected { reason String }
        Connecting   { host String, attempt Int }
        ConnectCmd   { host String }

        step_connection(state, event) [
            (state, event) . {
                (Disconnected, ConnectCmd) [ Connecting { host connect_cmd.host, attempt 1 } ]
                _                          [ state ]
            }
        ]

        load_parallel(id String) [
            id .| {
                a [ "{_}_profile" ]
                b [ "{_}_recs" ]
            }
        ]

        main() [
            global_counter.swap((s CounterState) [ update_counter(s, Increment { by 5 }) ])
            dash ("u42" . load_parallel)
            next_conn step_connection(Disconnected { reason "init" }, ConnectCmd { host "razomy.org" })
            (global_counter.read().count, dash.a, dash.b, next_conn.host, next_conn.attempt)
        ]
    "#;

    let res = run_and_log("Parallel pipe, Atom & FSM", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::Int(15),
            Value::String("u42_profile".into()),
            Value::String("u42_recs".into()),
            Value::String("razomy.org".into()),
            Value::Int(1),
        ])
    );
}

#[test]
fn test_runtime_deep_router_and_compile_time_metaprogramming() {
    let code = r#"
        HttpRequest  { method Symbol, path Tuple(String) }
        HttpResponse { status Int, body String }

        route(req HttpRequest) [
            req . {
                { method 'GET', path ("users", id) } [
                    HttpResponse { status 200, body "User:{id}" }
                ]
                _ [ HttpResponse { status 404, body "Not Found" } ]
            }
        ]

        build_multiplier(factor Int) [
            Meta.compose([ _ * (factor * 10) ])
        ] & Compile

        derive_json(TargetForm) [
            TargetForm & {
                to_json() [
                    inner (
                        self . Meta.fields_of . map((field) [
                            "\"{field.name}\": {field.value . Json.format_value}"
                        ])
                        . join(", ")
                    )
                    "\{ {inner} \}"
                ]
            }
        ] & Compile

        Product (Compile & [
            {
                id    Int
                title String
            } . derive_json
        ])

        main() [
            resp (HttpRequest { method 'GET', path ("users", "77") } . route)
            mul50 build_multiplier(5)
            calc_res (3 . mul50)
            item (Product { id 1, title "Book" })
            (resp.body, calc_res, item.to_json())
        ]
    "#;

    let res = run_and_log("Router & Compile Metaprogramming", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("User:77".into()),
            Value::Int(150),
            Value::String("{ \"id\": 1, \"title\": \"Book\" }".into()),
        ])
    );
}

#[test]
fn test_runtime_csp_channels_and_stream_composition() {
    let code = r#"
        step_1 [ _ + 10 ]
        step_2 [ _ * 2 ]
        combined_pipeline step_1 & step_2

        trim_str  [ _ . trim ]
        upper_str [ _ . to_upper ]
        clean_str trim_str & upper_str

        main() [
            ch Channel()
            ch.send("  razomy v5.7  ")
            received ch.recv()
            processed (received.value . clean_str)
            math_res (5 . combined_pipeline)
            (processed, math_res)
        ]
    "#;

    let res = run_and_log("CSP Channels & Stream Composition", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("RAZOMY V5.7".into()),
            Value::Int(30),
        ])
    );
}

#[test]
fn test_runtime_parse_dialect_and_meta_steps_of() {
    let code = r#"
        LogEntry { date String, level String, msg String }

        parse_log_line(line String) [
            line . Parse.run(
                Parse.copy('date', Parse.to(" "))
            ) . {
                LogEntry [ "{log_entry.level}:{log_entry.msg}" ]
            }
        ]

        sample_stream [
            _ + 1
            _ * 2
            _ - 3
        ]

        main() [
            parsed ("2026-03-30 [WARN] Disk_almost_full" . parse_log_line)
            steps_count (sample_stream . Meta.steps_of . length)
            (parsed, steps_count)
        ]
    "#;

    let res = run_and_log("Parse Dialect & Meta.steps_of", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("WARN:Disk_almost_full".into()),
            Value::Int(3),
        ])
    );
}

// =============================================================================
// ДОПОЛНИТЕЛЬНЫЕ ТЕСТЫ ПО СПЕЦИФИКАЦИИ V5.7 (РАЗДЕЛЫ 3, 4, 5, 8, 12, 14, 17, 23, 25)
// =============================================================================

#[test]
fn test_runtime_spec_sec3_deep_lens_rebinding_and_tuple_auto_lifting() {
    let code = r#"
        Order      { id String, amount Int }
        Department { title String, budget Int }
        Company    { name String, departments Tuple(Department) }

        main() [
            raw_order Order { id "o1", amount 100 }
            counter 0
            counter (counter + 1)

            // 1. Мутация по имени поля:
            raw_order.amount 14

            // 2. Авто-лифтинг кортежа `(Department { ... })` и глубокая мутация по пути `corp.departments.0.budget`:
            corp Company { name "Acme", departments (Department { title "R&D", budget 1000 }) }
            corp.departments.0.budget 2500

            // 3. Позиционная мутация `.1` (второе поле `amount`):
            raw_order.1 20

            // 4. Слияние `&` и Enrich Pipe `.&`:
            next_order (raw_order & { amount 25 })
            next_order (next_order .& { amount 50 })

            (counter, raw_order.amount, corp.departments.0.budget, next_order.amount)
        ]
    "#;

    let res = run_and_log("Sec 3 Deep Lens & Tuple Auto-Lifting", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::Int(1),
            Value::Int(20),
            Value::Int(2500),
            Value::Int(50),
        ])
    );
}

#[test]
fn test_runtime_spec_sec4_and_sec5_traits_fbounded_and_math_ops() {
    let code = r#"
        User { id String, name String, age Int }

        Printable {
            format() String
        }

        PrintableUser User & Printable {
            format() [ "User({self.name}, id={self.id})" ]
        }

        Comparable(Other) {
            < (other Other) Bool
        }

        SortableUser User & Comparable(User) {
            < (other User) [ self.age < other.age ]
        }

        MathOps(a Int, b Int) {
            add      (a + b)
            sub      (a - b)
            mul      (a * b)
            div      (a / b)
            mod      (a % b)
            pow      (a ** b)
            lazy_div [ self.a / self.b ]
            shift_l  (a << 2)
            shift_r  (a >> 1)
        }

        main() [
            pu (PrintableUser { id "u7", name "Alice", age 20 })
            su1 (SortableUser { id "u1", name "Bob", age 19 })
            su2 (SortableUser { id "u2", name "Eve", age 30 })
            m (MathOps(10, 2))
            (pu.format(), (su1 < su2), m.add, m.pow, m.lazy_div, m.shift_l, m.shift_r)
        ]
    "#;

    let res = run_and_log("Sec 4 & 5 Traits, F-Bounded & MathOps", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("User(Alice, id=u7)".into()),
            Value::Bool(true),
            Value::Int(12),
            Value::Int(100),
            Value::Int(5),
            Value::Int(40),
            Value::Int(5),
        ])
    );
}

#[test]
fn test_runtime_spec_sec8_do_notation_and_finalize_bill_enrichment() {
    let code = r#"
        User     { id String, name String, age Int }
        Cart     { cart_id String, amount Int }
        NotFound { entity String, id String } & Bypass

        Db {
            find_user(id String) User | NotFound [
                id . {
                    "missing" [ NotFound { entity "User", id id } ]
                    _         [ User { id id, name "User_{id}", age 25 } ]
                }
            ]
        }

        finalize_bill(ctx User & Cart) [
            ctx & {
                discount [
                    self.amount . {
                        [ _ >= 2000 ] [ 300 ]
                        [ _ >= 1000 ] [ 100 ]
                        _             [ 0 ]
                    }
                ]
                final_price [ self.amount - self.discount ]
            }
        ]

        load_user_and_friend(uid1 String, uid2 String) [
            { first_id uid1, second_id uid2 }
            .& [ _.first_id . Db.find_user . [ { u1 _ } ] ]
            .& [ _.second_id . Db.find_user . [ { u2 _ } ] ]
            .  [ "{_.u1.name} is friends with {_.u2.name}" ]
        ]

        main() [
            bill ((User { id "u1", name "Alice", age 25 } & Cart { cart_id "c1", amount 2500 }) . finalize_bill)
            ok_pair load_user_and_friend("1", "2")
            err_pair load_user_and_friend("1", "missing")
            (bill.discount, bill.final_price, ok_pair, err_pair.id)
        ]
    "#;

    let res = run_and_log("Sec 8 Multi-Query do-notation & finalize_bill", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::Int(300),
            Value::Int(2200),
            Value::String("User_1 is friends with User_2".into()),
            Value::String("missing".into()),
        ])
    );
}

#[test]
fn test_runtime_spec_sec10_numbered_catalog_and_fizzbuzz() {
    let code = r##"
        Item { name String, price Int, in_stock Bool }

        format_numbered_catalog(items Tuple(Item), discounts Tuple(Int)) [
            items
            . zip(discounts)
            . enumerate
            . map((idx Int, (item Item, disc Int)) [
                "#{idx}: {item.name} -> {item.price - disc} USD"
            ])
        ]

        fizz_buzz(limit Int) [
            (1..limit) . map((n Int) [
                (n % 3, n % 5) . {
                    (0, 0) [ "FizzBuzz" ]
                    (0, _) [ "Fizz" ]
                    (_, 0) [ "Buzz" ]
                    _      [ "{n}" ]
                }
            ])
        ]

        main() [
            items (
                Item { name "Laptop", price 1200, in_stock true },
                Item { name "Screen", price 400,  in_stock true }
            )
            cat format_numbered_catalog(items, (200, 50))
            fb  fizz_buzz(5)
            (cat.0, cat.1, fb.2, fb.4)
        ]
    "##;

    let res = run_and_log("Sec 10 Catalog zip/enumerate & FizzBuzz", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("#0: Laptop -> 1000 USD".into()),
            Value::String("#1: Screen -> 350 USD".into()),
            Value::String("Fizz".into()),
            Value::String("Buzz".into()),
        ])
    );
}

#[test]
fn test_runtime_spec_sec12_and_sec14_signals_option_monad_and_list_map() {
    let code = r#"
        Some(T)   { value T }
        None      {} & Bypass
        Option(T) Some(T) | None

        OptionMonad {
            pure(A, a A) [ Some { value a } ]
            flat_map(A, B, ma Option(A), f (A) [Option(B)]) [
                ma . {
                    Some [ some.value . f ]
                    None [ None ]
                }
            ]
        }

        Cons(T) { head T, tail Any }

        list_map(A, B, l, action) [
            l . {
                None [ None ]
                Cons [
                    Cons {
                        head (cons.head . action)
                        tail (cons.tail . list_map(action))
                    }
                ]
            }
        ]

        count_sig   Signal(2)
        doubled_sig [ count_sig.get() * 2 ]

        main() [
            count_sig.set(10)
            sig_val (count_sig.get() . doubled_sig)

            opt1 OptionMonad.pure(21)
            opt2 OptionMonad.flat_map(opt1, (x Int) [ OptionMonad.pure(x * 2) ])

            lst Cons { head 3, tail (Cons { head 7, tail None }) }
            mapped_lst (lst . list_map([ _ * 10 ]))

            (sig_val, opt2.value, mapped_lst.head, mapped_lst.tail.head)
        ]
    "#;

    let res = run_and_log("Sec 12 & 14 Signals, OptionMonad & list_map", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::Int(20),
            Value::Int(42),
            Value::Int(30),
            Value::Int(70),
        ])
    );
}

#[test]
fn test_runtime_spec_sec17_sec23_sec25_scoped_fs_ai_agent_and_transfer_safe() {
    let code = r#"
        FileHandle { fd Int }
        IoError    { msg String } & Bypass

        Fs {
            open(path String) [
                path . {
                    "bad.json" [ IoError { msg "ENOENT" } ]
                    _          [ FileHandle { fd 7 } ]
                }
            ]
            close(FileHandle) [ file_handle.fd ]
            with_file(path String, action) [
                path . Fs.open . {
                    FileHandle [
                        file_handle . action
                        .> [ file_handle . Fs.close ]
                    ]
                }
            ]
        }

        CompileError { code Int, log String } & Bypass
        AgentState   { err CompileError, retries Int, patches Int }

        agent_loop(state AgentState) [
            state.retries . {
                [ _ == 0 ] [ "halted_after_{state.patches}_patches" ]
                _ [
                    next_state (state & {
                        retries (state.retries - 1)
                        patches (state.patches + 1)
                    })
                    agent_loop(next_state)
                ]
            }
        ]

        Account { iban String, balance Int }

        transfer_safe(source Account, target Account, amount Int) [
            source.balance (source.balance - amount)
            target.balance (target.balance + amount)
            (source.balance, target.balance)
        ]

        main() [
            ok_file  Fs.with_file("config.json", (f FileHandle) [ "fd:{f.fd}" ])
            err_file Fs.with_file("bad.json",    (f FileHandle) [ "fd:{f.fd}" ])

            agent_res agent_loop(AgentState {
                err     (CompileError { code 500, log "E01" })
                retries 3
                patches 0
            })

            balances transfer_safe(
                Account { iban "A1", balance 500 },
                Account { iban "A2", balance 200 },
                150
            )

            (ok_file, err_file.msg, agent_res, balances.0, balances.1)
        ]
    "#;

    let res = run_and_log("Sec 17, 23 & 25 Scoped Fs, AI Agent TCO & transfer_safe", code);
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("fd:7".into()),
            Value::String("ENOENT".into()),
            Value::String("halted_after_3_patches".into()),
            Value::Int(350),
            Value::Int(350),
        ])
    );
}