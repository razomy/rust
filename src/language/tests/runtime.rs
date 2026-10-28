// tests/runtime
// =============================================================================
// RAZOMY V4.7 — RUNTIME EXECUTION, DIALECTS & CONCURRENCY TEST SUITE
// Запуск: cargo test --test v47_runtime_suite -- --nocapture
// =============================================================================

use razomy_language::runtime::{run_program, Value};

#[test]
fn test_runtime_lineage_and_index_access() {
    let code = r#"
        HasParentName { ParentName String }
        Jonn String
        JonnName HasParentName { ParentName Jonn }
        JonnDoy Jonn

        JonnDoyName2 JonnName {
            ParentName JonnDoy
        } & { ParentName "JonnDoyBoy" }

        JonnDoyName3 JonnDoyName2 & { D "d" }

        Main [
            (
                JonnDoyName2.ParentName,
                JonnDoyName2.JonnName.ParentName,
                JonnDoyName3.0,
                JonnDoyName3.-1
            )
        ]
    "#;

    let res = run_program(code).expect("Lineage program failed");
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("JonnDoyBoy".into()),
            Value::Symbol("Jonn".into()),
            Value::String("JonnDoyBoy".into()),
            Value::String("d".into()),
        ])
    );
}

#[test]
fn test_runtime_reactive_forms_excel_dataflow() {
    let code = r#"
        Invoice {
            Price    $100.00
            Quantity 3
            TaxRate  20%

            Subtotal [ Price * Quantity ]
            Tax      [ Subtotal * TaxRate ]
            Total    [ Subtotal + Tax ]
        }

        Order1 Invoice & { Quantity 5 }

        Main [
            (Invoice.Total, Order1.Subtotal, Order1.Tax, Order1.Total)
        ]
    "#;

    let res = run_program(code).expect("Reactive Invoice failed");
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::Money("$360.00".into()),
            Value::Money("$500.00".into()),
            Value::Money("$100.00".into()),
            Value::Money("$600.00".into()),
        ])
    );
}

#[test]
fn test_runtime_operator_overloading_and_math() {
    let code = r#"
        Vec2 {
            X Float
            Y Float

            +(Other Vec2)  [ Vec2 { X (X + Other.X), Y (Y + Other.Y) } ]
            *(Scale Float) [ Vec2 { X (X * Scale),   Y (Y * Scale) } ]
        }

        Main [
            V1 Vec2 { X 2.0, Y 3.0 }
            V2 Vec2 { X 4.0, Y 1.0 }
            V3 ((V1 + V2) * 2.0)
            (V3.X, V3.Y)
        ]
    "#;

    let res = run_program(code).expect("Vec2 operator overloading failed");
    assert_eq!(res, Value::Tuple(vec![Value::Float(12.0), Value::Float(8.0)]));
}

#[test]
fn test_runtime_type_subtraction_and_map_manipulation() {
    let code = r#"
        User  { Id String, Name String }
        Cloud { Id String, Name String, Region String }

        UserAndCloud      User & Cloud
        UserWithCloudName UserAndCloud / User.Name

        ConfigMap {
            "host" "localhost"
            "port" "8080"
        }
        UpdatedConfig ConfigMap & { "port" "9090", "tls" "true" } / "host"

        Main [
            Inst (User { Id "u1", Name "Alice" } & Cloud { Id "c1", Name "AWS", Region "eu" })
            Clean (Inst / User.Name)
            PortOpt (UpdatedConfig . Map.Get("port"))
            HostOpt (UpdatedConfig . Map.Get("host"))
            (Clean.Name, PortOpt.Value, HostOpt)
        ]
    "#;

    let res = run_program(code).expect("Type subtraction and Map failed");
    if let Value::Tuple(items) = res {
        assert_eq!(items[0], Value::String("AWS".into()));
        assert_eq!(items[1], Value::String("9090".into()));
    } else {
        panic!("Expected Tuple");
    }
}

#[test]
fn test_runtime_defaults_partial_application_and_named_overrides() {
    let code = r#"
        Connect(Host "localhost", Port 8080, Tls false) [
            "{Host}:{Port}:{Tls}"
        ]

        DefaultConn   Connect()
        SecureConnect (Connect & { Port 9090, Tls true })
        ActiveConn    SecureConnect()

        AddTwo(A Int, B Int) [ A + B ]
        AddTen AddTwo(10)

        Main [
            (DefaultConn, ActiveConn, AddTen(25))
        ]
    "#;

    let res = run_program(code).expect("Defaults and Partial Application failed");
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
fn test_runtime_pipelines_refinements_and_law3_outcome_bypass() {
    let code = r#"
        Order      { Id String, Amount Int }
        Reserved   { ReservedAt Int }
        Paid       { TransactionId String }
        OutOfStock { Reason String }
        NoMoney    { Reason String }

        ReserveStock(O Order) [
            O.Amount . {
                [ _ > 0 ] [ O & Reserved { ReservedAt 100 } ]
                _         [ OutOfStock { Reason "Empty" } ]
            }
        ]

        ChargePayment(O Order & Reserved) [
            O.Amount . {
                [ _ < 50000 ] [ O & Paid { TransactionId "tx_777" } ]
                _             [ NoMoney { Reason "Too expensive" } ]
            }
        ]

        Execute(O Order) [
            O . ReserveStock . ChargePayment
        ]

        Main [
            Good (Order { Id "1", Amount 500 } . Execute)
            Empty (Order { Id "2", Amount 0 } . Execute)
            (Good.TransactionId, Empty.Reason)
        ]
    "#;

    let res = run_program(code).expect("Law 3 pipeline failed");
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("tx_777".into()),
            Value::String("Empty".into()),
        ])
    );
}

#[test]
fn test_runtime_collections_early_exit_done_and_catamorphism() {
    let code = r#"
        Done(T) { Value T }

        SumUntilLimit(Numbers, Max Int) [
            Numbers . Fold(0, (Acc, N) [
                (Acc + N) . {
                    [ _ > Max ] [ Done { Value _ } ]
                    _           [ _ ]
                }
            ])
        ]

        Num(T)    { Value T }
        Add(L, R) { Left L, Right R }
        Mul(L, R) { Left L, Right R }

        Cata(Expression, Algebra) [
            Expression . {
                Num [ Algebra.OnNum(Value) ]
                Add [ Algebra.OnAdd(Left . Cata(Algebra), Right . Cata(Algebra)) ]
                Mul [ Algebra.OnMul(Left . Cata(Algebra), Right . Cata(Algebra)) ]
            }
        ]

        CalcAlgebra {
            OnNum(V)    [ V ]
            OnAdd(A, B) [ A + B ]
            OnMul(A, B) [ A * B ]
        }

        Main [
            EarlySum ((1..10) . SumUntilLimit(12))
            Ast (Mul { Left (Add { Left (Num { Value 2 }), Right (Num { Value 3 }) }), Right (Num { Value 4 }) })
            EvalRes (Ast . Cata(CalcAlgebra))
            (EarlySum, EvalRes)
        ]
    "#;

    let res = run_program(code).expect("Done & Catamorphism failed");
    assert_eq!(res, Value::Tuple(vec![Value::Int(15), Value::Int(20)]));
}

#[test]
fn test_runtime_parallel_pipe_and_atoms() {
    let code = r#"
        CounterState { Count Int }
        Increment    { By Int }

        UpdateCounter(State CounterState, Msg Increment) [
            CounterState { Count (State.Count + Msg.By) }
        ]

        GlobalCounter Atom(CounterState { Count 10 })

        LoadParallel(Id String) [
            Id .| {
                A [ "{_}_profile" ]
                B [ "{_}_recs" ]
            }
        ]

        Main [
            GlobalCounter.Swap((S) [ UpdateCounter(S, Increment { By 5 }) ])
            Dash ("u42" . LoadParallel)
            (GlobalCounter.Read().Count, Dash.A, Dash.B)
        ]
    "#;

    let res = run_program(code).expect("Parallel pipe & Atom failed");
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::Int(15),
            Value::String("u42_profile".into()),
            Value::String("u42_recs".into()),
        ])
    );
}

#[test]
fn test_runtime_deep_router_and_metaprogramming() {
    let code = r#"
        HttpRequest  { Method Symbol, Path Tuple }
        HttpResponse { Status Int, Body String }

        Route(Req HttpRequest) [
            Req . {
                { Method 'GET', Path ("users", Id) } [
                    HttpResponse { Status 200, Body "User:{Id}" }
                ]
                _ [ HttpResponse { Status 404, Body "Not Found" } ]
            }
        ]

        BuildMultiplier(Factor Int) [
            Compose([ _ * (Factor * 10) ])
        ]

        DeriveJson(TargetForm) [
            TargetForm & {
                ToJson() [
                    Inner (
                        Self . Meta.FieldsOf . Map((Field) [
                            "\"{Field.Name}\": {Field.Value . Json.FormatValue}"
                        ])
                        . Join(", ")
                    )
                    "\{ {Inner} \}"
                ]
            }
        ]

        Product { Id Int, Title String } . DeriveJson

        Main [
            Resp (HttpRequest { Method 'GET', Path ("users", "77") } . Route)
            Mul50 BuildMultiplier(5)
            CalcRes (3 . Mul50)
            Item (Product { Id 1, Title "Book" })
            (Resp.Body, CalcRes, Item.ToJson())
        ]
    "#;

    let res = run_program(code).expect("Router & Metaprogramming failed");
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("User:77".into()),
            Value::Int(150),
            Value::String("{ \"Id\": 1, \"Title\": \"Book\" }".into()),
        ])
    );
}

#[test]
fn test_runtime_csp_channels_and_stream_composition() {
    let code = r#"
        Step1 [ _ + 10 ]
        Step2 [ _ * 2 ]
        Combined (Step1 & Step2)
        CleanAndUpper (Trim & ToUpper)

        Main [
            Ch Channel()
            Ch.Send("  razomy v4.7  ")
            Received Ch.Recv()
            Processed (Received.Value . CleanAndUpper)
            MathRes (5 . Combined)
            (Processed, MathRes)
        ]
    "#;

    let res = run_program(code).expect("CSP Channels & Stream Composition failed");
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("RAZOMY V4.7".into()),
            Value::Int(30),
        ])
    );
}

#[test]
fn test_runtime_parse_dialect_and_meta_steps() {
    let code = r#"
        LogEntry { Date String, Level String, Msg String }

        ParseLogLine(Line String) [
            Line . Parse([
                Copy('Date', To(" "))
            ]) . {
                LogEntry [ "{Level}:{Msg}" ]
            }
        ]

        SampleStream [
            _ + 1
            _ * 2
            _ - 3
        ]

        Main [
            Parsed ("2026-03-30 [WARN] Disk_almost_full" . ParseLogLine)
            StepsCount (SampleStream . Meta.StepsOf . Length)
            (Parsed, StepsCount)
        ]
    "#;

    let res = run_program(code).expect("Parse Dialect & Meta.StepsOf failed");
    assert_eq!(
        res,
        Value::Tuple(vec![
            Value::String("WARN:Disk_almost_full".into()),
            Value::Int(3),
        ])
    );
}