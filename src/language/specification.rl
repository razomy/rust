name "Razomy language"
version "6.4"
extension ".rl"
tags (
  "HOMOICONIC"
  "AXIOMATIC"
  "FRACTAL"
  "SYMMETRIC"
  "ALGEBRAIC-LINEAGE"
  "CONCATENATIVE"
  "ONTOLOGICAL"
  "STAGE-POLYMORPHIC"
)
description r3"
1. FRACTAL & SYMMETRIC ARCHITECTURE OF THREE BRACKETS:
   Razomy is scale-invariant (Fractal): Project Directory == File == Module == Class == Router == Function Slots == AST Node == `{}`.
   Every construct has a symmetric dual: Form `{}` <-> Tuple `()`, Merge `&` <-> Subtract `/`, Positive `.?` <-> Negative `.!`,
   Type (`PascalCase`) <-> Instance (`snake_case`), Anonymous Hole `_` <-> Named Type Hole `_T`, Compile-Time (`Compile &`) <-> Runtime.

{} — Form   (Key-Value: Data structures, Classes, Argument slots, Trait impls, Routers, Modules, Maps).
            • Prefix spread `..Ancestor` inside `{}` (`ElectricCar { ..Vehicle, ..HasBattery }` or `Monad(_M(_)) { ..Functor(_M) }`)
              embeds ancestors into `{}` with full Lineage history (equivalent to `Vehicle & HasBattery & { ... }`).
            • A bare `PascalCase { ... }` inside `{}` (`SubToken { ... }`) declares a nested submodule or field without polluting outer keys.
            • Parens on a key position `{ (K) V }` define a parameterized/computed Map key or compound AST pattern key.
            • All Forms `{}` and Tuples `()` have structural equality `==` and hashing by default.
            • Valid vs Error (`E08_AMBIGUOUS_ROUTER_KEY`): Router keys in `x . { ... }` must be Types, Literals, `'sym'`, `_`, `(expr)`, or `.op val`.
              `x . { .== target [ 10 ] }` or `x . { (target) [ 10 ] }` is valid; bare `x . { target [ 10 ] }` is rejected as a Form getter field.

[] — Stream (Executable body: Lazy sequence of operations, Predicates, Loops, Lambdas, Dialects).
            • Leading-dot relative operators (`.field`, `.0`, `.method`, `.+ 1`) bind strictly to the innermost enclosing `[...]`
              (or expand to `[ .field ]` when passed as a stream argument or router key via Sugar #3).
            • Enclosing Forms and outer stream inputs are accessed via Law 1a direct lexical scope, Law 1c `PascalCase` -> `snake_case`
              auto-projected names (`User` -> `user`, `Cluster` -> `cluster`), or explicit parameter names.

() — Tuple  (Ordered data, Fixed-size heterogeneous `(_A, _B)`, Homogeneous slice `Tuple(_T)`, Positional projection of `{}`, Eager evaluation).
            • Canonical Triad `Name(Slots) {Form} [Stream]`, Auto-Generics & Form-Tuple Slot Duality:
              - `Name { ... }`                     -> Form / Struct / Module (`PascalCase`, immutable Prototype/Type)
              - `Box { item }` or `Some(value)`    -> Auto-Generic Open Slot (untyped `snake_case` field specializes on construction)
              - `Range { start _T, end _T }`       -> 1st-Order Generic Form (`_PascalCase` like `_T`, `_K`, `_V` is a Named Type Hole)
              - `Name(_F(_)) { ... }`              -> 2nd-Order Generic Form / HKT (`_F(_)` is Type Constructor `* -> *`)
              - `Name(_T(_(_), _)) { ... }`        -> 3rd-Order Generic Form (`_T` has Kind `(* -> *) -> * -> *`)
              - `Name(_TC Category, cat _TC)`      -> Bounded Generic Form (`_TC Category` constrains `_TC <: Category`; `cat _TC` is value slot)
              - `Name(Age) { new_age age }`        -> Auto-Projected Class (single known Type `Age` auto-binds `age Age` via Law 1c)
              - `Name(u1 User, u2 User) { ... }`   -> Duplicate or compound Type Slots (`A & B`) require explicit `snake_case` names
              - `Name { id String, meta _T }`      -> Form-Tuple Constructor Duality: callable via `Name { id "1", meta 99 }` or `Name("1", 99)`
              - `name { a Int, b Int 0 } [Body]`   -> Named function with Form slots `{}` (callable via `name { a 1 }` or `name(1, 2)`)
              - `name(args) ReturnType [Body]`     -> Named function with explicit return type (`snake_case`)
              - `name(args) [Body]`                -> Named function (with optional defaults `name(k 10)` or variadic `name(first, ..rest)`)
              - `name() [Body]`                    -> Parameterless function (explicit `()` required on direct call `name()`)
              - `(args) [Body]` or `[Body]`        -> Anonymous lambda `(args) [Body]` or relative pipe stream `[ .+ 1 ]`
              - `name(args) ReturnType`            -> Method contract signature inside a Form `{}`
              - `cb (_A) [_B]`                     -> Function Type signature in parameter or field slots

2. CASE RULE, ZERO KEYWORDS (`No self/Self`), NAMED TYPE HOLES (`_T`) & LEXER RULES:
• PascalCase  — Concrete Types, Forms, Classes, Modules, Union Variants (`User`, `Age`, `Option`, `Compile`).
                Acronyms use title case (`Http`, `Ui`, `Ai`, `Json`, `Gpr`) so `PascalCase` <-> `snake_case` is strictly 1-to-1.
                • Valid vs Error (`E06_IMMUTABLE_TYPE_BINDING`): `my_car (Car {})` then `my_car.max_speed .= 500` is valid;
                  mutating a prototype directly via `Car.max_speed .= 500` is a compile error.
• snake_case  — Variables, Field keys, Functions, Methods, Value Params (`user_id`, `find_user`).
                Immutable by default when bound via space `a (expr)`; updated in-place inside `[]` only via explicit
                relative update operators: `a .= val`, `a .+ 1`, `a .- 1`, `a .* 2`, `a .& { ... }`.
• '_' & '_T'  — Anonymous Hole (`_`) & Named Type Hole (`_PascalCase`: `_T`, `_A`, `_B`, `_Key`, `_Val`, `_M`):
                • Bare `_` is an anonymous wildcard (`Any`) in router branches `{ _ [...] }`, tuple patterns `(0, _)`, and kinds `_F(_)`.
                • `_PascalCase` is a Generic Type Variable (Named Type Hole) that never collides with concrete `PascalCase` types.
                • Valid vs Error (`E03_UNKNOWN_CONCRETE_TYPE`): `Pair { a _T, b _T }` is valid; `Pair(T) { a T, b T }` (where `T` is not a
                  defined concrete Type) is rejected because unbound type holes require the `_` prefix (`_T`).
• Compile     — Compile-Time Evaluation Tag (`Compile & [ ... ]` or `fn(...) [...] & Compile`).
                • Valid vs Error (`E07_STATIC_ASSERT_FAILED`): `Meta.error("msg")` inside `Compile & [...]` halts compilation with `"msg"`.
• 'symbol'    — Paired Symbol Literal: exact constant tag without interpolation (`'draft'`, `'GET'`).
• "..."       — Standard String (multiline): evaluates `{expr}` inside and supports `\` escapes (`\n`, `\t`, `\\`, `\"`, `\{`, `\}`).
• r"..."      — Raw String (multiline, also numbered `r0"..."0`, `r1"..."1`, `r2"..."2`): zero `\` escapes and zero `{}` interpolation.
• '#'         — Homoiconic Comments (attached to AST as `Meta.doc_of`):
                • `# comment` — single-line comment (when `#` is followed by space/text on the same line).
                • `#\n ... \n#` (or `#1\n ... \n1#`) — multiline block comment (when `#` is followed immediately by `\n`).
• ',' & '\n'  — Universal Element Separator: interchangeable across `{}`, `[]`, `()`, except when the next line starts with a binary/pipeline
                operator.
                • Valid vs Error (`E09_DANGLING_BIND_NEWLINE`): `discount [ amount - 10 ]` is valid; placing `\n` before the opening bracket
                  `discount \n [ amount - 10 ]` splits `discount` and `[...]` into two separate elements.
• ' ' (Space) — Pair & Token Separator (Overloadable AST node `Bind(Target, Source)`):
                • Declares a field in `{}`, a module member, or a new local binding in `[]`.
                • Valid vs Error (`E01_AMBIGUOUS_CALL_OR_BIND`): `area(Circle { radius 5.0 })` calls `area`, and `c_area (Circle { radius 5.0 } . area)`
                  binds `c_area`; `area (Circle { radius 5.0 })` with a space on an existing function name `area` is rejected.
                • Valid vs Error (`E10_MULTIPLE_REST_PATTERNS`): `(0, ..a, ..b)` on the right spreads multiple slices, and `(first, ..rest) nums`
                  on the left captures the tail; `(first, ..r1, ..r2) nums` with two rest captures on the left is rejected.

3. COMPLETE OPERATOR TABLE & SYNTACTIC SUGAR CATALOG:
'a.b'     (no spaces)   — Tight Path Access: field, module, ancestor `a.Parent.field`, index `a.0`, `a.-1`, or method call `a.fn()`
'.field'  (tight prefix)— Relative Getter / Method: accesses field, index `.0`, or method `.trim` of innermost `[...]` input
'.+ b'    (tight prefix)— Relative Operator Step: applies operator (`.+ 1`, `.>= 18`, `.== 0`) to innermost `[...]` input
'!x'      (tight prefix)— Logical / Type Negation: boolean `!flag` or Closed-World Negative Type Guard `Order & !Cancelled`
'-x'      (tight prefix)— Arithmetic Negation: negates number, variable, or group (`-5`, `-vel`, `-(a + b)`)
'..x'     (tight prefix)— Symmetric Slice Spread (in Source) & Rest Capture (in Target):
                          • In `{}` / File  : `..Vehicle`, `..root.prelude`, `..Path"../ext.rl"` embeds public members & lineage
                          • In `()` / `fn`  : `(1, ..other_tuple, 4)`, `fn(..args)`, or variadic definition `fn(first, ..rest) [...]`
                          • In `[]`         : `[ ..validation_stream, save ]` inlines stream steps
                          • On left of bind : `(first, ..rest) nums` or `{ id, ..rest } user` captures remaining elements/fields
'a .= b'  (with spaces) — Explicit In-Place Assignment (`snake_case` variable or deep path `a.b.0 .= val`; Perceus O(1) mutation)
'a .+ b'  (with spaces) — Explicit In-Place Compound Update (`a .+ 1`, `a .- 1`, `a .* 2`, `a ./ 2`, `a .& { ... }`)
'a . b'   (with spaces) — Dataflow Pipeline: passes `a` left-to-right (`a . b` == `b(a)`, `a . b(c, d)` == `b(a, c, d)`)
'a .> b'                — Tap Pipeline: executes side-effect `b(a)` and passes original `a` further
'a .& b'                — Enrich Pipeline (or in-place enrich on variable): calls `b(a)` and merges `&` result with `a`
'a ./ b'                — Subtract/Strip Pipeline: removes field path, key symbol, tag, or ancestor `b` from `a` (`a / b`)
'a .? b'                — Positive Filter, Pattern Test & Conditional Exit Pipe (Triggers when TRUE):
                          • Filter / Pattern test `val .? Pred` : if `Pred` is a Stream, Type, Range, or Union, tests `val`;
                            in a pipeline passes narrowed `val` if true (else `None & Bypass`), or returns `Bool` in boolean context.
                          • Conditional exit `cond .? BypassVal` : if `BypassVal <: Bypass` (`Break`, `Return`) and `cond` is `true`,
                            immediately exits the enclosing loop or stream with `BypassVal`.
'a .! b'                — Negative Guard / Assert Pipe (Triggers when FALSE or `None`):
                          • Flat guard `cond .! ErrVal` : asserts `cond` is `true`; if `false`, short-circuits stream with `ErrVal` (`& Bypass`).
                          • In pipeline `val .? Pred .! ErrVal` : if `val .? Pred` yields `None`, replaces `None` with `ErrVal`.
'a .| {}'               — Parallel Pipeline: concurrent fan-out execution of branches in `{}`, merged via `&`
'a & b'   (with spaces) — Universal Algebraic Composition / Intersection (7 Canonical Combinations):
                          1. Form & Form       -> Inheritance, Trait implementation, or Structural Merge with Lineage
                          2. Type & Stream/!T  -> Refinement Type (`Int & [ .> 0 ]`) or Negative Type Guard (`Order & !Cancelled`)
                          3. Function & Form   -> Partial application / Default & DI override (`fn & { db MockDb }`)
                          4. Stream & Stream   -> Sequential pipeline composition (`[ a . b ]`)
                          5. Value & TagForm   -> Metadata / visibility / compile-time attachment (`val & Private`, `Compile & [...]`)
                          6. TypeA & TypeB(A)  -> F-Bounded Trait constraint (`User & Comparable(User)`)
                          7. Type & Bypass     -> Marks a failure/empty/control Union outcome (`NotFound & Bypass`) for Law 3 short-circuiting
'a | b'                 — Union: algebraic sum of types, patterns, or outcomes
'a / b'                 — Structural Subtraction (for `PascalCase` Types/Forms or Maps) vs Overloaded `/` (for `snake_case` Values):
                          • Form / FieldPath   -> Removes specific field (`User / User.password_hash`)
                          • Form / 'key'       -> Removes field key from all ancestors (`Combined / 'name'`)
                          • Form / Ancestor    -> Removes `Ancestor` slice from `Form` lineage (`ElectricCar / Vehicle`)
                          • Form / TagForm     -> Removes all members tagged with `TagForm` (`AuthInternal / Private`)
                          • Map  / key_val     -> Removes entry key from a Map (`config_map / "host"`)
                          • Union / Type       -> Removes branch from Union (`Result / Error`)
                          • val_a / val_b      -> On runtime value instances (`snake_case` or `(expr)`), invokes overloaded `/` method

OFFICIAL SYNTACTIC SUGAR CATALOG (100% Desugaring Equivalences):
1. Tagged String Sugar        : `Type"..."`               ==> `Type("...")` (e.g. `Money"199.99"` ==> `Money("199.99")`)
2. UFCS Pipeline Sugar        : `a . fn` or `a . fn(b)`   ==> `fn(a)` or `fn(a, b)` (and `a.fn(b)` ==> `fn(a, b)`)
3. Point-Free Relative Sugar  : `.field` or `.op val`     ==> `[ .field ]` or `[ .op val ]` when passed as a `Stream` argument
                                (`items . map(.price)`, `users . filter(.age >= 18)`, `nodes .? .is_healthy`) or Router key (`.>= 2000 [ 300 ]`)
4. Auto-Generic Open Slot     : `Box { item }`            ==> `Box(_T) { item _T }` (any untyped `snake_case` field is polymorphic)

4. FOUR IRON LAWS OF RAZOMY & COMPILER GUARANTEES:
- LAW 1 (UNIFIED LEXICAL SCOPING, INNERMOST RELATIVE `.` & UNIVERSAL `PascalCase` -> `snake_case` TYPE PROJECTION):
  Razomy has zero context keywords (`no self`, `no Self`, `no _0`, `no _._`). Values enter `{ form }` or `[ stream ]` in 4 ways:
  (a) Direct Lexical Scope — inside any Form `FormName { ... }` or enrichment block `target & { ... }` / `target .& { ... }`,
      fields of `FormName` (or `target`) and slot parameters are directly visible by name in all sibling fields and `[...]` getters.
  (b) Innermost Leading-Dot Relative Access (`.field`, `.0`, `.method`, `.+ x`) — inside `[...]`, a leading dot always targets the
      input of the innermost enclosing `[...]`. It never crosses `[...]` boundaries.
  (c) Universal Single-Type Projection (`KnownType` -> `known_type`) — whenever a single atomic concrete `PascalCase` Type is the subject
      of a Form definition `FormName { ... }`, a slot `Name(KnownType)`, or a router branch `{ KnownType [...] }`, it automatically binds
      its `snake_case` instance variable `known_type` (`User` -> `user`, `LedgerNode` -> `ledger_node`, `Cluster` -> `cluster`).
      • Valid vs Error (`E02_DUPLICATE_UNNAMED_TYPE`): `transfer(src Account, dst Account, Money)` is valid; `transfer(Account, Account, Money)`
        with duplicate unnamed `Account` types is rejected.
  (d) Explicit Parameter / Destructuring — `fn(u User)`, `fn { name } [...]`, or `Some(val) [...]`.
- LAW 2 (LINEAGE NAMESPACING & DEAD LINEAGE ELIMINATION): Upon merge `A & B` or `..A`, unique fields are accessed directly (`ctx.age`)
  or via ancestor path (`ctx.A.age`). Colliding fields require explicit ancestor path (`ctx.A.id`, `ctx.B.id`) until resolved via `/`.
  • Valid vs Error (`E05_AMBIGUOUS_LINEAGE_FIELD`): `entity.User.name` or `(entity / Cloud.name).name` is valid; unqualified `entity.name`
    on `User & Cloud` when both define `name` is rejected.
- LAW 3 (SUBTYPING-DRIVEN `Bypass` OUTCOME SHORT-CIRCUITING): When `A | B` flows into `fn(Param)`, partial router `{ Param [...] }`,
  or typed local binding `val Param (expr)` where `A <: Param` and `B` is not a subtype of `Param`:
  • Valid vs Error (`E04_UNHANDLED_UNION_BRANCH`): If `B <: Bypass` (`None`, `NotFound`, `Break`, `Return`), `B` automatically short-circuits
    into return type `Out | B`; if `B` is not marked with `& Bypass` (`User | Company | NotFound` flowing into `fn(User)`), compilation fails.
- LAW 4 (UFCS AUTOCOMPLETE, ZERO-COST GETTERS & GUARANTEED TCO): Any `fn(x Type)` in scope is suggested by IDE autocomplete on `val .`.
  Lazy fields `k [e]` compile to zero-cost static getters, and tail-recursive streams / `Loop [...]` compile to zero-allocation machine loops.
"3


#
0. CORE AXIOMS, UNIVERSAL TYPE PROJECTION (LAW 1) & LINEAGE HISTORY
#

# Built-in Compile-Time & Control Contracts:
Compile {} # Forces compile-time evaluation (`Compile & [...]` or `fn(...) [...] & Compile`)
Bypass  {} # Marks Union failure/empty/control branches (`None`, `NotFound`, `Break`, `Return`) that short-circuit via Law 3

# --- A. The 4 Core Axioms of Forms & Lineage ---

# Axiom 1: [TypeName] [BaseType] (type alias) or [var_name] [value] (value binding)
BrandName String
default_user_name "Alice"

# Axiom 2: [FormName] { [key] [Type_or_Default] } (Declares an immutable Form Prototype/Type)
Vehicle {
  brand     BrandName            # Open field of type `BrandName` (`String`)
  max_speed Int 180              # Explicit type `Int` with default value `180`
  tag       "standard"           # Inferred type `String` with default `"standard"`
}

# Axiom 3: Form Composition via Prefix Lineage Spread `..Ancestor` inside `{}` or Binary `&`
HasBattery {
  battery_kwh Int 75
}

ElectricVehicle {
  ..Vehicle
  ..HasBattery
  charging_port String "Type2"
}

ElectricVehicleIntersectStyle Vehicle & HasBattery & {
  charging_port String "Type2"
}

# Axiom 4: [NewFormName] [BaseForm] { [key] [Override_Value] }
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

# 1. Tight dot '.' access on Form Prototypes: Active Defaults vs Ancestor Lineage History
MyCar.brand                              # "CustomPorsche" (active default on MyCar prototype)
MyCar.SportsCar.brand                    # "Porsche" (value at SportsCar ancestor level)
MyCar.SportsCar.Car.brand                # "GenericMotor" (value at Car ancestor level)
MyCar.Vehicle.brand                      # BrandName (base specification type on root Vehicle)

MyGarageCar MyCar & { garage_slot 1 }
MyGarageCar.brand                        # "CustomPorsche"
MyGarageCar.MyCar.SportsCar              # Access to ancestor slice
MyGarageCar.0                            # Access default by field index 0 ("CustomPorsche")
MyGarageCar.-1                           # Access default by field index -1 from end (1)

# 2. Instantiating a Form Prototype & Updating Data via `.=`, `.&`, or `&`:
mutate_car_demo() [
  my_car (MyGarageCar {})

  # Create new updated Form instance via `&` while preserving Lineage history:
  upgraded_car (my_car & { max_speed 350, garage_slot 2 })

  # Explicit in-place update of `snake_case` variable via `.&` or `.=`:
  my_car       .& { max_speed 320 }
  my_car.brand .= "Tesla"
  my_car.0     .= "Rivian"
  my_car.-1    .= 42
]

# --- B. Universal Slot Rule: Auto-Generics, Named Type Holes (`_T`), Bounded Generics & Law 1 Parameters ---

Category { title String }
Age      Int

# 1. Auto-Generic Open Form (`item` has no fixed type -> polymorphic `_T`):
Box {
  item
}

# 2. Named Type Hole (`_T` links multiple fields to the same type):
PairOfSame {
  first  _T
  second _T
}

# 3. Bounded Generic Form (`_TC Category` constrains `_TC <: Category`; `active_cat _TC` is a value slot):
CategorizedUser(_TC Category, active_cat _TC) {
  backup_cat _TC
  current    active_cat
}

# 4. Single Auto-Projected Concrete Type (`Age` auto-binds `age` via Law 1c):
UserWithAge(Age) {
  new_age age
}

is_user_18(Age) [
  age == 18
]

# 5. Multiple Identical Concrete Type Parameters use explicit `snake_case` names:
compare_ages(candidate_age Age, minimum_age Age) [
  candidate_age >= minimum_age
]

# 6. Explicit Value Parameter:
GreetingCard(recipient String) {
  message "Hello, {recipient}!"
}

# 7. Form-Tuple Constructor Duality (Open fields vs Default fields vs Dynamic `[]` fields):
ProfileCard {
  id       String                # Open slot 0
  name     String                # Open slot 1
  meta     _T                    # Open slot 2
  role     String 'user'         # Has default -> skipped in positional `()` call unless overridden
  is_admin [ role == 'admin' ]   # Lazy getter `[]` -> accesses sibling `role` via Law 1a
}

card_by_form  ProfileCard { id "u1", name "Alice", meta 99 }
card_by_tuple ProfileCard("u1", "Alice", 99)

# --- C. Law 1 Scoping: Direct Lexical Fields, Type Projection, Innermost `.` & Prefix `..` ---
WithName { name String }

ReadViaLexicalScope {
  name "Razomy"
  get_name()           [ name ]                         # Law 1a: Direct lexical access to sibling field `name`
  get_via_projection() [ read_via_lexical_scope.name ]  # Law 1c: Form `ReadViaLexicalScope` auto-projects `read_via_lexical_scope`
}

read_via_type(WithName)                   [ with_name.name ]                  # Law 1c: Single `WithName` auto-binds `with_name`
join_two(first WithName, second WithName) [ "{first.name} & {second.name}" ]  # Law 1d: Explicit names for 2+ identical types
read_via_var(w WithName)                  [ w.name ]                          # Law 1d: Explicit parameter variable `w`
read_unpacked({ name } WithName)          [ name ]                            # Law 1d: Form field destructuring `{ name }`

unpacking_demo(nums (Int, Int, Int, Int), user_form WithName) [
  (first_num, second_num, ..rest_nums) nums
  { name, ..other_fields }             user_form
  expanded (0, ..rest_nums, 99)
  "{name}: {first_num + second_num}, count={expanded.length}"
]

extract_name_demo(prefix String, WithName) [
  with_name . [
    "{prefix}: {.name} (via type projection: {with_name.name})"
  ]
]

sum_pair_pipe(pair (Int, Int)) [
  pair . [ _.0 + _.1 ]
]

do_init() [ "Razomy v6.4 initialized" . Shell.log ]
do_init()


#
1. PRIMITIVES, EXACT DECIMAL `Money`, `Range`, TAGGED STRINGS (`Type"..."`) & RAW STRINGS
#

String ""
Symbol 'symbol'
Int    0
Float  0.0
Bool   false
Byte   0x00
Any    _

# Symmetric Range Form (`_T` ensures `start` and `end` share the same type):
Range {
  start _T
  end   _T
  contains(val _T) [ (val >= start) && (val <= end) ]
}

# Exact Fixed-Point `Money` (4 decimal places stored as integer micro-units: 1.0000 == 10000):
Money(raw String "0.00") {
  scaled (raw . Decimal.parse_scale4)

  from_scaled(val Int) [ Money { scaled val } ]

  +   (other Money)   [ Money.from_scaled(scaled + other.scaled) ]
  -   (other Money)   [ Money.from_scaled(scaled - other.scaled) ]
  *   (factor Float)  [ Money.from_scaled((scaled.to_float * factor).round_to_int) ]
  /   (divisor Float) [ Money.from_scaled((scaled.to_float / divisor).round_to_int) ]
  ==  (other Money)   [ scaled == other.scaled ]
}

Usd(raw String)        { money Money(raw), currency 'USD' }
Date(iso String)       { value iso }
Time(iso String)       { value iso }
Duration(spec String)  { value spec }
Pair(spec String)      { value spec }
Hex(code String)       { value code }
Ip(addr String)        { value addr }
Version(semver String) { value semver }
Url(href String)       { value href }
Path(fs_path String)   { value fs_path }

StringExamples {
  interpolated "Hello, {default_user_name}!
This string spans multiple lines natively
and escapes \"quotes\" and \{braces\}."

  regex_pat    r"^[^@]+@[^@]+\.[^@]+$"
  nested_json  r1"{"query": "SELECT * FROM \"users\"", "interpolated": "{ignored}"}"1
  nested_code  r2"Can freely embed both " and "1 without any lexer conflict!"2
}

HumanLiterals {
  money_explicit    Money("199.99"),        money_val    Money"199.99"
  currency_explicit Usd("500.00"),          currency_val Usd"500.00"
  date_explicit     Date("2026-03-30"),     date_val     Date"2026-03-30"
  time_explicit     Time("14:30:00"),       time_val     Time"14:30:00"
  duration_explicit Duration("5m30s"),      duration_val Duration"5m30s"
  coord_explicit    Pair("1920x1080"),      coord_pair   Pair"1920x1080"
  color_explicit    Hex("#2A9D8F"),         color_val    Hex"#2A9D8F"
  ip_explicit       Ip("192.168.0.1:8080"), ip_addr      Ip"192.168.0.1:8080"
  version_explicit  Version("1.4.2-beta"),  version_val  Version"1.4.2-beta"
  url_explicit      Url("https://razomy.org"), web_url   Url"https://razomy.org"
  path_explicit     Path("./data/users.json"), local_file Path"./data/users.json"

  tax_rate          0.155
  email_addr        "admin@razomy.org"
}


#
2. TUPLES `()`, FORMS `{}`, MAPS & TYPE ALGEBRA (`&`, `|`, `/`)
#

# 1. Fixed-Size Tuples `(Float, Float)` vs Homogeneous Slices `Tuple(_T)` & Spread `..tuple`
Point2D    (Float, Float)
origin     (0.0, 0.0)
extra_dims (1.0, 2.0)
point_4d   (..origin, ..extra_dims)

# 2. Type Unions (Sum Types) with paired symbols `'...'`
Status    'draft' | 'published' | 'archived'
Primitive String | Int | Float | Bool | Byte | Money

# 3. Forms `{}` (Structures) & Lineage Inheritance
Identifiable { id String }
Auditable    { created_at Int, updated_at Int }

User Identifiable {
  id            String
  name          String
  age           Int
  role          'admin' | 'manager' | 'user' | 'guest'
  password_hash String
}

Cloud Identifiable {
  id     String
  name   String
  region String
}

# 4. Compile-Time Structural Type Subtraction (`/`) & Lineage Disambiguation:
PublicUser         User / User.password_hash
AnonymousUsr       PublicUser / 'name'
UserAndCloud       User & Cloud
UserWithCloudName  UserAndCloud / User.name
CloudWithUserName  UserAndCloud / Cloud.name
NoNameEntity       UserAndCloud / 'name'
RestoredUser       UserAndCloud / Cloud
GuestUser          PublicUser / PublicUser.role & { role 'guest' }

# 5. Generic Key-Value Map & Set (`(_K)` and `_V` are Named Type Holes):
Map {
  (_K) _V
  get(m Map(_K, _V), key _K) [ (m, key) . native_map_get ]
}
Set(_T) { (_T) Bool }

config_map {
  "host" "localhost"
  "port" "8080"
}

updated_config config_map & { "port" "9090", "tls" "true" } / "host"


#
3. UPDATING DATA: PURE BINDING `a b` vs EXPLICIT IN-PLACE UPDATE `a .= val`, `a .+ 1`, `a .& {...}`
#

Order      { id String, amount Int }
Department { title String, budget Int }
Company    { name String, departments Tuple(Department) }

update_order_demo() [
  raw_order (Order { id "o1", amount 100 })

  # 1. Explicit In-Place Operator Update (`.+`, `.-`, `.=`):
  counter 0
  counter .+ 1
  counter .- 1
  counter .= 10

  # 2. Explicit Deep Path & Positional Index Update via `.=` (Perceus O(1) mutation when uniquely owned):
  raw_order.amount .= 14
  raw_order.1      .= 20

  corp (Company { name "Acme", departments (Department { title "R&D", budget 1000 }) })
  corp.departments.0.budget .= 2500

  # 3. Explicit In-Place Enrich Update (`.& { ... }`):
  raw_order .& { amount 25 }

  # 4. Creating a New Immutable Updated Instance via `&`:
  next_order (raw_order & { amount 50 })
]


#
4. COMPLETE MATRIX OF `&` COMBINATIONS & TRAIT COMPOSITION
#

# 1. Form & Form -> Structural Merge / Inheritance / Trait Implementation:
Printable {
  format() String
}

PrintableUser User & Printable {
  format() [ "User({name}, id={id})" ]
}

# 2. Type & Stream `[...]` or `!Type` -> Refinement Type & Closed-World Negative Type Guard:
PositiveInt Int  & [ .> 0 ]
AdultUser   User & [ .age >= 18 ]

# 3. Function & Form `{...}` -> Partial Application / Default Override / Dependency Injection:
connect { host String "localhost", port Int 8080, tls Bool false } [
  "Connecting to {host}:{port} (TLS: {tls})"
]
secure_connect connect & { port 9090, tls true }

# 4. Stream `[...]` & Stream `[...]` -> Sequential Function Composition:
trim_str      [ .trim ]
upper_str     [ .to_upper ]
clean_str     trim_str & upper_str
clean_inlined [ ..trim_str, ..upper_str ]

# 5. Value/Member & TagForm -> Metadata / Visibility / Compile-Time Attachment:
Public  {}
Private {}
secret_key "xyz987" & Private

# 6. Type & Parameterized Trait `Trait(_Other)` -> F-Bounded Polymorphism:
Comparable(_Other) {
  < (other _Other) Bool
}

SortableUser User & Comparable(User) {
  < (other User) [ age < other.age ]
}

# 7. Type & `Bypass` -> Marks an Empty/Error/Control Outcome for Law 3 Pipeline Short-Circuiting:
None     {} & Bypass
NotFound { entity String, id String } & Bypass

# Open Extension Methods (UFCS):
greet(User) [ "Welcome, {user.name}!" ]


#
5. PARAMETERIZED FORMS, EAGER `()` vs LAZY `[]` & OPERATOR OVERLOADING
#

MathOps(a Int, b Int) {
  add      (a + b)
  sub      (a - b)
  mul      (a * b)
  div      (a / b)
  mod      (a % b)
  pow      (a ** b)
  neg_a    (-a)
  lazy_div [ a / b ]

  bit_and  (a . bit_and(b))
  bit_or   (a . bit_or(b))
  bit_xor  (a . bit_xor(b))
  shift_l  (a << 2)
  shift_r  (a >> 2)
}

Vec2 {
  x Float
  y Float

  +  (other Vec2)  [ Vec2 { x (x + other.x), y (y + other.y) } ]
  -  (other Vec2)  [ Vec2 { x (x - other.x), y (y - other.y) } ]
  *  (scale Float) [ Vec2 { x (x * scale),   y (y * scale) } ]
  /  (scale Float) [ Vec2 { x (x / scale),   y (y / scale) } ]
  == (other Vec2)  [ (x == other.x) && (y == other.y) ]
}

Shape {
  vertices Tuple(Vec2)
  / (cutter Shape) [ (shape, cutter) . native_csg_subtract ]
}

LogicOps(x Bool, y Bool, val Int) {
  and_op   (x && y)
  or_op    (x || y)
  not_op   (!x)
  eq       (val == 42)
  neq      (val != 0)
  gt       (val > 10)
  gte      (val >= 10)
  lt       (val < 100)
  lte      (val <= 100)
  in_range (val .? Range(1, 100))
}

string_ops(first String, last String) [
  full "{first} {last}"

  full
  . trim
  . to_upper
  . split(" ")
]


#
6. CONTROL FLOW: POSITIVE `.?` (FILTER / EXIT ON TRUE), NEGATIVE `.!` (ASSERT ON FALSE), `Loop` & `Break`
#

Break  { value _T, label Symbol 'default' } & Bypass
Return { value _T } & Bypass
Loop(body Stream) [ body . native_loop ]

MinorError { name String } & Bypass
EmptyError {} & Bypass

process_checkout { user User, items Tuple(Int) } [
  user.age >= 18   .! MinorError { name user.name }
  items.length > 0 .! EmptyError {}

  total 0
  i     0
  Loop [
    i >= items.length .? Break(total)
    total .+ items.get(i)
    i     .+ 1
  ]
]

classical_fold_demo(numbers Tuple(Int), limit Int) [
  limit <= 0 .? Return(0)
  numbers . fold(0, (acc Int, n Int) [
    next_acc (acc + n)
    next_acc > limit .? Break(next_acc)
    next_acc
  ])
]


#
7. FUNCTION ARGUMENTS: FORM-TUPLE DUALITY, AUTO-PROJECTED SLOTS, DEFAULTS, DI & PARTIAL APPLICATION
#

Shell {
  log(msg String)  [ msg . native_print ]
  exec(cmd String) [ cmd . native_exec ]
}

Cart { cart_id String, amount Int }

Db {
  find_user(id String) User | NotFound [
    User { id id, name "Alice", age 25, role 'user', password_hash "xyz" }
  ]

  find_cart(Identifiable) [
    Cart { cart_id "c_for_{identifiable.id}", amount 2500 }
  ]
}

default_conn   connect()
pos_conn       connect("10.0.0.1", 9000, true)
form_conn      connect { port 9090, tls true }
partial_conn   connect("db.internal")
custom_connect connect & { port 9090, tls true }
active_conn    custom_connect()

FindOptions { case_sensitive false, limit 0, last false }

find(collection, target, opts FindOptions) [
  collection . native_find(target, opts.case_sensitive, opts.limit, opts.last)
]

search_users(names) [
  names . find("Alice", FindOptions & { case_sensitive true, limit 5 })
]


#
8. PIPELINE OPERATORS (`.`, `.>`, `.&`, `./`, `.?`, `.!`, `.|`), POINT-FREE SUGAR & FLAT `do`-NOTATION
#

finalize_bill(ctx User & Cart) [
  ctx & {
    discount [
      amount . {
        .>= 2000 [ 300 ]
        .>= 1000 [ 100 ]
        _        [ 0 ]
      }
    ]
    final_price [ amount - discount ]
  }
  ./ User.password_hash
  .> [ "Receipt for {.name}: {.final_price} USD (Discount: {.discount})" . Shell.log ]
]

process_order_pipeline(user_id String, db Db) [
  user_id
  .  db.find_user
  .? [ .age >= 18 ] .! MinorError { name "underage" }
  .& db.find_cart
  .  finalize_bill
]

load_user_and_friend(uid1 String, uid2 String) [
  u1 User (uid1 . Db.find_user)
  u2 User (uid2 . Db.find_user)
  "{u1.name} is friends with {u2.name}"
]


#
9. REFINEMENT TYPES & UNIVERSAL ROUTER AXIOM (`{ Pattern [Body] }`)
#

is_adult(User)   [ user.age >= 18 ]
is_minor(User)   [ user.age < 18 ]
is_manager(User) [ user.role .? ('admin' | 'manager') ]
non_empty        [ .length > 0 ]
EmailStr         String & [ .Regex.matches(r"^[^@]+@[^@]+\.[^@]+$") ]

Adult   User & is_adult
Minor   User & is_minor
Manager User & is_manager

classify_account(account User) [
  account . {
    Manager [ "Dashboard: {manager.name}" . Shell.log ]
    Adult   [ "Adult account: {adult.name}" . Shell.log ]
    Minor   [ "Child account: {minor.name}" . Shell.log ]
    _       [ "Unknown profile" . Shell.log ]
  }
]

Circle   { radius Float }
Rect     { width Float, height Float }
Geometry Circle | Rect

area(g Geometry) [
  g . {
    Circle [ 3.14159 * circle.radius * circle.radius ]
    Rect   [ rect.width * rect.height ]
  }
]


#
10. COLLECTIONS IN PARENS `()`, STREAM ITERATION (`zip`, `enumerate`, `fold`) & TUPLE MATCHING
#

Item { name String, price Int, in_stock Bool }

order_list (
  Item { name "Laptop", price 1200, in_stock true }
  Item { name "Mouse",  price 25,   in_stock false }
  Item { name "Screen", price 400,  in_stock true }
)

calculate_total_in_stock(items Tuple(Item)) [
  items
  . filter(.in_stock)
  . map(.price)
  . fold(0, (acc Int, price Int) [ acc + price ])
]

format_numbered_catalog(items Tuple(Item), discounts Tuple(Int)) [
  items
  . zip(discounts)
  . enumerate
  . map((idx Int, (item Item, disc Int)) [
    "#{idx}: {item.name} -> {item.price - disc} USD"
  ])
]

fizz_buzz(limit Int) [
  Range(1, limit) . map((n Int) [
    (n % 3, n % 5) . {
      (0, 0) [ "FizzBuzz" ]
      (0, _) [ "Fizz" ]
      (_, 0) [ "Buzz" ]
      _      [ "{n}" ]
    }
  ])
]


#
11. LINEAGE COLLISION RESOLUTION (LAW 2) & TYPE MIGRATION
#

UserOrder User & Order

process_combined(combined_context) [
  combined_context . {
    UserOrder [
      "Client Name: {user_order.name}, Order Amount: {user_order.amount}" . Shell.log
      "User ID: {user_order.User.id}, Order ID: {user_order.Order.id}" . Shell.log
    ]
  }
]

UserOrderClean UserOrder / Order.id

ClientAccount { account_id String, name String, is_adult Bool }

migrate_user(User) [
  ClientAccount {
    account_id user.id
    name       user.name
    is_adult   (user.age >= 18)
  }
]


#
12. REACTIVE FORMS (DIRECT LEXICAL FIELDS) & UNIFIED REACTIVE SIGNALS (`Signal`, `Effect [...]`)
#

Invoice {
  price    Money"100.00"
  quantity 3.0
  tax_rate 0.20

  subtotal [ price * quantity ]
  tax      [ subtotal * tax_rate ]
  total    [ subtotal + tax ]
}

order_1 Invoice & { quantity 5.0 }

count_sig   Signal(2)
doubled_sig [ count_sig() * 2 ]

run_signal_demo() [
  Effect [
    "Count: {count_sig()}, Doubled: {doubled_sig()}" . Shell.log
  ]
  count_sig(10)
  count_sig .+ 5
]


#
13. ONTOLOGICAL TYPING (TYPESTATE), NEGATIVE TYPE GUARD (`& !State`) & OUTCOME BYPASS (LAW 3)
#

Reserved   { reserved_at Int }
Paid       { transaction_id String }
Cancelled  { cancelled_at Int }

OutOfStock { reason String } & Bypass
NoMoney    { reason String } & Bypass

reserve_stock(o Order & !Cancelled) [
  o.amount > 0 .! OutOfStock { reason "Warehouse empty" }
  o & Reserved { reserved_at 1620000000 }
]

charge_payment(o Order & Reserved) [
  o.amount < 50000 .! NoMoney { reason "Insufficient funds" }
  o & Paid { transaction_id "tx_001" }
]

execute_fulfillment(initial_order Order & !Cancelled) [
  initial_order
  . reserve_stock
  . charge_payment
]

ship_to_customer(order Order & Reserved & Paid) [
  "Shipping order {order.id}. Transaction {order.transaction_id} confirmed." . Shell.log
]

audit_any_outcome(outcome (Order & Reserved & Paid) | OutOfStock | NoMoney) [
  "Auditing fulfillment outcome: {outcome}" . Shell.log
]


#
14. AUTO-GENERICS, NAMED TYPE HOLES (`_T`, `_A`, `_B`) & 1ST/2ND/3RD-ORDER GENERICS (HKT & MONADS)
#

Zero      {}
Succ      { prev _N }
One       Succ(Zero)
Two       Succ(One)
Three     Succ(Two)

FixedVector {
  data   Tuple(_T)
  length _N
}

Json String | Int | Float | Bool | None | Tuple(Json) | Map(String, Json)

Cons {
  head _T
  tail List(_T)
}
List(_T) None | Cons(_T)

RoseTree {
  value    _T
  children Tuple(RoseTree(_T)) ()
}

Leaf {}
Node {
  value _T
  left  Tree(_T)
  right Tree(_T)
}
Tree(_T) Leaf | Node(_T)

sort(items List(Comparable)) [
  items . native_sort_by((a, b) [ a < b ])
]

sort_users(users List(User & Comparable(User))) [
  users . sort
]

NestedBox(_T) Box(Box(_T))

Functor(_F(_)) {
  fmap(fa _F(_A), f (_A) [_B]) _F(_B)
}

Monad(_M(_)) {
  ..Functor(_M)
  pure(a _A)                           _M(_A)
  flat_map(ma _M(_A), f (_A) [_M(_B)]) _M(_B)
}

Some      { value _T }
Option(_T) Some(_T) | None

OptionMonad Monad(Option) {
  fmap(fa, f) [
    fa . {
      Some(val) [ Some(val . f) ]
      None      [ None ]
    }
  ]
  pure(a) [ Some(a) ]
  flat_map(ma, f) [
    ma . {
      Some(val) [ val . f ]
      None      [ None ]
    }
  ]
}

OptionT(_M(_), _A) {
  run _M(Option(_A))
}

MonadTrans(_T(_(_), _)) {
  lift(_M(_) Monad, ma _M(_A)) _T(_M, _A)
}


#
15. HIGHER-ORDER FUNCTIONS, EARLY EXIT (`Break`) & CATAMORPHISMS
#

sum_until_limit(numbers List(Int), max Int) [
  numbers . fold(0, (acc Int, n Int) [
    next (acc + n)
    next > max .? Break(next)
    next
  ])
]

first_adult(users List(User)) [
  users
  . take_while([ .age > 0 ])
  . find([ .age >= 18 ])
]

list_map(l List(_A), action (_A) [_B]) List(_B) [
  l . {
    None         [ None ]
    Cons(hd, tl) [ Cons(hd . action, tl . list_map(action)) ]
  }
]

Num  { value _T }
Add  { left _L, right _R }
Mul  { left _L, right _R }
Expr Num(Int) | Add(Expr, Expr) | Mul(Expr, Expr)

cata(expression Expr, algebra) [
  expression . {
    Num(v)    [ algebra.on_num(v) ]
    Add(l, r) [ algebra.on_add(l . cata(algebra), r . cata(algebra)) ]
    Mul(l, r) [ algebra.on_mul(l . cata(algebra), r . cata(algebra)) ]
  }
]

CalcAlgebra {
  on_num(v Int)        [ v ]
  on_add(a Int, b Int) [ a + b ]
  on_mul(a Int, b Int) [ a * b ]
}


#
16. CONCURRENCY (`.|` PARALLEL PIPE, CSP CHANNELS), ATOMS & STATE MACHINES
#

fetch_profile(id String)         [ Url"https://api.ex/users/{id}" . Http.get ]
fetch_recommendations(id String) [ Url"https://api.ex/recs/{id}" . Http.get ]
fetch_ads()                      [ Url"https://api.ex/ads" . Http.get . timeout(Duration"2s") ]

load_dashboard_data(user_id String) [
  user_id .| {
    profile [ user_id . fetch_profile ]
    recs    [ user_id . fetch_recommendations ]
    ads     [ fetch_ads() ]
  }
]

Channel(_T) {
  send(val _T) [ (channel, val) . native_chan_send ]
  recv()       [ channel . native_chan_recv ]
}

worker_loop(jobs Channel(String), results Channel(Int)) [
  jobs.recv() . {
    None      [ "Channel closed, terminating worker." . Shell.log ]
    Some(job) [
      job . length . results.send
      worker_loop(jobs, results)
    ]
  }
]

CounterState { count Int }
Increment    { by Int }
Decrement    { by Int }
Reset        {}
CounterMsg   Increment | Decrement | Reset

update_counter(state CounterState, msg CounterMsg) [
  msg . {
    Increment [ CounterState { count (state.count + increment.by) } ]
    Decrement [ CounterState { count (state.count - decrement.by) } ]
    Reset     [ CounterState { count 0 } ]
  }
]

Atom {
  value            _T
  read()           [ atom . native_atom_read ]
  swap(transition) [ (atom, transition) . native_atom_swap ]
}

global_counter Atom(CounterState) {
  value CounterState { count 0 }
}

modify_global_counter() [
  global_counter.swap((state CounterState) [ update_counter(state, Increment { by 5 }) ])
]

Disconnected { reason String }
Connecting   { host String, attempt Int }
Connected    { host String, session_id String }
ConnState    Disconnected | Connecting | Connected

ConnectCmd    { host String }
HandshakeOk   { session_id String }
NetworkError  { reason String } & Bypass
DisconnectCmd {}
ConnEvent     ConnectCmd | HandshakeOk | NetworkError | DisconnectCmd

step_connection(state ConnState, event ConnEvent) ConnState [
  (state, event) . {
    (Disconnected, ConnectCmd)  [ Connecting { host connect_cmd.host, attempt 1 } ]
    (Connecting, HandshakeOk)   [ Connected { host connecting.host, session_id handshake_ok.session_id } ]
    (Connecting, NetworkError)  [ Disconnected { reason network_error.reason } ]
    (Connected, DisconnectCmd)  [ Disconnected { reason "User closed" } ]
    _                           [ state ]
  }
]


#
17. RESOURCE MANAGEMENT, SCOPED LIFETIMES (TYPED BYPASS BIND) & UNIFIED PORTS
#

FileHandle { fd Int, path Path }
IoError    { msg String } & Bypass

Fs {
  open(path Path)   [ path . native_fs_open ]
  read(FileHandle)  [ file_handle.fd . native_fs_read ]
  close(FileHandle) [ file_handle.fd . native_fs_close ]

  with_file(path Path, action) [
    fh FileHandle (path . Fs.open)
    fh . action
    .> [ fh . Fs.close ]
  ]
}

read_config(path Path) [
  Fs.with_file(path, (f FileHandle) [
    f . Fs.read . Json.parse
  ])
]

sync_remote_backup() [
  Url"https://api.razomy.org/v1/users"
  . Port.read
  . Port.write(Path"./backup/users.json")
]


#
18. DEEP PATTERN ROUTING (STRUCTURAL & TUPLE DESTRUCTURING)
#

HttpRequest {
  method  'GET' | 'POST' | 'PUT' | 'DELETE'
  path    Tuple(String)
  headers Map(String, String)
  body    String
}

HttpResponse { status Int, body String }
Unauthorized { reason String } & Bypass

auth_middleware(req HttpRequest) [
  req.headers . Map.get("Authorization") . {
    Some(tok) [ req & { auth_user (tok . Db.find_user_by_token) } ]
    None      [ Unauthorized { reason "Missing Authorization Header" } ]
  }
]

api_router(req HttpRequest) [
  req . {
    { method 'GET', path ("health") } [
      HttpResponse { status 200, body "OK" }
    ]

    { method 'GET', path ("users", id) } [
      req . auth_middleware . {
        Unauthorized [ HttpResponse { status 401, body unauthorized.reason } ]
        HttpRequest  [
          json_str (id . Db.find_user . Json.encode)
          HttpResponse { status 200, body json_str }
        ]
      }
    ]

    _ [ HttpResponse { status 404, body "Not Found" } ]
  }
]


#
19. BUILT-IN DIALECTS: PARSE (STREAM GRAMMARS) & VIEW/DRAW (DECLARATIVE UI)
#

Parse {
  end                        'eof'
  match(pat)                 [ .native_parse_match(pat) ]
  one_or_more(rule)          [ .native_parse_one_or_more(rule) ]
  repeat(bounds, rule)       [ .native_parse_repeat(bounds, rule) ]
  skip(count Int)            [ .native_parse_skip(count) ]
  to(delimiter)              [ .native_parse_until(delimiter) ]
  copy(field Symbol, target) [ .native_parse_capture(field, target) ]
  run(input String, rules)   [ input . rules ]
}

digit Range("0", "9")
alpha Range("a", "z") | Range("A", "Z")

EmailGrammar String & [
  .Parse.run(
    Parse.one_or_more(alpha)
    & Parse.match("@")
    & Parse.one_or_more(alpha)
    & Parse.match(".")
    & Parse.repeat(Range(2, 6), alpha)
  )
]

LogEntry { date String, level String, msg String }

parse_log_line(line String) [
  line . Parse.run(
    Parse.copy('date', Parse.to(" "))
    & Parse.skip(1)
    & Parse.match("[")
    & Parse.copy('level', Parse.to("]"))
    & Parse.skip(2)
    & Parse.copy('msg', Parse.to(Parse.end))
  ) . {
    LogEntry [ log_entry ]
  }
]

Ui {
  view(spec)           [ spec . native_ui_view ]
  draw(children Tuple) [ children . native_ui_draw ]
}

counter_app(state Atom(CounterState)) [
  Ui.view({
    title "Razomy Counter"
    size  Pair"1920x1080"
    body  (
      heading("Current Count: {state.read().count}")
      row(
        button("Increment", [ state.swap((s CounterState) [ update_counter(s, Increment { by 1 }) ]) ]),
        button("Reset",     [ state.swap((s CounterState) [ update_counter(s, Reset {}) ]) ])
      )
      canvas(Pair"200x100", [
        Ui.draw(
          pen(Hex"#FF0000"),
          fill_pen(Hex"#00FF00"),
          circle((50, 50), 40)
        )
      ])
    )
  })
]


#
20. COMPILE-TIME METAPROGRAMMING (`Compile &`), DOC-COMMENTS (`Meta.doc_of`), `Bind` REWRITE & SELF-HOSTED `Meta.rewrite`
#

Meta {
  type_of(entity)           [ entity . native_meta_type_of ]            & Compile
  name_of(entity)           [ entity . native_meta_name_of ]            & Compile
  doc_of(entity)            [ entity . native_meta_doc_of ]             & Compile
  fields_of(entity)         [ entity . native_meta_fields_of ]          & Compile
  steps_of(stream)          [ stream . native_meta_steps_of ]           & Compile
  map_fields(form, mapper)  [ (form, mapper) . native_meta_map_fields ] & Compile
  compose(stream)           [ stream . native_meta_eval_parens ]        & Compile
  error(msg String)         [ msg . native_meta_compile_error ]         & Compile
  identity                  [ .native_identity ]

  rewrite(node, rules) [
    walked (node . {
      Form   [ node . Meta.map_fields((k, v) [ (k, v . Meta.rewrite(rules)) ]) ]
      Tuple  [ node . map([ .Meta.rewrite(rules) ]) ]
      Stream [
        node . Meta.steps_of
        . map([ .Meta.rewrite(rules) ])
        . fold(Meta.identity, (acc, step) [ acc & step ])
      ]
      _      [ node ]
    })

    walked . (rules & { _ [ walked ] })
  ] & Compile
}

step_1 [ .+ 10 ]
step_2 [ .* 2 ]
combined_pipeline step_1 & step_2

build_multiplier(factor Int) [
  Meta.compose([ .* (factor * 10) ])
] & Compile

trace_execution(code_block) [
  code_block
  . Meta.steps_of
  . map((step) [
    step & [ .> ((res) [ "Step result: {res}" . Shell.log ]) ]
  ])
  . fold(Meta.identity, (acc, step) [ acc & step ])
] & Compile

derive_json(_TargetForm) [
  _TargetForm & {
    to_json() [
      inner (
        _TargetForm . Meta.fields_of . map((field) [
          "\"{field.name}\": {field.value . Json.format_value}"
        ])
        . join(", ")
      )
      "\{ {inner} \}"
    ]
  }
] & Compile

#
Represents a catalog product with auto-derived JSON serialization.
#
Product (Compile & [
  {
    id    Int    # Unique product identifier
    title String # Display title in catalog
  } . derive_json
])

Resource { kind String "Milk" }
Cat      { status String }
Factory  {
  + (res Resource) [ Cat { status "Fed with {res.kind}" } ]
}

abstract_law [
  a (b + c)
]

specialize_law(code_stream) [
  code_stream . Meta.rewrite({
    'a'       [ cat_instance ]
    'b'       [ Factory {} ]
    'c'       [ Resource { kind "Fish" } ]
    (x + y)   [ x + y ]
    Compiler.Bind(target, val) [
      Compiler.Bind(target, val)
      & [ "Bound {Meta.name_of(target)} to {target}" . Shell.log ]
    ]
  })
] & Compile

run_cat_factory (Compile & [ abstract_law . specialize_law ])


#
21. IMPORTS, PREFIX LINEAGE SPREAD (`..Mod`), FIELD UNPACKING (`{}`) & DI INSTANTIATION
#

# 1. Static Module Alias (`PascalCase` = 0-alloc compile-time prototype binding):
Base   root.base
Http   std.net.http
Sqlite std.db.sqlite
ExtMod Path"../shared/ext_mod.rl"

# 2. Selective Field Unpacking via `{}`:
{ String, Int, Money } root.base
{ get, post }          std.net.http

# 3. Full Context Mixin / Lineage Spread via Symmetric Prefix `..`:
..root.prelude
..Path"../shared/ext_mod.rl"
..(std.net.http / 'legacy_request')

# 4. In-File Submodules (`SubToken { ... }`) vs Lineage Spread (`..Ancestor`) & `/ Private`:
AuthInternal {
  salt "x9f2a" & Private
  hash(raw String)         [ "{raw}_{salt}" . native_sha256 ] & Private
  verify(u User, p String) [ (p . hash) == u.password_hash ]
  SubToken {
    secret_key "k123" & Private
    issue(uid String) [ "jwt_{uid}_{secret_key}" ]
  } / Private
}
AuthModule AuthInternal / Private

# 5. Module Facades / Re-exports:
BillingModule { charge(m Money) [ m ] }
AppSdk        (AuthModule & BillingModule) / Private

# 6. Parameterized Modules (DI) vs Stateful Runtime Instances (`snake_case`):
DbDriver(conn_url String, max_pool Int 10) {
  active_conns Atom(Int) { value 0 } & Private
  query(sql String) [ "Running '{sql}' on {conn_url} (pool={max_pool})" . Shell.log ]
}

primary_db   (DbDriver { conn_url "postgres://prod:5432/app", max_pool 32 })
analytics_db (DbDriver { conn_url "postgres://olap:5432/bi",  max_pool 4  })

run_module_demo(u User) [
  u . AuthModule.verify("pass123")
  "u_1" . AuthModule.SubToken.issue
  "SELECT 1" . primary_db.query
  "SELECT COUNT(*)" . analytics_db.query
]


#
22. TESTING AND PROPERTY-BASED ASSERTIONS
#

TestPassed { name String }
TestFailed { name String, expected Any, actual Any }
TestResult TestPassed | TestFailed

assert_eq(name String, expected, actual) [
  (expected == actual) . {
    true  [ TestPassed { name name } ]
    false [ TestFailed { name name, expected expected, actual actual } ]
  }
]

run_test_suite() [
  migrated_bob (User { id "1", name "Bob", age 20, role 'user', password_hash "x" } . migrate_user)

  (
    assert_eq("Math Addition", 4, (2 + 2)),
    assert_eq("Area of Rect", 20.0, (Rect { width 4.0, height 5.0 } . area)),
    assert_eq("User Migration", true, migrated_bob.is_adult)
  )
  . for_each((result TestResult) [
    result . {
      TestPassed [ "[PASS] {test_passed.name}" . Shell.log ]
      TestFailed [ "[FAIL] {test_failed.name}: Expected {test_failed.expected}, got {test_failed.actual}" . Shell.log ]
    }
  ])
]


#
23. CAPSTONE: STATEFUL RECURSIVE AUTONOMOUS AI AGENT & MAIN ENTRY POINT
#

CompileError { code Int, log String } & Bypass
BuildSuccess { status String, artifact String }
BuildResult  BuildSuccess | CompileError

FatalError CompileError & [ .code >= 500 ]
Warning    CompileError & [ .code < 500 ]

AiQuery    { system_prompt String, user_prompt String }
AiResponse { diff String, explanation String }

AgentState {
  err             CompileError
  retries         Int
  applied_patches Tuple(String) ()
}

Ai {
  call(query AiQuery) AiResponse [ query . native_ai_llm_request ]
}

run_build() BuildResult [ "cargo build --release" . Shell.exec . native_parse_build_result ]

agent_loop(state AgentState) [
  state.retries . {
    0 [ "Retry limit reached. Human intervention required." . Shell.log ]

    _ [
      ai_resp (
        AiQuery {
          system_prompt "You are an engineer. Fix the compilation error with a git diff patch."
          user_prompt   "Error log: {state.err.log}. Write a patch."
        }
        . Ai.call
        .> [ "AI suggests solution (attempt {state.retries}): {.explanation}" . Shell.log ]
        .> [ "git apply {.diff}" . Shell.exec ]
      )

      run_build() . {
        BuildSuccess [
          "Build succeeded after applying patch!" . Shell.log
        ]

        CompileError [
          "Patch did not help ({compile_error.log}). Starting next attempt..." . Shell.log
          next_state (state & {
            err             compile_error
            retries         (state.retries - 1)
            applied_patches (state.applied_patches . append(ai_resp.diff))
          })
          agent_loop(next_state)
        ]
      }
    ]
  }
]

main() [
  "Starting test suite..." . Shell.log
  run_test_suite()

  "Starting project build process..." . Shell.log
  run_build() . {
    FatalError [
      "Critical compilation failure ({fatal_error.code}). Launching agent..." . Shell.log
      agent_loop(AgentState { err fatal_error, retries 3 })
    ]

    Warning [
      "Compiler warnings: {warning.log}" . Shell.log
      "cargo fix --allow-no-vcs" . Shell.exec
      "Automatic linting completed." . Shell.log
    ]

    BuildSuccess [
      "Project is completely clean ({build_success.artifact}). Ready for release!" . Shell.log
    ]
  }
]


#
24. APPENDIX A: SELF-REWRITING DISTRIBUTED ONTOLOGY & HOMOICONIC GRAPH OPTIMIZATION (`Compile &`)
#

NodeId    String & [ .length > 0 ]
Timestamp Int    & [ .> 1700000000 ]

Entity {
  id         NodeId
  version    Int 1
  created_at Timestamp
}

Replicated {
  ..Entity
  region     Symbol 'eu_central'
  state      'synced' | 'drifted' | 'offline'
  secret_key Hex & Private
  sync()     [ "Replicating {id} to {region} with {secret_key}" ]
}

Audited {
  ..Entity
  auditor    String "system_daemon"
  state      'verified' | 'flagged'
  signature  Hex & Private
  sync()     [ "Auditing {id} by {auditor}" ]
}

LedgerNode Replicated & Audited & {
  payload    Json
  sync_all() [
    (
      ledger_node.Replicated.sync()
      ledger_node.Audited.sync()
    )
  ]
  is_healthy [
    (ledger_node.Replicated.state == 'synced')
    && (ledger_node.Audited.state == 'verified')
  ]
}

ConsensusFault { node_id NodeId, reason String } & Bypass
QuorumLost     { active_nodes Int, required Int } & Bypass
StorageCorrupt { sector Int }                     & Bypass
EngineFailure  ConsensusFault | QuorumLost | StorageCorrupt

Cluster {
  min_quorum Int 3
  nodes      Tuple(LedgerNode)

  verify_and_commit(tx_batch Tuple(Json)) [
    healthy_nodes (nodes .? .is_healthy)

    healthy_nodes.length >= min_quorum .! QuorumLost {
      active_nodes healthy_nodes.length
      required     min_quorum
    }

    healthy_nodes . map((LedgerNode) [
      committed_txs (
        tx_batch . map([
          {
            target_node   ledger_node.Entity.id
            node_region   ledger_node.region
            quorum_target cluster.min_quorum
            tx_data       .
          }
        ])
      )

      (ledger_node / Audited.state) & {
        version (ledger_node.Entity.version + 1)
        payload committed_txs
      }
    ])
  ]
}

ServerKernel {
  NodeModel LedgerNode
  save_node(LedgerNode) [
    ledger_node.Replicated.state == 'synced' .! ConsensusFault {
      node_id ledger_node.Entity.id
      reason  "Node out of sync"
    }
    ledger_node . Storage.write
  ]
}

derive_edge_client(_KernelForm) [
  _KernelForm . Meta.rewrite({
    LedgerNode          [ (LedgerNode / Private) / Audited ]
    (.Replicated.state) [ .state ]
    (Storage.write)     [ RpcClient.send & [ .timeout(Duration"3s") ] ]
  })
] & Compile

EdgeClientKernel (Compile & [ ServerKernel . derive_edge_client ])

Cuboid {
  size     (1.0, 1.0, 1.0)
  anchor   (0.5, 0.5, 0.5)
  position (0.0, 0.0, 0.0)
  pivot    (0.5, 0.5, 0.5)
  rotation (0.0, 0.0, 0.0)
  scale    (1.0, 1.0, 1.0)
}
PlayButton {
  ..Cuboid
  label "Play"
}

FlatPlayButton (PlayButton / Cuboid) & {
  canvas_2d_rect (0.5, 0.35, 0.15, 0.15)
}

RoundButton {
  ..Cuboid
  name     String
  children {}
}

render_for_mobile(screen_graph) [
  screen_graph . Meta.rewrite({
    (RoundButton & {
      name     btn_name
      size     btn_size
      position btn_pos
      children { icon { icon_name ico } }
    }) [
      NativeMobileButton {
        id    btn_name
        rect  (btn_pos, btn_size)
        asset ico
      }
    ]
  })
] & Compile


#
25. APPENDIX B: COMPILER ERROR CATALOG (`E01`–`E10`) & OPPOSITIONAL EDGE CASES
#

# 1. E01_AMBIGUOUS_CALL_OR_BIND — Spaced binding on an existing function name in scope
edge_case_1_demo() [
  # ❌ ERROR `E01_AMBIGUOUS_CALL_OR_BIND`: `area` is an existing function; space before `(` looks like rebinding `area`
  # area (Circle { radius 5.0 })

  # ✅ SOLUTION: Use tight parens `area(...)` or `.` pipeline to call, or bind to a distinct variable name
  res1        area(Circle { radius 5.0 })
  res2        (Circle { radius 5.0 } . area)
  circle_area (Circle { radius 5.0 } . area)
]

# 2. E02_DUPLICATE_UNNAMED_TYPE — Multiple identical `PascalCase` types without explicit `snake_case` names
Account { iban String, balance Money }

# ❌ ERROR `E02_DUPLICATE_UNNAMED_TYPE`: `Account` appears twice without disambiguating parameter names
# transfer_bad(Account, Account, Money) [ ... ]

# ✅ SOLUTION: Name duplicate `Account` slots explicitly (`source`, `target`); single `Money` auto-binds `money` (Law 1c)
transfer_safe(source Account, target Account, Money) [
  source.balance .= (source.balance - money)
  target.balance .= (target.balance + money)
  (source, target)
]

# 3. E03_UNKNOWN_CONCRETE_TYPE — Unprefixed generic type variable (`Elem` instead of open slot or `_Elem`)
# ❌ ERROR `E03_UNKNOWN_CONCRETE_TYPE`: `Elem` does not start with `_` and is not a defined Type in scope
# Wrapper(Elem) { item Elem }

# ✅ SOLUTION: Use an auto-generic open slot `item` or a `_`-prefixed Named Type Hole `_Elem`
Wrapper      { item }
TypedWrapper { item _Elem }
UserWrapper(_U User, current_user _U) {
  active _U current_user
}

# 4. E04_UNHANDLED_UNION_BRANCH — Unrouted domain branch not marked with `& Bypass`
fetch_actor(id String) User | Company | NotFound [
  User { id id, name "Alice", age 30, role 'admin', password_hash "hash" }
]

send_welcome_email(User) [ "Sent email to {user.name}" ]

handle_actor_pipeline(id String) [
  # ❌ ERROR `E04_UNHANDLED_UNION_BRANCH`: `Company` is a domain type (not `& Bypass`) and cannot be skipped silently
  # id . fetch_actor . send_welcome_email

  # ✅ SOLUTION: Route both `User` and `Company` explicitly (`NotFound` auto-bypasses via Law 3)
  id . fetch_actor . {
    User    [ user . send_welcome_email ]
    Company [ "Sent corporate onboarding to {company.name}" ]
  }
]

# 5. E05_AMBIGUOUS_LINEAGE_FIELD — Unqualified access to colliding field on merged lineage
resolve_diamond_demo(entity User & Cloud) [
  # ❌ ERROR `E05_AMBIGUOUS_LINEAGE_FIELD`: both `User` and `Cloud` define `name`
  # entity.name . Shell.log

  # ✅ SOLUTION: Qualify via ancestor path `entity.User.name` or subtract one colliding field via `/`
  "User: {entity.User.name}, Cloud: {entity.Cloud.name}" . Shell.log
  clean_entity (entity / Cloud.name)
  clean_entity.name . Shell.log
]

# 6. E06_IMMUTABLE_TYPE_BINDING — Mutating an immutable `PascalCase` Form Prototype
immutable_demo() [
  # ❌ ERROR `E06_IMMUTABLE_TYPE_BINDING`: `Car` is a `PascalCase` Type Prototype and cannot be mutated in-place
  # Car.max_speed .= 500

  # ✅ SOLUTION: Instantiate `Car` into a `snake_case` variable first, then mutate via `.=`
  my_car (Car {})
  my_car.max_speed .= 500
]

# 7. E07_STATIC_ASSERT_FAILED — Compile-time invariant check via `Compile &` and `Meta.error`
VerifyPublicUserSafety (Compile & [
  fields (PublicUser . Meta.fields_of . map(.name))
  (fields . contains("password_hash")) . {
    true  [ Meta.error("SECURITY VIOLATION: PublicUser leaked password_hash!") ]
    false [ true ]
  }
])

# 8. E08_AMBIGUOUS_ROUTER_KEY — Bare `snake_case` identifier in Router key position
router_key_demo(x Int, target_val Int) [
  # ❌ ERROR `E08_AMBIGUOUS_ROUTER_KEY`: bare `target_val [ "Matched" ]` looks like a Form field declaration
  # x . { target_val [ "Matched" ], _ [ "No" ] }

  # ✅ SOLUTION: Use relative operator `.== target_val` or parens `(target_val)`
  x . {
    .== target_val [ "Matched" ]
    _              [ "No" ]
  }
]

# 9. E09_DANGLING_BIND_NEWLINE — Newline between target key and opening bracket `[` / `{` / `(`
newline_bind_demo(amount Int) [
  # ❌ ERROR `E09_DANGLING_BIND_NEWLINE`: `\n` before `[` splits `discount` and `[ amount - 10 ]` into two elements
  # discount
  # [ amount - 10 ]

  # ✅ SOLUTION: Keep the opening bracket `[` on the same line as `discount`
  discount [ amount - 10 ]
  discount()
]

# 10. E10_MULTIPLE_REST_PATTERNS — Multiple `..rest` captures on the left side of a destructuring bind
rest_pattern_demo(nums (Int, Int, Int, Int)) [
  # ❌ ERROR `E10_MULTIPLE_REST_PATTERNS`: cannot split a sequence into two ambiguous rest captures on the left
  # (first, ..r1, ..r2) nums

  # ✅ SOLUTION: Use at most one `..rest` capture on the left (multiple `..spreads` on the right are valid)
  (first, ..rest) nums
  combined (first, ..rest, ..rest)
  combined
]


#
26. ОНТОЛОГИЯ, ГРАММАТИКА И ЭВОЛЮЦИЯ ТИПОВ RAZOMY v6.4 (САМООПИСАНИЕ НА БАЗЕ ЯЗЫКА)
Единая фрактальная и симметричная аксиоматика:
1. Фрактальность: Папка == Файл == Модуль == Класс == Роутер == Аргументы == Узел AST == `{}`.
2. Каноническая триада: `Name(Слоты) {Форма} [Поток]` (где `()` — позиционная проекция `{}`).
3. Явное встраивание предков через префиксный спред `..Ancestor` внутри `{}` (`..Vehicle`, `..Functor(_M)`)
   или пересечение `Vehicle & HasBattery & { ... }`. Вложенное имя без `..` (`SubToken { ... }`) — подмодуль.
4. Разрешение скоупов и авто-дженериков:
   • Поля Формы, обогащаемого объекта `ctx & { ... }` и аргументы видны напрямую в лексическом скоупе (Закон 1a).
   • Ведущая точка (`.field`, `.0`, `.+ 1` — Закон 1b) обращается ко входу самого внутреннего `[...]`.
   • Конкретный Тип `PascalCase` проецирует `snake_case` имя (`User` -> `user`, `Cluster` -> `cluster` — Закон 1c).
   • Символ `_` — безымянная дырка (`Any`), а `_PascalCase` (`_T`, `_A`, `_M`) — именованная типовая дырка (Generic Type Hole).
5. Симметричные шлюзы и чистые бинарные обновления:
   • `val .? Pred` (фильтр/проверка паттерна или `Range`) и `cond .? BypassVal` (выход при `true`) vs `cond .! ErrVal` (выход при `false`).
   • Пробел `a b` — чистое объявление, а `a .= val`, `a .+ 1`, `a .& {...}` — явное обновление на месте.
#

# ============================================================================
# СЛОЙ 0. ЯДРО КОМПИЛЯТОРА, ГРАММАТИКА И СВЯЗЫВАНИЯ (`Compiler`)
# ============================================================================
Compiler {
  # 1. Первичный корень AST и Токены
  Node          {}
  Token         Node
  Sep           Token                           # `,` или `\n`
  Literal       Token                           # Константы (`0`, `0.0`, `'sym'`)
  Identifier    Token                           # Именованный токен
  Wildcard      Token                           # `_` — безымянная дырка (`Any`)

  # Разделение по Case Rule:
  ValueKey      Identifier                      # `snake_case`: переменные, поля, функции
  TypeKey       Identifier                      # `PascalCase`: конкретные типы, формы, модули
  TypeHole      Identifier                      # `_PascalCase` (`_T`, `_A`, `_M`): именованная дженерик-дырка типа

  # 2. Три Фундаментальные Скобки (The Three Brackets)
  Bracket       Node
  TextQuote     Bracket                         # `"..."`, `r"..."`, `Type"..."`
  CommentBlock  Bracket                         # `# ...` или `#\n ... \n#` (`Meta.doc_of`)

  TupleBracket  Bracket                         # `()` — Позиционный кортеж, срез `Tuple(_T)`, группировка `(a + b)`
  FormBracket   Bracket                         # `{}` — Именованная форма: классы, аргументы, модули, роутеры
  StreamBracket Bracket                         # `[]` — Поток выполнения: ленивые вычисления, циклы, лямбды

  # 3. Грамматическая композиция
  Repeat(_T)    Node
  Expr          Literal | Identifier | Wildcard | Tuple | Form | Stream
  Pair          (Identifier | Tuple) & Expr
  LineageSpread ExpandSpread & (TypeKey | TypeHole | ((TypeKey | TypeHole) & Tuple))

  Tuple         TupleBracket  & Repeat(Expr)
  Form          FormBracket   & Repeat(Pair | LineageSpread)
  Stream        StreamBracket & Repeat(Expr)

  # 4. Дуальность Формы и Кортежа в Функциях и Классах:
  Slots         Form | Tuple
  Class         TypeKey  & Slots
  Callable      Slots    & Stream
  Function      ValueKey & Slots & Stream

  # 5. Матрица связываний (Пробел `' '` — узел `Bind(Target, Source)`)
  Bind(_Target, _Source) Node
  TypeAliasBind Bind(TypeKey, TypeKey)
  ProtoFormBind Bind(TypeKey, Form)
  FieldSpecBind Bind(ValueKey, (TypeKey | TypeHole) & Expr)
  LazyFieldBind Bind(ValueKey, Stream)
  VarBind       Bind(ValueKey, Expr)
  TypedVarBind  Bind(ValueKey & TypeKey, Expr)
  RestUnpack    Bind(Tuple | Form, Expr)

  # 6. Операторы (Унарные — префиксные, Бинарные — через пробел)
  Operator       Token
  UnaryOperator  Operator
  BinaryOperator Operator

  Not            UnaryOperator                  # `!x`     — логическое отрицание или негативный шлюз типа (`Order & !Cancelled`)
  Neg            UnaryOperator                  # `-x`     — арифметическое отрицание (`-5`)
  RelGet         UnaryOperator                  # `.field` — относительный геттер/метод самого внутреннего `[...]`
  RelOp          UnaryOperator                  # `.+ x`   — относительный шаг потока (`.+ 1`, `.>= 18`, `.== 0`)
  ExpandSpread   UnaryOperator                  # `..x`    — слева собирает остаток `(a, ..rest)`, справа встраивает `{ ..Vehicle }` / `(0, ..x)`

  # 6b. Явные операторы обновления на месте (Perceus O(1) In-Place Mutation)
  MutateOperator BinaryOperator
  AssignMut      MutateOperator                 # `a .= b`
  AddMut         MutateOperator                 # `a .+ b`
  SubMut         MutateOperator                 # `a .- b`
  MulMut         MutateOperator                 # `a .* b`
  EnrichMut      MutateOperator                 # `a .& b`

  # 6c. Потоковые операторы (Pipelines)
  PipeOperator   BinaryOperator
  TightDot       PipeOperator                   # `a.b`    — плотный доступ к полю, предку `a.Vehicle.id`, индексу `a.0`, вызову `a.fn()`
  Pipe           PipeOperator                   # `a . b`  — передача данных `b(a)`
  TapPipe        PipeOperator                   # `a .> b` — сайд-эффект `b(a)`, возвращает исходный `a`
  EnrichPipe     PipeOperator                   # `a .& b` — вызывает `b(a)` и сливает через `&` с `a`
  SubtractPipe   PipeOperator                   # `a ./ b` — удаляет поле/ключ/тег/предка `b` из `a` в потоке (`a / b`)
  PositivePipe   PipeOperator                   # `a .? b` — проверка/фильтр (`val .? Range(1, 10)`) или выход при `true` (`cond .? Break(v)`)
  NegativePipe   PipeOperator                   # `a .! b` — шлюз/assert: продолжает при `true`, выбрасывает `b` (`Bypass`) при `false`
  ParallelPipe   PipeOperator                   # `a .| {}`— параллельный fan-out веток в `{}`

  # 6d. Алгебра, Математика, Сравнение и Логика
  AlgebraOp      BinaryOperator
  And            AlgebraOp                      # `a & b`  — пересечение / слияние с сохранением Lineage
  Or             AlgebraOp                      # `a | b`  — объединение (Union)
  Sub            AlgebraOp                      # `a / b`  — структурное вычитание поля `'key'`, пути `A.id`, тега `Private` или предка `Vehicle`

  MathOp         BinaryOperator
  Plus           MathOp                         # `a + b`
  Minus          MathOp                         # `a - b`
  Mult           MathOp                         # `a * b`
  Div            MathOp                         # `a / b` (на `snake_case` значениях вызывает метод `/`)
  Mod            MathOp                         # `a % b`
  Pow            MathOp                         # `a ** b`
  ShiftL         MathOp                         # `a << b`
  ShiftR         MathOp                         # `a >> b`

  CompareOp      BinaryOperator
  Eq             CompareOp                      # `a == b` (структурное равенство)
  Neq            CompareOp                      # `a != b`
  Gt             CompareOp                      # `a > b`
  Gte            CompareOp                      # `a >= b`
  Lt             CompareOp                      # `a < b`
  Lte            CompareOp                      # `a <= b`

  LogicOp        BinaryOperator
  LogicAnd       LogicOp                        # `a && b`
  LogicOr        LogicOp                        # `a || b`

  # 7. Директивы управления и компиляции
  Directive           TypeKey
  Compile             Directive
  Bypass              Directive
  Loop(body Stream)   Directive
  Effect(body Stream) Directive
  Break               Directive & Bypass & { value _T, label Symbol 'default' }
  Return              Directive & Bypass & { value _T }
  Public              Directive
  Private             Directive
}


# ============================================================================
# СЛОЙ 1. АППАРАТНЫЕ РЕГИСТРЫ И ПРИМИТИВЫ (`Primitives`)
# ============================================================================
Primitives {
  ..Compiler

  Any        TypeKey
  None       TypeKey & Bypass
  Never      !Any

  BuiltIn    TypeKey
  Gpr(_B)    BuiltIn & { bytes _B, reg 'general' }
  Xmm(_B)    BuiltIn & { bytes _B, reg 'vector'  }
  Mem(_B)    BuiltIn & { bytes _B, align 8       }

  Bool       Gpr(1)
  Byte       Gpr(1)
  Int        Gpr(8)
  Float      Xmm(8)
  Ptr(_T)    Gpr(8) & { target _T }
  Symbol     Gpr(8)
  String     Mem(16) & { ptr Ptr(Byte), len Int }

  Scalar     Bool | Byte | Int | Float | Ptr(Any) | Symbol
  Primitive  Scalar | String
}


# ============================================================================
# СЛОЙ 2. ПРИКЛАДНЫЕ ДОМЕННЫЕ ЛИТЕРАЛЫ (`DomainTypes`)
# ============================================================================
DomainTypes {
  ..Primitives

  Range                    Form & { start _T, end _T }
  TaggedString(raw String) Form & { value raw }

  PortLiteral TaggedString
  Url         PortLiteral                       # Url"https://razomy.org"
  Path        PortLiteral                       # Path"./data/users.json"
  Ip          TaggedString                      # Ip"192.168.0.1:8080"

  Temporal    TaggedString
  Date        Temporal                          # Date"2026-03-30"
  Time        Temporal                          # Time"14:30:00"
  Datetime    Temporal                          # Datetime"2026-03-30T14:30:00Z"
  Duration    Temporal                          # Duration"5m30s"

  Hex         TaggedString                      # Hex"#2A9D8F"
  Pair        TaggedString                      # Pair"1920x1080"
  Version     TaggedString                      # Version"1.4.2-beta"

  Money(raw String) TaggedString(raw) & { scaled Int }
}


# ============================================================================
# СЛОЙ 3. КОЛЛЕКЦИИ, АЛГЕБРАИЧЕСКИЕ КОНТЕЙНЕРЫ И ВЫСШИЕ РОДЫ (`Collections`)
# ============================================================================
Collections {
  ..DomainTypes

  Some          Form & { value _T }
  Option(_T)    Some(_T) | None

  Map           Form & { (_K) _V }
  Set(_T)       Map(_T, Bool)

  Cons          Form & { head _T, tail List(_T) }
  List(_T)      None | Cons(_T)

  RoseTree      Form & { value _T, children Tuple(RoseTree(_T)) }
  Json          String | Int | Float | Bool | None | Tuple(Json) | Map(String, Json)

  Comparable(_Other) Form & {
    < (other _Other) Bool
  }

  Functor(_F(_)) Form & {
    fmap(fa _F(_A), f (_A) [_B]) _F(_B)
  }

  Monad(_M(_)) {
    ..Functor(_M)
    pure(a _A)                           _M(_A)
    flat_map(ma _M(_A), f (_A) [_M(_B)]) _M(_B)
  }

  MonadTrans(_T(_(_), _)) Form & {
    lift(_M(_) Monad, ma _M(_A)) _T(_M, _A)
  }
}


# ============================================================================
# СЛОЙ 4. РАНТАЙМ, СОСТОЯНИЕ, ОШИБКИ И МЕТАПРОГРАММИРОВАНИЕ (`Runtime`)
# ============================================================================
Runtime {
  ..Collections

  Error            Form & Bypass
  Result(_T, _E)   _T | (Error & _E)

  NotFound         Error & { entity String, id String }
  IoError          Error & { msg String }
  NetworkError     Error & { reason String }
  Unauthorized     Error & { reason String }
  CompileError     Error & { code Int, log String }

  Unique           Form & { value _T }          # Perceus RC == 1 (O(1) апдейт через `.= val`, `.+ 1`)
  Shared           Form & { value _T }          # Perceus RC > 1 (Copy-on-Write)
  Atom             Form & { value _T }          # Транзакционная CAS-ячейка (`read`, `swap`)
  Channel(_T)      Form                         # CSP-канал передачи сообщений (`send`, `recv`)
  Signal           Form & { value _T }          # Реактивный сигнал (`sig()`, `sig(new_val)`, `sig .+ 1`)

  Parse            Form                         # Потоковые PEG-грамматики (`match`, `copy`, `to`, `run`)
  Ui               Form                         # Декларативный UI (`view`, `draw`)
  Meta             Form & Compile               # Рефлексия и перезапись AST (`type_of`, `doc_of`, `fields_of`, `error`, `rewrite`)
}
