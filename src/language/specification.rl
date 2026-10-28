// =============================================================================
// RAZOMY LANGUAGE V5.5: HOMOICONIC AXIOMATICS OF THREE BRACKETS (`.rl`)
// =============================================================================
// 1. STRICT SEPARATION OF THREE BRACKETS (NO ROLE OVERLAP):
// {} — Form   (Key-Value: Data structures, Classes, Trait impls, Routers, Modules, Maps).
//             Parens on a key position `{ (K) V }` define a parameterized/computed Map key.
//             All Forms `{}` and Tuples `()` have structural equality `==` and hashing by default.
// [] — Stream (Executable body: Lazy sequence of operations, Predicates, Lambdas, Dialects).
//             Every `[...]` is a Stream with an active input context (`_`, or depth-indexed `_0`, `_1`, `_2`).
// () — Tuple  (Ordered array of data, Tuples, Argument slots, Eager value evaluation in `{}`).
//             • Canonical Triad `Name(Slots) {Form} [Stream]` & Universal Slot Resolution Rule:
//               - `Name { ... }`                     -> Form / Struct / Module (`PascalCase`, immutable!)
//               - `Name(T, U) { ... }`               -> 1st-Order Generic Form (unbound `PascalCase` `T`, `U` are Type Params `*`)
//               - `Name(F(_)) { ... }`               -> 2nd-Order Generic Form / HKT (`F(_)` is Type Constructor `* -> *`)
//               - `Name(T(_(_), _)) { ... }`         -> 3rd-Order Generic Form (`T` has Kind `(* -> *) -> * -> *`)
//               - `Name(TC Category, cat TC) { ... }`-> Bounded Generic Form (`PascalCase` `TC Category` constrains Type `TC <: Category`;
//                                                       `snake_case` `cat TC` binds runtime value parameter `cat` of type `TC`)
//               - `Name(Age) { new_age age }`        -> Auto-Unpacked Class (known Type `Age` in scope auto-binds `age Age` via Law 1c!)
//               - `Name(User, User) { ... }`         -> Duplicate known Type Slots auto-bind indexed `user1`, `user2`!
//               - `Name(a Type, b Int) { ... }`      -> Explicit Class (`snake_case` value slots are also Form fields on `self`!)
//               - `Name(T, a T) { ... }`             -> Generic Class (`PascalCase` Type Param `T` + `snake_case` Value Param `a`)
//               - `name(Age) [ age == 18 ]`          -> Function with auto-bound `age` from known `Age` Type (Law 1c)
//               - `name(A, B, x A) B [Body]`         -> Generic function: pure Type slots (`A`, `B`) are compile-time and auto-inferred;
//                                                       runtime UFCS calls `val . name` bind directly to the 1st value slot (`x`)!
//               - `name(args) ReturnType [Body]`     -> Named function with explicit return type (`snake_case`)
//               - `name(args) [Body]`                -> Named function (with optional defaults `name(k 10)`)
//               - `name() [Body]`                    -> Parameterless function (explicit `()` required!)
//               - `(args) [Body]` or `[Body]`        -> Anonymous lambda `(args) [Body]` or pipe stream `[ _ + 1 ]`
//               - `name(args) ReturnType`            -> Method contract signature inside a Form `{}`
//               - `cb (InType) [OutType]`            -> Function Type signature in parameter or field slots
//             • Tuple-Argument Duality & Auto-Lifting:
//               An incoming argument list `(a, b, c)` IS a Tuple `()`. A function expecting `Tuple`
//               accepts `fn(a, b, c)` without double parens `(())`, and a slot expecting `Tuple(T)`
//               automatically lifts a single element `(x)` without requiring a trailing comma!
//
// 2. CASE RULE, IMMUTABILITY OF TYPES, TAGGED STRINGS & SPACE-DELIMITED LEXER:
// • PascalCase  — Types, Forms, Classes, Modules, Union Variants, Type Params (`User`, `Age`, `TC`).
//                 RULE: `PascalCase` bindings are IMMUTABLE Type/Form definitions (`Product { id Int }`).
//                 Writing `ExistingForm { id 3 }` inside an expression instantiates `ExistingForm`.
// • snake_case  — Variables, Field keys, Functions, Methods, Value Params (`user_id`, `find_user`).
//                 RULE: Only `snake_case` bindings inside `[]` can be rebound/mutated in-place!
// • 'symbol'    — Paired Symbol Literal: exact constant tag without interpolation (`'draft'`, `'GET'`).
// • "..."       — Standard String (multiline by default): evaluates `{expr}` inside ("Hello, {name}")
//                 and supports `\` escapes (`\n`, `\t`, `\\`, `\"`, `\{`).
// • r"..."      — Raw String (multiline by default, also numbered `r1"..."1`, `r2"..."2`):
//                 zero `\` escapes and zero `{}` interpolation; never uses `#` so comments never collide!
// • Type"..."   — Type-Prefixed String Sugar: ANY `Type"..."` (`Money"199.99"`, `Path"./a.json"`,
//                 `Url"https://ex.com"`, `Date"2026-03-30"`, `Hex"#2A9D8F"`, `Pair"1920x1080"`) is
//                 100% syntax sugar for the constructor call `Type("...")` (`Money("199.99")`, etc.)!
// • ',' & '\n'  — Universal Element Separator: Inline separator (`,`) and Line separator (`\n`) are 100%
//                 interchangeable across ALL three brackets: fields in `{}`, steps in `[]`, and items in `()`.
// • '#' & '//'  — Comments & Homoiconic Documentation: `//` is a single-line comment; `# ... #` (when `#`
//                 is followed by whitespace/newline) is a multiline block comment. Any comment placed
//                 immediately before or inline after a declaration is preserved in the AST as `Meta.doc_of`!
// • ' ' (Space) — Pair & Token Separator:
//                 (a) Universal Binding Axiom (`key value`, deep in-place path/index update `a.b.0.c val`,
//                     or Tuple destructuring `(a, b) source`): binds a field in `{}`, a module member at root,
//                     or a local variable / Perceus O(1) in-place mutation inside `[]`.
//                     (There is NO `=` assignment operator in Razomy!).
//                 (b) Tight Call `fn(x)` vs Spaced Binding `x (expr)`:
//                     • `name(args)` (NO space before `(`) is a function call or type constructor application.
//                     • `name (expr)` (WITH space before `(`) binds or rebinds variable/field `name` to `(expr)`.
//                 (c) Space-Delimited Lexer Rule: All binary operators (`+`, `-`, `*`, `/`, `%`, `**`,
//                     `<<`, `>>`, `==`, `!=`, `>`, `<`, `>=`, `<=`, `&&`, `||`, `.`, `.>`, `.&`, `.?`,
//                     `.|`, `&`, `|`) MUST be surrounded by spaces (`a - b`, `a / b`, `a . b`), while
//                     unary `!` prefixes tightly (`!x`).
//
// 3. COMPLETE OPERATOR & ALGEBRAIC OVERLOADING REFERENCE:
// 'a.b'     (no spaces)   — Tight Path Access: field, module, ancestor `a.Parent.field`, or index `a.0`, `a.-1`
// 'a . b'   (with spaces) — Dataflow Pipeline: passes `a` left-to-right (`a . b` is sugar for `b(a)`,
//                           and `a . b(c, d)` is UFCS sugar for `b(a, c, d)`)
// 'a .> b'                — Tap Pipeline: executes side-effect `b(a)` and passes original `a` further
// 'a .& b'                — Enrich Pipeline: calls `b(a)` and merges `&` result with left context `a`
//                           (distributes over Unions: `A .& (B | Err)` produces `(A & B) | Err` via Law 3!)
// 'a .? b'                — Safe/Filter Pipe: passes `a` further only if predicate/type `b` matches, else `None`
// 'a .| {}'               — Parallel Pipeline: concurrent fan-out execution of branches in `{}`, merged via `&`
// 'a & b'                 — Universal Algebraic Composition / Intersection (6 Canonical Combinations):
//                           1. Form & Form       -> Inheritance, Trait `impl`, or Structural Merge with Lineage
//                           2. Type & Stream     -> Refinement Type (type constrained by predicate `[...]`)
//                           3. Function & Form   -> Partial application / Default & DI override (`fn & { db MockDb }`)
//                           4. Stream & Stream   -> Sequential pipeline composition (`[ a . b ]`)
//                           5. Value & TagForm   -> Metadata / visibility attachment (`val & Private`)
//                           6. TypeA & TypeB(A)  -> F-Bounded Trait constraint (`User & Comparable(User)`)
// 'a | b'                 — Union: algebraic sum of types, patterns, or outcomes
// 'a / b'                 — Relative Subtraction (for Types/Forms/Maps) or Overloaded Method `/` (for Values):
//                           • Form / FieldPath   -> Removes specific field (`User / User.password_hash`)
//                           • Form / 'key'       -> Removes field key from all ancestors (`Combined / 'name'`)
//                           • Form / Ancestor    -> If `Ancestor` is in `Form` lineage, removes its slice (`UserAndCloud / Cloud`)
//                           • Form / TagOrState  -> If `TagOrState` is NOT in `Form` lineage, acts as a Negative Type
//                                                   Constraint / Typestate Guard `Form & ![ _ . is(TagOrState) ]` (`Order / Cancelled`)
//                           • Map  / key_val     -> Removes entry key from a Map (`config_map / "host"`)
//                           • Union / Type       -> Removes branch from Union (`Result / Error`)
//                           • ValueA / ValueB    -> Invokes overloaded `/` method (`num1 / num2`, `shape1 / shape2`)
// 'a..b'                  — Range: sequence generator (`1..10`, `"a".."z"`)
// 'x == Pattern'          — Equality or Pattern Inclusion: when `Pattern` is a Range `a..b` or Union `A | B`
//                           (`val == (1..100)`, `role == ('admin' | 'manager')`), `==` is infix sugar for `x . matches(Pattern)`!
// '_' & '_0, _1, _2'      — Active Stream Subject & Depth-Indexed Subjects:
//                           • Inside any `[...]`, `_` is ALWAYS the input of the innermost enclosing `[...]`.
//                           • When streams are nested `fn(a) [ a . [ ... ] ]`, `_0` is the root function stream's
//                             input tuple, `_1` is the 1st nested `[...]` input (so inside the 1st nested `[...]`,
//                             `_1` and `_` are identical!), `_2` is the 2nd nested `[...]` input, etc.
//                           • In router keys `{ _ [...] }`, Type slots `F(_)`, or imports `_ mod`, `_` is wildcard/hole.
// 'self'                  — Current Form Instance: references the enclosing Form `{}` inside its methods/formulas
// 'Self'                  — Current Form Type: references the enclosing Form Type in contracts and recursive types
// '{k (e)}' vs '{k [e]}'  — Eager vs Lazy/Reactive Field: `k (e)` evaluates `(e)` once on creation; `k [e]` is a
//                           zero-cost inline getter (or reactive computed node when reading a `Signal`)
// 'x . { K [V] }'         — Universal Router Axiom: every branch key `K` in a router `{ K [V] }` is unified sugar for
//                           a predicate check `[ _ . matches(K) ]`. By Law 3, if `x` is a Union `A | B` and a partial
//                           router only handles `{ A [Out] }`, unhandled branch `B` automatically bypasses into `Out | B`!
//                           • Value key `0` or `(0, 0)`        -> sugar for `[ _ == (0, 0) ]`
//                           • Type key `Circle`                -> sugar for `[ _ . is(Circle) ]` (binds `circle` via Law 1c)
//                           • Tuple of Types `(Disconnected, ConnectCmd)` -> matches element types & binds `disconnected`, `connect_cmd`
//                           • Structural pattern `{ path ("users", id) }` -> matches fields and extracts `id`
//                           • Predicate stream `[ _ > 10 ]`    -> executes boolean stream directly
//
// 4. FOUR IRON LAWS OF RAZOMY & COMPILER GUARANTEES:
// - LAW 1 (TRANSPARENT SCOPING & DUAL SLOT PROJECTION — NO HIDDEN FIELD MAGIC):
//   Variables enter `{ form }` or `[ stream ]` in 4 explicit, predictable ways:
//   (a) `self.field` — inside Form definitions `{}`, methods and reactive formulas access sibling fields via `self`.
//                      Value parameter slots in `Form(a Int, b Int) { ... }` are automatically fields on `self`!
//   (b) `_` & `_0, _1` — inside any stream `[...]`, `_` is the input of the nearest `[...]`, while `_0`, `_1`, `_2`
//                      index enclosing streams from the function root (`_0`) down to nested pipe streams (`_1`, `_2`).
//   (c) `Name(KnownType)` / `Name(KnownType, KnownType)` — Dual Slot Projection: when a single `PascalCase` identifier
//                      in `()` refers to an EXISTING Type in scope, it acts as shorthand for `(known_type KnownType)`:
//                      • Single occurrence `(Age)` -> binds `age` of type `Age`.
//                      • Multiple occurrences of the same Type `(User, User)` -> auto-binds indexed `user1`, `user2`!
//                      • Conversely, an unbound `PascalCase` identifier `Box(T)` or bounded `PascalCase` slot
//                        `Name(TC Category)` declares a compile-time Type Parameter (`T` or `TC <: Category`).
//   (d) `fn(u User)` or `fn({ name } User)` — explicit parameter naming (`u.name`) or explicit field destructuring
//                      (`{ name }`) when custom variable names or direct fields are desired in scope.
// - LAW 2 (LINEAGE NAMESPACING & DEAD LINEAGE ELIMINATION): Upon name collision during merge `A & B`, unique
//   fields are accessed directly (`ctx.age`), while colliding fields require the ancestor path (`ctx.A.id`, `ctx.B.id`)
//   until resolved via `/`. Unreferenced ancestor fields are eliminated at compile time (zero runtime RAM overhead).
// - LAW 3 (SUBTYPING-DRIVEN OUTCOME BYPASS): When a Union `A | B` is piped into `fn(Param)` or partial router `{ Param [...] }`:
//   • If `(A | B) <: Param` (e.g. `Param` is `Any` or `A | B`), `fn` receives the entire `A | B` directly.
//   • If `A <: Param` but `B` is NOT a subtype of `Param`, branch `B` automatically bypasses `fn` without invocation,
//     accumulating into the final Union return type `Out | B`!
// - LAW 4 (UFCS AUTOCOMPLETE & GUARANTEED TCO): Because `x . fn` == `fn(x)`, any function `fn(x Type)` in scope is
//   suggested by IDE autocomplete when typing `val .`. Tail-recursive stream calls are guaranteed to compile into
//   zero-allocation machine loops (Tail-Call Optimization).
// =============================================================================


#
0. CORE AXIOMS, UNIVERSAL SLOTS (`user1`, `user2`), DEPTH-INDEXED `_1` & LINEAGE HISTORY
#

// Single line comments (`//`) and multiline block comments (`# ... #`)
// are homoiconic doc-elements attached to the following or preceding AST node (`Meta.doc_of`).

// --- A. The 4 Core Axioms of Forms & Lineage (Clear Vehicle Hierarchy) ---

// Axiom 1: [TypeName] [BaseType] (immutable type alias) or [var_name] [value] (value binding)
BrandName String
default_user_name "Alice"

// Axiom 2: [FormName] { [key] [Type_or_Default] } (Declares a new immutable Form Type)
Vehicle {
  brand     BrandName            // Required field of type `BrandName` (`String`)
  max_speed Int 180              // Explicit type `Int` with default value `180`
  tag       "standard"           // Inferred type `String` with default `"standard"`
}

// Axiom 3: [FormName] { [key] [Value] } & [Parent1] & [Parent2]
HasBattery {
  battery_kwh Int
}

ElectricVehicle {
  charging_port String "Type2"
} & Vehicle & HasBattery

// Axiom 4: [NewFormName] [BaseForm] { [key] [Override_Value] }
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

// 1. Tight dot '.' access: Active Values vs Ancestor Specification (Lineage History)
//    Note: Compiler resolves static ancestor accesses at compile-time with 0 runtime memory cost!
MyCar.brand                              // "CustomPorsche" (active overridden value on MyCar)
MyCar.SportsCar.brand                    // "Porsche" (value specified at SportsCar ancestor level)
MyCar.SportsCar.Car.brand                // "GenericMotor" (value from Car ancestor)
MyCar.Vehicle.brand                      // BrandName (base specification type on root Vehicle)

MyGarageCar MyCar & { garage_slot 1 }
MyGarageCar.brand                        // "CustomPorsche"
MyGarageCar.MyCar.SportsCar              // Access to entire ancestor slice

MyGarageCar.0                            // Access by field index 0 (brand -> "CustomPorsche")
MyGarageCar.-1                           // Access by field index -1 from end (garage_slot -> 1)

// 2. Updating Data by Deep Field Path, Positional Index (`.0`, `.-1`), or `&` Merge:
mutate_car_demo() [
  my_car (MyGarageCar {})

  // (a) Create new updated Form via `&` while preserving Lineage history:
  upgraded_car (my_car & { max_speed 350, garage_slot 2 })

  // (b) In-place rebinding of `snake_case` variable by field name (Perceus O(1) zero-copy mutation):
  my_car.brand "Tesla"

  // (c) In-place rebinding by positional field index (`.0` is `brand`, `.-1` is `garage_slot`):
  my_car.0  "Rivian"
  my_car.-1 42
]

// --- B. Universal Slot Rule: Generics, Bounded Generics & Auto-Indexed Parameters (Law 1c) ---

Category { title String }
Age      Int

// 1. Unbound 1st-Order Generic Form (`T` is unbound `PascalCase` -> Type Parameter `*`):
Box(T) {
  item T
}

// 2. Bounded Generic Form:
//    • `TC Category` (`PascalCase`) constrains Type Parameter `TC <: Category`
//    • `active_cat TC` (`snake_case`) is a value slot (and automatic Form field) of type `TC`!
CategorizedUser(TC Category, active_cat TC) {
  backup_cat TC                  // Uses `TC` (`PascalCase`) as field Type specification
  current    active_cat          // Uses `active_cat` (`snake_case`) value instance
}

// 3. Single Auto-Named Type Parameter (`Age` is a known Type in scope -> auto-binds `age` via Law 1c):
UserWithAge(Age) {
  new_age age
}

is_user_18(Age) [
  age == 18
]

// 4. Multiple Identical Type Parameters (`(Age, Age)` auto-binds indexed `age1` and `age2` via Law 1c!):
compare_ages(Age, Age) [
  age1 >= age2
]

// 5. Explicit Value Parameter (`recipient` is `snake_case` with Type `String`):
GreetingCard(recipient String) {
  message "Hello, {recipient}!"
}

// 6. Combined Generic + Explicit Value Parameter (`T` is Type Param, `capacity` is Value Param):
BoundedQueue(T, capacity Int) {
  items Tuple(T) ()
  limit capacity
}

// --- C. Law 1 Scoping & Depth-Indexed Stream Subjects (`_`, `_0`, `_1`) ---
WithName { name String }

ReadViaSelf {
  name "Razomy"
  get_name() [ self.name ]                             // Law 1a: `self` references enclosing Form `{}`
}

// Named functions ALWAYS have `()` parameter slots:
read_via_type(WithName)             [ with_name.name ] // Law 1c: Known `WithName` auto-binds `with_name`
join_two(WithName, WithName)        [ "{with_name1.name} & {with_name2.name}" ] // Law 1c: Duplicate types bind `1`, `2`
read_via_var(w WithName)            [ w.name ]         // Law 1d: Explicit parameter variable `w`
read_unpacked({ name } WithName)    [ name ]           // Law 1d: Explicit field destructuring `{ name }`

// Law 1b: `_` is the input of the nearest enclosing `[...]`, while `_0`, `_1`, `_2` index stream depth!
extract_name_demo(w WithName) [                        // Root stream `_0`
  w . [ _1.name ]                                      // 1st nested stream: `_1` (or `_`) is the piped `w`!
]

// Accessing tuple elements inside a nested pipe stream via `_1.0` and `_1.1` (or `_.0` and `_.1`):
sum_pair_pipe(pair (Int, Int)) [                       // Root stream `_0` (where `_0.0` is `pair`)
  pair . [ _1.0 + _1.1 ]                               // Nested stream `_1` (or `_`): `_1.0` + `_1.1` sums `pair`!
]

// Top-level parameterless function uses explicit `()`:
do_init() [ "Razomy v5.5 initialized" . Shell.log ]
do_init()


#
1. PRIMITIVES, EXACT DECIMAL `Money`, TAGGED STRINGS (`Type"..."`) & RAW STRINGS
#
// Razomy eliminates cryptic lexer symbols (`$199`, `%./`) in favor of explicit constructors
// `Money("199.99")` and uniform Type-Prefixed String sugar `Type"..."` == `Type("...")`!
// Raw strings use `r"..."`, `r1"..."1`, `r2"..."2` (no `#`, so `#` is strictly for comments!).

String ""
Symbol 'symbol'
Int    0
Float  0.0
Bool   false
Byte   0x00
Any    _

// Exact Fixed-Point `Money` (4 decimal places stored as integer micro-units: 1.0000 == 10000):
// Eliminates IEEE-754 floating-point rounding errors in financial calculations!
Money(raw String) {
  scaled (raw . Decimal.parse_scale4) // e.g., "100.00" -> 1000000 (Int64)

  +(other Money)   [ Money.from_scaled(self.scaled + other.scaled) ]
  -(other Money)   [ Money.from_scaled(self.scaled - other.scaled) ]
  *(factor Float)  [ Money.from_scaled((self.scaled.to_float * factor).round_to_int) ]
  /(divisor Float) [ Money.from_scaled((self.scaled.to_float / divisor).round_to_int) ]
  ==(other Money)  [ self.scaled == other.scaled ]
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

// 1. Standard, Numbered Raw & Type-Prefixed Strings:
StringExamples {
  // (A) Standard String `"..."`: multiline, evaluates `{expr}` and `\` escapes:
  interpolated "Hello, {default_user_name}!
This string spans multiple lines natively
and escapes \"quotes\" and \{braces\}."

  // (B) Raw Prefixed String `r"..."`, `r1"..."1`, `r2"..."2`:
  //     Multiline, zero escapes, zero interpolation, and zero `#` collision with comments!
  regex_pat    r"^[^@]+@[^@]+\.[^@]+$"
  nested_json  r1"{"query": "SELECT * FROM \"users\"", "interpolated": "{ignored}"}"1
  nested_code  r2"Can freely embed both " and "1 and # without any lexer conflict!"2
}

// 2. Explicit Constructor `Type("...")` vs Concise Tagged Sugar `Type"..."` (100% Equivalent!):
HumanLiterals {
  // Explicit Constructor Form              // Equivalent Tagged Sugar Form (used throughout code)
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

  tax_rate          0.155                   // Percentages/ratios are plain `Float` numbers (15.5% = 0.155)
  email_addr        "admin@razomy.org"      // Standard string
}


#
2. TUPLES `()`, FORMS `{}`, MAPS & TYPE ALGEBRA (`&`, `|`, `/`)
#

// 1. Tuples are expressed via parens `()` (items separated by `,` or `\n`)
Point2D (Float, Float)
origin  (0.0, 0.0)

// 2. Type Unions (Sum Types) with paired symbols `'...'`
Status    'draft' | 'published' | 'archived'
Primitive String | Int | Float | Bool | Byte | Money

// 3. Forms `{}` (Structures) & Lineage Inheritance (structural `==` equality works out of the box)
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

// 4. Advanced Type Subtraction (`/`) & Lineage Disambiguation:
// Case A: Remove field by path or symbol key
PublicUser   User / User.password_hash
AnonymousUsr PublicUser / 'name'

// Case B: Merge causing field collisions on `id` and `name`
UserAndCloud User & Cloud

// Case C: Subtracting a specific ancestor's colliding field resolves the collision!
UserWithCloudName  UserAndCloud / User.name   // `.name` now resolves directly to `Cloud.name`!
CloudWithUserName  UserAndCloud / Cloud.name  // `.name` now resolves directly to `User.name`!

// Case D: Subtracting a symbol key removes `name` from ALL merged ancestors (`User` and `Cloud`)
NoNameEntity       UserAndCloud / 'name'

// Case E: Subtracting an entire ancestor type removes all fields originating from `Cloud`
RestoredUser       UserAndCloud / Cloud

// Case F: Subtracting and overriding a field in one expression
GuestUser          PublicUser / PublicUser.role & { role 'guest' }

// 5. Generic Key-Value Map, Set & Dictionary Manipulation (parens `(K)` define parameterized key)
Map(K, V) { (K) V }
Set(T)    { (T) Bool }

config_map {
  "host" "localhost"
  "port" "8080"
}

updated_config config_map & { "port" "9090", "tls" "true" } / "host"


#
3. UPDATING DATA: DEEP IN-PLACE REBINDING (LENSES) vs `&` MERGE
#
// How does Razomy update fields or local variables without `=` and without cloning?
// 1. `PascalCase` Types/Forms are IMMUTABLE (`Order { id String, amount Int }` cannot be rebound).
//    Writing `Order { id "o1", amount 100 }` inside an expression instantiates `Order`.
// 2. `snake_case` variables inside `[]` can be rebound by name (`counter (counter + 1)` — note the space
//    before `(` which distinguishes binding from function call `counter(1)`!) or by deep path
//    (`corp.departments.0.budget 2500`), which is sugar for nested `&` updates.
// 3. Perceus / Uniqueness Optimization: when `raw_order` is uniquely owned (refcount == 1),
//    the compiler mutates the target offset IN-PLACE in O(1) with ZERO heap allocation!

Order      { id String, amount Int }
Department { title String, budget Int }
Company    { name String, departments Tuple(Department) }

update_order_demo() [
  // Instantiating `Order` and binding to `snake_case` local variable `raw_order`:
  raw_order Order { id "o1", amount 100 }

  // ❌ COMPILE ERROR: Razomy has no `=` assignment operator, and `Order` Type cannot be rebound!
  // raw_order.amount = 14

  // ✅ WAY 1 (Direct Variable Rebinding via spaced parens — e.g. for counters or accumulators):
  counter 0
  counter (counter + 1)

  // ✅ WAY 2 (Path & Deep Lens Rebinding via space — mutates in-place in O(1) when uniquely owned!):
  raw_order.amount 14

  // Single element inside `(Department { ... })` auto-lifts to `Tuple(Department)` without trailing comma:
  corp Company { name "Acme", departments (Department { title "R&D", budget 1000 }) }
  corp.departments.0.budget 2500

  // ✅ WAY 3 (Positional Index Rebinding via space — updates 2nd field `amount` in-place!):
  raw_order.1 20

  // ✅ WAY 4 (Explicit `&` Merge — creates updated view while keeping Lineage history):
  next_order (raw_order & { amount 25 })

  // ✅ WAY 5 (Enrich Pipe `.&` Rebinding):
  next_order (next_order .& { amount 50 })
]


#
4. COMPLETE MATRIX OF `&` COMBINATIONS & HOW RAZOMY REPLACES RUST'S `impl`
#
// In Rust, you need separate keywords (`struct`, `trait`, `impl Type`, `impl Trait for Type`).
// In Razomy, ALL of these are unified under the single algebraic operator `&`!

// --- A. The 6 Canonical Combinations of `A & B` ---

// 1. Form & Form -> Structural Merge / Inheritance / Trait Implementation:
Printable {
  format() String                // Contract signature (no body `[]`)
}

// Replacing Rust's `impl Printable for User` — simply intersect `User & Printable { ... }`:
PrintableUser User & Printable {
  format() [ "User({self.name}, id={self.id})" ]
}

// 2. Type & Stream `[...]` -> Refinement Type (Value-constrained Type):
PositiveInt Int & [ _ > 0 ]
AdultUser   User & [ _.age >= 18 ]

// 3. Function & Form `{...}` -> Partial Application / Default Override / Dependency Injection:
connect(host String "localhost", port Int 8080, tls Bool false) [
  "Connecting to {host}:{port} (TLS: {tls})"
]
// `connect` is a Function, `{ port 9090, tls true }` is a Form overriding its parameter slots:
secure_connect connect & { port 9090, tls true }

// 4. Stream `[...]` & Stream `[...]` -> Sequential Function Composition:
trim_str  [ _ . trim ]
upper_str [ _ . to_upper ]
clean_str trim_str & upper_str   // Produces stream `[ _ . trim . to_upper ]`

// 5. Value/Member & TagForm -> Metadata / Visibility Attachment:
Public  {}
Private {}
secret_key "xyz987" & Private

// 6. Type & Parameterized Trait `Trait(Self)` -> F-Bounded Polymorphism:
Comparable(Other Self) {
  <(other Other) Bool
}

SortableUser User & Comparable(User) {
  <(other User) [ self.age < other.age ]
}

// --- B. Open Extension Methods (UFCS) without modifying the original Form ---
// Because `user . greet` == `greet(user)`, any standalone function `fn(User)`
// automatically acts as a method on `User` (with full IDE autocomplete via Law 4)!
greet(User) [ "Welcome, {user.name}!" ]


#
5. PARAMETERIZED FORMS, EAGER `()` vs LAZY `[]` & RELATIVE OPERATOR `/` OVERLOADING
#

// Parameterized Form `MathOps(a Int, b Int) { ... }`:
// • By Law 1a, constructor value slots `a` and `b` are automatically fields on `self` (`self.a`, `self.b`).
// • Inside `{}`, single parens `(a + b)` compute EAGERLY on construction,
//   while brackets `[ self.a / self.b ]` define a LAZY Zero-Cost Getter evaluated on access!
MathOps(a Int, b Int) {
  add      (a + b)
  sub      (a - b)
  mul      (a * b)
  div      (a / b)
  mod      (a % b)
  pow      (a ** b)
  lazy_div [ self.a / self.b ]

  bit_and  (a . bit_and(b))
  bit_or   (a . bit_or(b))
  bit_xor  (a . bit_xor(b))
  shift_l  (a << 2)
  shift_r  (a >> 2)
}

// Custom Operator Overloading inside Forms (including relative `/` on Value instances!):
// • When `/` is applied to Types/Forms (`User / 'name'`), it performs algebraic type subtraction.
// • When `/` is applied to Form Value instances (`vec_a / 2.0` or `shape_a / shape_b`), it invokes `/(...)`!
Vec2 {
  x Float
  y Float

  +(other Vec2)  [ Vec2 { x (self.x + other.x), y (self.y + other.y) } ]
  -(other Vec2)  [ Vec2 { x (self.x - other.x), y (self.y - other.y) } ]
  *(scale Float) [ Vec2 { x (self.x * scale),   y (self.y * scale) } ]
  /(scale Float) [ Vec2 { x (self.x / scale),   y (self.y / scale) } ]
  ==(other Vec2) [ (self.x == other.x) && (self.y == other.y) ]
}

// Overloading `/` for CSG Boolean Geometry Subtraction (`Shape / Shape`):
Shape {
  vertices Tuple(Vec2)
  /(cutter Shape) [ (self, cutter) . native_csg_subtract ]
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
  in_range (val == (1..100))     // `==` against a Range or Union is infix sugar for `val . matches(1..100)`!
}

// Universal Binding Axiom: `full "{first} {last}"` binds a local variable inside `[]`:
string_ops(first String, last String) [
  full "{first} {last}"

  full
  . trim
  . to_upper
  . split(" ")
]


#
6. HOW FAMILIAR IMPERATIVE CONCEPTS MAP TO RAZOMY (NO `return`, `break`, `null`, `if`)
#
// Razomy has NO imperative statements (`if`, `switch`, `for`, `while`, `return`, `break`, `try/catch`).
// Everything is an expression built from the 3 brackets `{}`, `[]`, `()`:
//
// | Classical Language         | Razomy Equivalent                                         |
// |----------------------------|-----------------------------------------------------------|
// | `if (c) A else B`          | `c . { true [ A ], false [ B ] }`                         |
// | `switch (x) { case ... }`  | `x . { Pattern1 [ A ], Pattern2 [ B ], _ [ Default ] }`   |
// | `return val` (early exit)  | Router branch `{ [ _ < 0 ] [ Err ], _ [ continue_pipe ] }`|
// | `break` (in loop/fold)     | `Break { value acc }` inside `. fold(...)`                |
// | `for (x in list)`          | `list . map([ ... ])` or `list . for_each([ ... ])`       |
// | `while (cond)`             | Tail-recursive stream `loop(state) [ ... loop(next) ]`    |
// | `null` / `nil` / `undefined`| `None {}` inside a Union `User | None` (bypasses via Law 3)|
// | `try { f() } catch (e)`    | Union outcome `f() . { Success [...], IoError [...] }`    |

// Built-in control-flow signal for early termination of `fold` / `reduce` (supports optional label):
Break(T) { value T, label Symbol 'default' }

classical_patterns_demo(numbers Tuple(Int), limit Int) [
  // ❌ COMPILE ERROR: Razomy has no `return`, `break`, or `if` keywords!
  // if (limit < 0) { return 0 }

  // ✅ 1. Early Exit / Guard Clause via Router `{}`:
  limit . {
    [ _ <= 0 ] [ 0 ]
    _ [
      // ✅ 2. Loop with Early `Break` via `fold`:
      numbers . fold(0, (acc Int, n Int) [
        (acc + n) . {
          [ _ > limit ] [ Break { value _ } ] // Immediately halts `fold` and unwraps `value`!
          _             [ _ ]                 // Continues iteration
        }
      ])
    ]
  }
]


#
7. FUNCTION ARGUMENTS: NAMED vs AUTO-NAMED SLOTS, DEFAULTS, DI & PARTIAL APPLICATION
#

Shell {
  log(msg String)  [ msg . native_print ]
  exec(cmd String) [ cmd . native_exec ]
}

Cart     { cart_id String, amount Int }
NotFound { entity String, id String }

Db {
  // 1. Named Slot (`id String`) returning `User | NotFound`:
  find_user(id String) [
    User { id id, name "Alice", age 25, role 'user', password_hash "xyz" }
  ]

  // 2. Known Type Slot (`Identifiable`):
  //    Triggers Law 1c (auto-binds `identifiable`) and Law 3 (bypasses `None` / `NotFound`)!
  find_cart(Identifiable) [
    Cart { cart_id "c_for_{identifiable.id}", amount 2500 }
  ]
}

// 3. Default Arguments, Partial Application & Dependency Injection via `&`:
default_conn   connect()                                // Uses all defaults ("localhost", 8080, false)
partial_conn   connect("db.internal")                   // Fixes 1st arg, returns fn of (port, tls)
named_call     connect(port 9090, tls true)             // Passes named arguments directly in `()`
custom_connect connect & { port 9090, tls true }        // Rebinds default slots via `&` into a new fn
active_conn    custom_connect()                         // Invokes the customized function

// 4. Function Refinements via Option Forms:
FindOptions { case_sensitive false, limit 0, last false }

find(collection, target, opts FindOptions) [
  collection . native_find(target, opts.case_sensitive, opts.limit, opts.last)
]

search_users(names) [
  names . find("Alice", FindOptions & { case_sensitive true, limit 5 })
]


#
8. PIPELINE OPERATORS (`.`, `.>`, `.&`, `.?`, `.|`), CONTEXT ENRICHMENT & MULTI-QUERY `do`-NOTATION
#

// Known intersection slot `(User & Cart)` binds `user` and `cart` (Law 1c).
// Notice how `user`, `cart`, `self`, and `_` (or depth-indexed `_1`) have zero ambiguity:
finalize_bill(User & Cart) [
  (user & cart) & {
    discount [
      self.amount . {
        [ _ >= 2000 ] [ 300 ]
        [ _ >= 1000 ] [ 100 ]
        _             [ 0 ]
      }
    ]
    final_price [ self.amount - self.discount ]
  }
  .> [ "Receipt for {_1.name}: {_1.final_price} USD (Discount: {_1.discount})" . Shell.log ]
]

// 1. Linear Pipeline with Dependency Injection slot (`db Db`):
//    In tests, override via `(process_order_pipeline & { db MockDb })("u1")`!
process_order_pipeline(user_id String, db Db) [
  user_id
  .  db.find_user         // `.`  Standard Pipe: returns `User | NotFound`
  .? [ _.age >= 18 ]      // `.?` Filter Pipe: returns `User | NotFound | None`
  .& db.find_cart         // `.&` Enrich Pipe: `NotFound | None` bypasses via Law 3; `User` merges -> `(User & Cart) | NotFound | None`
  .  finalize_bill        // `.`  Processes `(User & Cart)`, while `NotFound | None` bypasses to final output
]

// 2. Combining Multiple Fallible Queries (Replaces Haskell `do`-notation and Rust `?` operator!):
//    Because `.&` distributes over Unions (`Context .& (Result | Err)` -> `(Context & Result) | Err`),
//    any error in step 1 or step 2 automatically short-circuits the remaining `.&` steps via Law 3!
load_user_and_friend(uid1 String, uid2 String) [
  { first_id uid1, second_id uid2 }
  .& [ { u1 (_.first_id  . Db.find_user) } ] // If `NotFound`, bypasses remaining steps!
  .& [ { u2 (_.second_id . Db.find_user) } ] // Only runs if `u1` succeeded!
  .  [ "{_.u1.name} is friends with {_.u2.name}" ]
]


#
9. REFINEMENT TYPES & UNIVERSAL ROUTER AXIOM (`{ Pattern [Body] }`)
#
// Every key `K` in a router `x . { K [Body] }` is sugar for `[ _ . matches(K) ]`:
// • Value `(0, 0)` -> `[ _ == (0, 0) ]`
// • Type `Manager` -> `[ _ . is(Manager) ]` + auto-binds `manager` (Law 1c)
// • Tuple of Types `(Disconnected, ConnectCmd)` -> matches both types + binds `disconnected` & `connect_cmd`!

// Predicates can specify known input type `(User)` or use stream subject `_`:
is_adult(User)   [ user.age >= 18 ]
is_minor(User)   [ user.age < 18 ]
is_manager(User) [ user.role == ('admin' | 'manager') ] // Infix sugar for `user.role . matches('admin' | 'manager')`
non_empty        [ _.length > 0 ]
EmailStr         String & [ _ . Regex.matches(r"^[^@]+@[^@]+\.[^@]+$") ]

// Refinement Type = Base Type `User` intersected `&` with boolean predicate function:
Adult   User & is_adult
Minor   User & is_minor
Manager User & is_manager

classify_account(account User) [
  account . {
    // Law 1c: Matching Type `Manager` automatically binds variable `manager`!
    Manager [ "Dashboard: {manager.name}" . Shell.log ]
    Adult   [ "Adult account: {adult.name}" . Shell.log ]
    Minor   [ "Child account: {minor.name}" . Shell.log ]
    _       [ "Unknown profile" . Shell.log ]
  }
]

// Named `Geometry` so it never collides with `Form` `{}` terminology:
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
// Ordered lists of data use parens `()`, keeping `[]` exclusively for executable streams!
// Inside `()`, items can be separated by newlines `\n` or commas `,` interchangeably.

Item { name String, price Int, in_stock Bool }

order_list (
  Item { name "Laptop", price 1200, in_stock true }
  Item { name "Mouse",  price 25,   in_stock false }
  Item { name "Screen", price 400,  in_stock true }
)

calculate_total_in_stock(items Tuple(Item)) [
  items
  . filter([ _.in_stock == true ])
  . map([ _.price ])
  . fold(0, (acc Int, price Int) [ acc + price ])
]

// Parallel iteration via `zip` and indexed iteration via `enumerate`:
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
      (0, 0) [ "FizzBuzz" ]      // Sugar for `[ _ == (0, 0) ]`
      (0, _) [ "Fizz" ]          // Sugar for `[ _.0 == 0 ]`
      (_, 0) [ "Buzz" ]          // Sugar for `[ _.1 == 0 ]`
      _      [ "{n}" ]
    }
  ])
]


#
11. LINEAGE COLLISION RESOLUTION (LAW 2), ERROR CASES & TYPE MIGRATION
#

// 1. Merging `User` and `Order` causes a collision on the `id` field
UserOrder User & Order

process_combined(combined_context) [
  combined_context . {
    UserOrder [
      // Unique fields (`name`, `amount`) are accessed directly on `user_order`:
      "Client Name: {user_order.name}, Order Amount: {user_order.amount}" . Shell.log

      // ❌ COMPILE ERROR (Law 2 Violation): Ambiguous field `id` (present in both `User` and `Order`)
      // "ID: {user_order.id}" . Shell.log

      // ✅ VALID: Colliding fields are accessed via explicit Lineage path:
      "User ID: {user_order.User.id}, Order ID: {user_order.Order.id}" . Shell.log
    ]
  }
]

// 2. Resolving collision permanently via Subtraction `/`:
UserOrderClean UserOrder / Order.id // Removes Order's id; `.id` now unambiguously refers to `User.id`!

// 3. Declarative Type Migration:
ClientAccount { account_id String, name String, is_adult Bool }

// Law 1c: Known Type slot `(User)` binds `user` for clean, self-documented mapping:
migrate_user(User) [
  ClientAccount {
    account_id user.id
    name       user.name
    is_adult   (user.age >= 18)
  }
]


#
12. REACTIVE FORMS (`self`) & FINE-GRAINED REACTIVE SIGNALS (`Signal`, `Effect`)
#

// 1. Excel-Style Reactive Forms `{}`:
//    Reactive streams `[]` inside a Form `{}` compile to zero-cost inline getters
//    and recompute automatically when source fields are overridden via `&`:
Invoice {
  price    Money"100.00"
  quantity 3.0
  tax_rate 0.20

  subtotal [ self.price * self.quantity ]
  tax      [ self.subtotal * self.tax_rate ]
  total    [ self.subtotal + self.tax ]
}

order_1 Invoice & { quantity 5.0 }
// order_1.total automatically evaluates to Money"600.00" using exact fixed-point arithmetic!

// 2. Fine-Grained Reactive Signals (`Signal(T)` + Computed `[...]` + `Effect([...])`):
count_sig   Signal(2)
doubled_sig [ count_sig.get() * 2 ]

run_signal_demo() [
  Effect([
    "Count: {count_sig.get()}, Doubled: {doubled_sig()}" . Shell.log
  ])
  count_sig.set(10) // Automatically re-runs Effect -> "Count: 10, Doubled: 20"
]


#
13. ONTOLOGICAL TYPING (TYPESTATE), OUTCOME BYPASS (LAW 3) & IDE AUTOCOMPLETE (LAW 4)
#
// Instead of exceptions or Result<T, E> monads, the pipeline collects outcomes into a Union.

Reserved   { reserved_at Int }
Paid       { transaction_id String }
Cancelled  { cancelled_at Int }

OutOfStock { reason String }
NoMoney    { reason String }

// Negative Type Constraint (Typestate Guard): because `Cancelled` is not a base field of `Order`,
// `Order / Cancelled` means `Order & ![ _ . is(Cancelled) ]` — cancelled orders cannot enter fulfillment!
reserve_stock(o Order / Cancelled) [
  o.amount . {
    [ _ > 0 ] [ o & Reserved { reserved_at 1620000000 } ]
    _         [ OutOfStock { reason "Warehouse empty" } ]
  }
]

charge_payment(o Order & Reserved) [
  o.amount . {
    [ _ < 50000 ] [ o & Paid { transaction_id "tx_001" } ]
    _             [ NoMoney { reason "Insufficient funds" } ]
  }
]

// Inferred return type: `(Order & Reserved & Paid) | OutOfStock | NoMoney`
// If `reserve_stock` returns `OutOfStock`, it automatically bypasses `charge_payment` (Law 3)!
execute_fulfillment(initial_order Order / Cancelled) [
  initial_order
  . reserve_stock
  . charge_payment
]

ship_to_customer(order Order & Reserved & Paid) [
  "Shipping order {order.id}. Transaction {order.transaction_id} confirmed." . Shell.log
]

// Subtyping Rule of Law 3: A function accepting the full Union (or `Any`) does NOT bypass!
audit_any_outcome(outcome (Order & Reserved & Paid) | OutOfStock | NoMoney) [
  "Auditing fulfillment outcome: {outcome}" . Shell.log
]

// --- LAW 4: IDE AUTOCOMPLETE FOR COMPLEMENTARY FUNCTIONS ---
// Because `o . reserve_stock` == `reserve_stock(o)`, whenever you have a variable of type
// `Order / Cancelled` and type `.`, the IDE automatically proposes `reserve_stock` and
// `execute_fulfillment` as complementary functions for that exact typestate!


#
14. TYPE-LEVEL COMPUTATIONS, RECURSIVE `Self` & 1ST/2ND/3RD-ORDER GENERICS (HKT & MONADS)
#

// 1. Type-level Peano numbers & Fixed-Length Vector (`T`, `N` are 1st-Order Type Params `*`)
Zero    {}
Succ(N) { prev N }
One     Succ(Zero)
Two     Succ(One)
Three   Succ(Two)

Tuple(T) ()

FixedVector(T, N) {
  data   Tuple(T)
  length N
}

// 2. Self-Recursive `Json` Union (`Self` references `Json`):
None {}
Json String | Int | Float | Bool | None | Tuple(Self) | Map(String, Self)

// 3. Recursive Generics: Linked List, Multi-Branch Tree (`Self`) and Binary Tree
Cons(T) {
  head T
  tail List(T)
}
List(T) None | Cons(T)

RoseTree(T) {
  value    T
  children Tuple(Self) ()
}

Leaf {}
Node(T) {
  value T
  left  Tree(T)
  right Tree(T)
}
Tree(T) Leaf | Node(T)

// 4. F-Bounded Polymorphism (replaces `T extends Comparable<T>`)
sort(items List(Comparable)) [
  items . native_sort_by((a, b) [ a < b ])
]

sort_users(users List(User & Comparable(User))) [
  users . sort
]

// 5. Higher-Order Generics: 1st-Order (`*`), 2nd-Order HKT (`* -> *`), and 3rd-Order (`(* -> *) -> * -> *`)!
//    • Function type signature is written naturally via Tuple + Stream: `(A) [B]`
//    • 2nd-order type constructor parameter is written `F(_)`
//    • 3rd-order type constructor parameter is written `T(_(_), _)`
//    • Unbound `PascalCase` slots (`A`, `B`) are compile-time type parameters inferred automatically at call sites!

// 1st-Order Generic Application:
NestedBox(T) Box(Box(T))

// 2nd-Order Generics (Higher-Kinded Types: Functor & Monad Contracts):
Functor(F(_)) {
  fmap(A, B, fa F(A), f (A) [B]) F(B)
}

Monad(M(_)) {
  pure(A, a A)                          M(A)
  flat_map(A, B, ma M(A), f (A) [M(B)]) M(B)
} & Functor(M)

// Concrete Implementation of 2nd-Order `Monad` for `Option` via `&`:
Some(T)   { value T }
Option(T) Some(T) | None

OptionMonad Monad(Option) {
  fmap(A, B, fa Option(A), f (A) [B]) [
    fa . {
      Some [ Some { value (some.value . f) } ]
      None [ None ]
    }
  ]
  pure(A, a A) [ Some { value a } ]
  flat_map(A, B, ma Option(A), f (A) [Option(B)]) [
    ma . {
      Some [ some.value . f ]
      None [ None ]
    }
  ]
}

// 3rd-Order Generics (Monad Transformer `OptionT` & `MonadTrans` with exact Kind `(* -> *) -> * -> *`):
OptionT(M(_), A) {
  run M(Option(A))
}

MonadTrans(T(_(_), _)) {
  lift(M(_) Monad, A, ma M(A)) T(M, A)
}


#
15. HIGHER-ORDER FUNCTIONS, EARLY EXIT (`Break`) & CATAMORPHISMS
#

// 1. Functional Early Exit in `fold` via `Break` (No imperative `break`/`return` needed)
sum_until_limit(numbers List(Int), max Int) [
  numbers . fold(0, (acc Int, n Int) [
    (acc + n) . {
      [ _ > max ] [ Break { value _ } ] // Immediately halts `fold` and unwraps `value`!
      _           [ _ ]                 // Continues iteration
    }
  ])
]

// Lazy stream short-circuiting via `take_while` and `find`:
first_adult(users List(User)) [
  users
  . take_while([ _.age > 0 ])
  . find([ _.age >= 18 ])        // Returns `User | None` immediately upon first match
]

// 2. Functor over recursive List (`A`, `B` are inferred compile-time type params; `Cons` match auto-binds `cons` via Law 1c):
list_map(A, B, l List(A), action (A) [B]) List(B) [
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

// 3. Catamorphism (Generic AST Folder with decoupled Evaluation Algebras)
Num(T)    { value T }
Add(L, R) { left L, right R }
Mul(L, R) { left L, right R }
Expr      Num(Int) | Add(Expr, Expr) | Mul(Expr, Expr)

cata(expression Expr, algebra) [
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


#
16. CONCURRENCY (`.|` PARALLEL PIPE, CSP CHANNELS), ATOMS & STATE MACHINES
#

// 1. Parallel Fan-Out Operator `.|` (Structured Concurrency + Fault Isolation):
// All branches in `{}` execute concurrently. If `ads` times out or returns `HttpError`,
// Law 3 isolates the failure inside the `.ads` field (`AdData | TimeoutError`) without
// crashing or canceling `profile` and `recs`!
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

// 2. CSP Channels & Tail-Recursive Actor Loop (`self` references the Channel instance via Law 1a)
Channel(T) {
  send(val T) [ (self, val) . native_chan_send ]
  recv()      [ self . native_chan_recv ]
}

worker_loop(jobs Channel(String), results Channel(Int)) [
  jobs.recv() . {
    None [ "Channel closed, terminating worker." . Shell.log ]
    Some [
      some.value
      . length
      . results.send

      worker_loop(jobs, results)
    ]
  }
]

// 3. Pure State Machine Reducer & Transactional Atom
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

Atom(T) {
  value            T
  read()           [ self . native_atom_read ]
  swap(transition) [ (self, transition) . native_atom_swap ]
}

global_counter Atom(CounterState) {
  value CounterState { count 0 }
}

modify_global_counter() [
  global_counter.swap((state CounterState) [ update_counter(state, Increment { by 5 }) ])
]

// 4. Explicit Finite State Machine (Connection Lifecycle FSM):
//    Notice how `(Disconnected, ConnectCmd)` is a Tuple of Types: by the Universal Router Axiom,
//    it checks `[ _.0 . is(Disconnected) && _.1 . is(ConnectCmd) ]` and auto-binds
//    `disconnected` and `connect_cmd` into the branch scope (Law 1c)!
Disconnected { reason String }
Connecting   { host String, attempt Int }
Connected    { host String, session_id String }
ConnState    Disconnected | Connecting | Connected

ConnectCmd    { host String }
HandshakeOk   { session_id String }
NetworkError  { reason String }
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
17. RESOURCE MANAGEMENT, SCOPED LIFETIMES & UNIFIED PORTS
#

FileHandle { fd Int, path Path }
IoError    { msg String }

Fs {
  open(path Path)   [ path . native_fs_open ]           // Returns `FileHandle | IoError`
  read(FileHandle)  [ file_handle.fd . native_fs_read ] // Law 1c binds `file_handle`
  close(FileHandle) [ file_handle.fd . native_fs_close ]

  // Bracket pattern: `FileHandle` match automatically binds `file_handle` so `Fs.close`
  // closes the file after `action` while returning `action`'s result (`IoError` bypasses the
  // partial router `{ FileHandle [...] }` directly into the return Union via Law 3)!
  with_file(path Path, action) [
    path . Fs.open . {
      FileHandle [
        file_handle . action
        .> [ file_handle . Fs.close ]
      ]
    }
  ]
}

read_config(path Path) [
  Fs.with_file(path, (f FileHandle) [
    f . Fs.read . Json.parse
  ])
]

// Unified Port Protocol: `Url"..."` and `Path"..."` share identical read/write verbs
sync_remote_backup() [
  Url"https://api.razomy.org/v1/users"
  . Port.read
  . Port.write(Path"./backup/users.json")
]


#
18. DEEP PATTERN ROUTING (STRUCTURAL & TUPLE DESTRUCTURING)
#
// Routers `{}` can simultaneously match Form fields and destructure nested Tuples `()`.

HttpRequest {
  method  'GET' | 'POST' | 'PUT' | 'DELETE'
  path    Tuple(String)
  headers Map(String, String)
  body    String
}

HttpResponse { status Int, body String }
Unauthorized { reason String }

auth_middleware(req HttpRequest) [
  req.headers . Map.get("Authorization") . {
    Some [ req & { auth_user (some.value . Db.find_user_by_token) } ]
    None [ Unauthorized { reason "Missing Authorization Header" } ]
  }
]

api_router(req HttpRequest) [
  req . {
    // Matches exact Form field AND single-element Tuple (via Tuple Auto-Lifting)
    { method 'GET', path ("health") } [
      HttpResponse { status 200, body "OK" }
    ]

    // Matches `method 'GET'` AND extracts variable `id` from the 2nd Tuple element!
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

// 1. Parse Dialect: PEG grammar stream where cursor state flows via `.` through rules.
//    Each combinator returns a cursor stream `[ _ . native_parse_... ]` so rules compose
//    seamlessly via `.` or `&` inside `Parse.run(...)`!
Parse {
  end                        'eof'
  match(pat)                 [ _ . native_parse_match(pat) ]
  one_or_more(rule)          [ _ . native_parse_one_or_more(rule) ]
  repeat(bounds, rule)       [ _ . native_parse_repeat(bounds, rule) ]
  skip(count Int)            [ _ . native_parse_skip(count) ]
  to(delimiter)              [ _ . native_parse_until(delimiter) ]
  copy(field Symbol, target) [ _ . native_parse_capture(field, target) ]
  run(input String, rules)   [ input . rules ]
}

digit "0".."9"
alpha "a".."z" | "A".."Z"

EmailGrammar String & [
  _ . Parse.run(
    Parse.one_or_more(alpha)
    & Parse.match("@")
    & Parse.one_or_more(alpha)
    & Parse.match(".")
    & Parse.repeat(2..6, alpha)
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

// 2. View & Draw Dialect: Declarative UI in Forms `{}` & Tuples `()`, canvas commands in `[]`
// Tuple-Argument Duality: Because incoming argument slots `()` ARE a Tuple, a function
// expecting `children Tuple` directly accepts comma- or newline-separated elements without double parens `(())`!
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
20. METAPROGRAMMING, DOC-COMMENTS (`Meta.doc_of`), COMPOSE & SELF-HOSTED `Meta.rewrite`
#

// Homoiconic `Meta` Module (Reflection, Partial Evaluation, Doc-Comments & Deep AST Rewriting):
Meta {
  type_of(entity)           [ entity . native_meta_type_of ]
  doc_of(entity)            [ entity . native_meta_doc_of ]
  fields_of(entity)         [ entity . native_meta_fields_of ]
  steps_of(stream)          [ stream . native_meta_steps_of ]
  map_fields(form, mapper)  [ (form, mapper) . native_meta_map_fields ]
  compose(stream)           [ stream . native_meta_eval_parens ]

  // Self-Hosted Deep Tree Rewriting (`Meta.rewrite` in 10 Lines of Razomy!):
  rewrite(node, rules) [
    // Step 1: Recursively traverse children across all 3 brackets (`Form`, `Tuple`, `Stream`):
    walked (node . {
      Form   [ node . Meta.map_fields((k, v) [ (k, v . Meta.rewrite(rules)) ]) ]
      Tuple  [ node . map([ _ . Meta.rewrite(rules) ]) ]
      Stream [
        node . Meta.steps_of
        . map([ _ . Meta.rewrite(rules) ])
        . fold([ _ ], (acc, step) [ acc & step ])
      ]
      _      [ node ]
    })

    // Step 2: Apply pattern rules to current node (or keep `walked` if no rule matches):
    walked . (rules & { _ [ walked ] })
  ]
}

// 1. Algebraic Stream Composition
step_1 [ _ + 10 ]
step_2 [ _ * 2 ]
combined_pipeline step_1 & step_2

// 2. Partial Evaluation (`Meta.compose`): evaluates only parenthesized `()` expressions inside `[]`
build_multiplier(factor Int) [
  Meta.compose([ _ * (factor * 10) ]) // Returns stream `[ _ * 50 ]` when factor == 5
]

// 3. Aspect-Oriented Stream Manipulation (`Meta.steps_of` + `fold` via `&`):
trace_execution(code_block) [
  code_block
  . Meta.steps_of
  . map((step) [
    step & [ _ .> [ "Step result: {_}" . Shell.log ] ]
  ])
  . fold([ _ ], (acc, step) [ acc & step ])
]

// 4. Compile-time AST Macro (`derive_json`), Reflection (`Meta.fields_of`) & Doc-Comments (`Meta.doc_of`):
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
]

#
Represents a catalog product with auto-derived JSON serialization.
Comments attached before or inline on declarations are inspectable via `Meta.doc_of`!
#
Product ({
  id    Int    // Unique product identifier
  title String // Display title in catalog
} . derive_json)

// Inspecting types, fields, and homoiconic documentation comments at compile-time or runtime:
inspect_entity(entity) [
  type_name (entity . Meta.type_of)
  doc_text  (entity . Meta.doc_of)
  "Inspecting type: {type_name} (Doc: {doc_text})" . Shell.log

  entity . Meta.fields_of . for_each((f) [
    "Field {f.name} ({f.doc}) has type {f.type} and value {f.value}" . Shell.log
  ])
]


#
21. IMPORTS, MODULE ENCAPSULATION (`/ Private`) & NATIVE WRAPPERS
#

// 1. Standard & Relative Imports (Native C/Rust/JS libraries are wrapped as normal `.rl` modules)
Base          root.base                // Named module import (`PascalCase`)
(String, Int) root.base                // Element destructuring import via Tuple `()`
_             root.base                // Wildcard import (all)
_             base                     // Relative module import
_             Path"../../../base"      // Deep relative path import via `Path"..."`
Http          std.net.http             // Standard library wrapper module
Sqlite        std.db.sqlite            // Native wrapper module (no FFI pollution in user code)

// 2. Module Visibility & Encapsulation via Algebraic Tag Subtraction `/ Private`:
AuthInternal {
  salt                     "x9f2a" & Private
  hash(raw String)         [ "{raw}_{self.salt}" . native_sha256 ] & Private
  verify(u User, p String) [ (p . self.hash) == u.password_hash ] & Public
}

// Subtracting `Private` removes all members tagged with `& Private`, exposing only public API:
AuthModule  AuthInternal / Private
CleanModule AuthInternal / 'salt' / 'hash'

// ❌ COMPILE ERROR: `salt` was subtracted on `AuthModule`:
// forbidden_read AuthModule.salt


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

CompileError { code Int, log String }
BuildSuccess { status String, artifact String }
BuildResult  BuildSuccess | CompileError

FatalError CompileError & [ _.code >= 500 ]
Warning    CompileError & [ _.code < 500 ]

AiQuery    { system_prompt String, user_prompt String }
AiResponse { diff String, explanation String }

// Explicit AgentState Form keeps all retry and patch history inspectable in one place:
AgentState {
  err             CompileError
  retries         Int
  applied_patches Tuple(String) ()
}

Ai {
  call(query AiQuery) [ query . native_ai_llm_request ]
}

run_build() [ "cargo build --release" . Shell.exec ]

// Pure tail-recursive loop over `AgentState` (compiled to a zero-overhead machine loop via Law 4 TCO):
agent_loop(state AgentState) [
  state.retries . {
    [ _ == 0 ] [ "Retry limit reached. Human intervention required." . Shell.log ]

    _ [
      ai_resp (
        AiQuery {
          system_prompt "You are an engineer. Fix the compilation error with a git diff patch."
          user_prompt   "Error log: {state.err.log}. Write a patch."
        }
        . Ai.call
        .> [ "AI suggests solution: {_1.explanation}" . Shell.log ]
        .> [ "git apply {_1.diff}" . Shell.exec ]
      )

      run_build() . {
        BuildSuccess [
          "Build succeeded after applying patch!" . Shell.log
        ]

        // Law 1c: `CompileError` match binds `compile_error`
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

// Top-level entry point function with explicit `()`:
main() [
  "Starting test suite..." . Shell.log
  run_test_suite()

  "Starting project build process..." . Shell.log
  run_build() . {
    FatalError [
      // Law 1c: `FatalError` binds `fatal_error`
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
24. APPENDIX: ADVANCED HOMOICONIC GRAPH REWRITING & SCENE OPTIMIZATION
#

ComplexSystem {
  name "Core"
  calc(x Float) [ x * 1.15 ]
  SubModule {
    Nested {
      logger(msg String) [ msg . Shell.log ]
      calc(x Float)      [ x * 1.15 ]
    }
  }
}

UpgradedSystem (ComplexSystem . Meta.rewrite({
  calc(_)   [ (x Float) [ x * 1.20 ] ] // Replaces all `calc` functions at every depth
  Shell.log [ Audit.write ]            // Replaces `Shell.log` inside any nested stream
}))

// --- Case 1: Simultaneous Field Declaration & Call-Site Refactoring ---
IdNameController {
  full_name [ "{self.name_attribute}_{self.i_id_attribute.id}" ]
}
LoggerController {
  on_start() [ self.i_id_attribute.id . Shell.log ]
}

refactor_id_to_my_id(context_tree) [
  context_tree . Meta.rewrite({
    // (a) Replace field declaration in Forms `{}`:
    { i_id_attribute val } [
      { my_id (MyAttributeForCurrentContext & val) }
    ]
    // (b) Replace all read accesses inside Streams `[]`:
    self.i_id_attribute [ self.my_id ]
    _.i_id_attribute    [ _.my_id ]
    i_id_attribute      [ my_id ]
  })
]

// --- Case 2A: Collapse a Component Group via Ancestor Subtraction (`/`) ---
Cuboid {
  size     (1.0, 1.0, 1.0)
  anchor   (0.5, 0.5, 0.5)
  position (0.0, 0.0, 0.0)
  pivot    (0.5, 0.5, 0.5)
  rotation (0.0, 0.0, 0.0)
  scale    (1.0, 1.0, 1.0)
}
PlayButton Cuboid & { label "Play" }

// Subtract all 6 `Cuboid` fields at once and attach a single `canvas_2d_rect`:
FlatPlayButton (PlayButton / Cuboid) & {
  canvas_2d_rect (0.5, 0.35, 0.15, 0.15)
}

// --- Case 2B: Subgraph Reduction (Batching Child Nodes into One `baked_layer`) ---
optimize_graph(root_context) [
  root_context . Meta.rewrite({
    {
      children {
        background { image bg_img, rounded r1 }
        foreground { image fg_img, rounded r2 }
        icon       icon_node
      }
    } [
      {
        children {
          baked_layer {
            combined_images (bg_img, fg_img)
            rounded         r1
          }
          icon icon_node
        }
      }
    ]
  })
]

// --- Case 2C: Flatten an Entire Deep Component Subtree into a Single Native Node ---
RoundButton Cuboid & {
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
]

// --- Architectural Best Practices for Graph Rewriting ---
// Rule 1: Preserve Lineage (`my_btn PlayButton & MyIdContext`) instead of destructive edits.
// Rule 2: Use Default Parameter Slots for 90% of variation instead of AST rewriting:
Element(id_attr ProceduralId) {
  id id_attr
} & Cuboid & StartNode

default_el Element()
custom_el  (Element & { id_attr MyAttributeForCurrentContext })()

// Rule 3: Run Graph `rewrite` in one centralized Render Pipeline at the end:
build_screen(raw_tree) [
  raw_tree
  . refactor_id_to_my_id
  . optimize_graph
  . render_for_mobile
]