// =============================================================================
// RAZOMY V5.7 (`.rl`) — HOMOICONIC AXIOMATICS OF THREE BRACKETS
// =============================================================================
// 1. THREE BRACKETS (STRICT SEPARATION, STRUCTURAL `==` & HASHING BY DEFAULT):
// {} — Form   (Key-Value: Structs, Classes, Trait impls, Routers, Modules, Maps).
//             `{(K) V}` = parameterized Map key or compound AST/path pattern key (`{(self.x) [...]}`).
// [] — Stream (Lazy executable sequence, Predicates, Lambdas, Dialects, Zero-cost getters).
//             Subject: `_` (innermost `[]`), or depth-indexed `_0` (root fn `[]`), `_1`, `_2` (nested/router `[]`).
// () — Tuple  (Ordered array, Argument slots, Eager value evaluation in `{}`).
//             • Canonical Triad `Name(Slots) {Form} [Stream]`:
//               - `Name { ... }`                      -> Immutable Form Prototype / Type (`PascalCase`)
//               - `Name(T, U) { ... }`                -> 1st-Order Generic (`*`; unbound `PascalCase`, no shadowing!)
//               - `Name(F(_)) { ... }`                -> 2nd-Order Generic / HKT (`* -> *`, invariant)
//               - `Name(T(_(_), _)) { ... }`          -> 3rd-Order Generic (`(* -> *) -> * -> *`)
//               - `Name(TC Category, cat TC) { ... }` -> Bounded Generic (`TC <: Category` + value slot `cat`)
//               - `Name(Age) { new_age age }`         -> Auto-Unpacked Class (single known `Age` binds `age` via Law 1c)
//               - `Name(u1 User, u2 User) { ... }`    -> Duplicate/compound slots (`A & B`) MUST have `snake_case` names!
//               - `Name(a Type, b Int) { ... }`       -> Explicit Class (`snake_case` slots become `self` fields!)
//               - `Name(T, a T) { ... }`              -> Generic Class (`T` Type Param + `a` Value Param)
//               - `name(A, B, x A) B [Body]`          -> Generic fn (`A`, `B` inferred; `val . name` binds to 1st value slot `x`)
//               - `name(args) Ret [Body]` / `name(k 10) [Body]` / `name() [Body]` -> Named functions
//               - `(args) [Body]` / `[Body]`          -> Anonymous lambda / stream
//               - `name(args) Ret` / `cb (In) [Out]`  -> Contract signature in `{}` / Function type in slots
//             • Tuple-Argument Duality: Incoming `(a, b)` IS a Tuple `()`. Functions taking `Tuple` accept `fn(a, b)`
//               without `(())`; single element `(x)` auto-lifts to `Tuple(T)` without trailing comma.
//
// 2. LEXER, CASE, STRINGS & BINDING AXIOMS:
// • PascalCase — Immutable Types/Forms/Prototypes/Variants. `Form.field` / `Form.0` reads static default;
//                `Form { k v }` or `Form {}` in expressions instantiates runtime instance.
// • snake_case — Variables, fields, functions, value params. ONLY `snake_case` inside `[]` can be mutated in-place!
// • Compile    — `Compile & [...]` or `fn() [...] & Compile` evaluates 100% at compile-time (0 runtime overhead).
// • Literals   — `'symbol'` (constant tag); `"..."` (multiline, `{expr}` interp, `\n \t \\ \" \{ \}` escapes);
//                `r"..."`, `r1"..."1`, `r2"..."2` (raw multiline, 0 escapes/interp, no `#` collision);
//                `Type"..."` == 100% sugar for constructor `Type("...")` (`Money"199.99"`, `Path"./a"`).
// • ',' & '\n' — 100% interchangeable across `{}`, `[]`, `()`.
// • '//' & '#' — Single-line `//` and block `# ... #` (with whitespace after `#`) attach to AST as `Meta.doc_of`.
// • ' ' (Space)— (a) Universal Binding (NO `=` in Razomy!): `k v`, deep lens `a.b.0.c val`, or `(a, b) src`.
//                (b) Tight Call `fn(x)` vs Spaced Bind `x (expr)` (emits `E01` if `x` is a Function in scope!).
//                (c) Binary ops (`+ - * / % ** << >> == != > < >= <= && || . .> .& .? .| & |`) REQUIRE spaces;
//                    unary `!` is tight (`!x`). Operator defs in `{}` have space before params: `+ (o T) [...]`.
//
// 3. OPERATORS & ROUTERS:
// • `a.b` (tight): field, module, lineage `a.Parent.field`, or index `a.0`, `a.-1`.
// • `a . b`: Pipe `b(a)` / UFCS `a . b(c)` == `b(a, c)` | `a .> b`: Tap (side-effect, returns `a`)
// • `a .& b`: Enrich (`a & b(a)`, distributes over `Bypass`: `A .& (B | Err)` -> `(A & B) | Err`)
// • `a .? b`: Filter (returns `a | None`) | `a .| {}`: Parallel fan-out of branches, merged via `&`.
// • `a & b` (7 Canonical Combinations): 1. Form & Form (Inherit/Trait impl/Merge) 2. Type & Stream (Refinement)
//   3. Fn & Form (Partial app/DI override) 4. Stream & Stream (Sequential compose) 5. Val & Tag (Metadata/Compile)
//   6. Type & Trait(Type) (F-Bounded) 7. Type & Bypass (Marks short-circuiting failure/empty Union branch).
// • `a | b`: Union | `a..b`: Range | `x == Pattern`: Equality or `x . matches(Range | Union)`.
// • `a / b`: Type/Form/Map Subtraction (`Form / Form.field`, `Form / 'key'`, `Form / Ancestor`, `Map / key`,
//   `Union / Branch`), Negative Typestate Guard (`Form / State` == `Form & ![ _ . is(State) ]` if `State` not in
//   lineage), or Value Operator overload `/(other)`.
// • `{ k (e) }` vs `{ k [e] }`: `(e)` is eager field stored in struct; `[e]` is 0-byte static inline getter
//   `Form.get_k(self)` (or reactive computed node on `Signal`), keeping `RefCount == 1` for Perceus O(1) mutation!
// • `x . { K [V] }`: Router (`K` is sugar for `[ _ . matches(K) ]`: value `0`, type `Circle`, type tuple `(A, B)`,
//   structural `{ path ("u", id) }`, computed `(self.x)`, or predicate `[ _ > 0 ]`).
//
// 4. FOUR IRON LAWS:
// • LAW 1 (Scoping): (a) `self.field` in `{}` (value slots in `Form(a Int)` are `self` fields);
//   (b) `_` (nearest `[]`) & `_0, _1, _2` (depth-indexed `[]`); (c) `Name(KnownType)` auto-binds single atomic
//   known type to `known_type` (duplicates `(T, T)` or compound `(A & B)` -> `E02`; unbound type param shadowing
//   existing type -> `E03`); (d) explicit `fn(u User)` or destructuring `fn({ name } User)`.
// • LAW 2 (Lineage): Colliding fields in `A & B` require `x.A.f` / `x.B.f` (`E05` if direct) until subtracted via `/`.
//   Unreferenced ancestor fields are eliminated at compile-time (0 RAM overhead).
// • LAW 3 (`Bypass` Short-Circuit): Piping `A | B` into `fn(Param)`: if `(A | B) <: Param`, passes whole union;
//   if `A <: Param` and `B <: Bypass`, `B` bypasses into `Out | B`; if `B` is NOT `Bypass` -> `E04`!
// • LAW 4 (UFCS & TCO): `x . fn` == `fn(x)` drives IDE autocomplete; tail recursion guarantees 0-alloc loops (TCO).
// =============================================================================


# 0. CORE AXIOMS, LINEAGE, SAFE SLOTS (LAW 1) & DEPTH-INDEXED STREAMS #

Compile {}
Bypass  {}

BrandName String
default_user_name "Alice"

Vehicle         { brand BrandName, max_speed Int 180, tag "standard" }
HasBattery      { battery_kwh Int 75 }
ElectricVehicle Vehicle & HasBattery & { charging_port String "Type2" }
Car             Vehicle   { brand "GenericMotor", max_speed 200 }
SportsCar       Car       { brand "Porsche",      max_speed 300 }
MyCar           SportsCar { brand "CustomPorsche" }
MyGarageCar     MyCar &   { garage_slot 1 }

// Static compile-time prototype & lineage inspection (0 runtime cost):
MyCar.brand                 // "CustomPorsche"
MyCar.SportsCar.brand       // "Porsche"
MyCar.SportsCar.Car.brand   // "GenericMotor"
MyCar.Vehicle.brand         // BrandName
MyGarageCar.0               // "CustomPorsche" (index 0)
MyGarageCar.-1              // 1 (last field `garage_slot`)

mutate_car_demo() [
  my_car (MyGarageCar {})
  upgraded_car (my_car & { max_speed 350, garage_slot 2 }) // Functional update preserving lineage
  my_car.brand "Tesla"                                     // Perceus O(1) in-place mutation by field
  my_car.0     "Rivian"                                    // In-place mutation by index 0
  my_car.-1    42                                          // In-place mutation by index -1
]

// Universal Slot Resolution:
Category { title String }
Age      Int

Box(T)                                        { item T }                               // 1st-order generic
CategorizedUser(TC Category, active_cat TC)   { backup_cat TC, current active_cat }    // Bounded generic + value slot
UserWithAge(Age)                              { new_age age }                          // Law 1c auto-bind `age`
GreetingCard(recipient String)                { message "Hello, {recipient}!" }        // Value slot
BoundedQueue(T, capacity Int)                 { items Tuple(T) (), limit capacity }    // Generic + value slot

is_user_18(Age)                               [ age == 18 ]                            // Law 1c
compare_ages(candidate Age, minimum Age)      [ candidate >= minimum ]                 // Law 1d (2+ same types need names)

WithName { name String }
ReadViaSelf { name "Razomy", get_name() [ self.name ] }                                // Law 1a (`self`)
read_via_type(WithName)                       [ with_name.name ]                       // Law 1c
join_two(first WithName, second WithName)     [ "{first.name} & {second.name}" ]       // Law 1d
read_via_var(w WithName)                      [ w.name ]                               // Law 1d
read_unpacked({ name } WithName)              [ name ]                                 // Law 1d destructuring
extract_name_demo(w WithName)                 [ w . [ _1.name ] ]                      // Law 1b (`_0` root, `_1` == `_` nested)
sum_pair_pipe(pair (Int, Int))                [ pair . [ _1.0 + _1.1 ] ]               // Tuple access in nested stream
do_init()                                     [ "Razomy v5.7 initialized" . Shell.log ]
do_init()


# 1. PRIMITIVES, FIXED-POINT `Money`, RAW & TAGGED STRINGS (`Type"..."`) #

String "", Symbol 'symbol', Int 0, Float 0.0, Bool false, Byte 0x00, Any _

Money(raw String "0.00") {
  scaled (raw . Decimal.parse_scale4)
  from_scaled(val Int) [ Money { scaled val } ]
  +  (other Money)   [ Money.from_scaled(self.scaled + other.scaled) ]
  -  (other Money)   [ Money.from_scaled(self.scaled - other.scaled) ]
  *  (factor Float)  [ Money.from_scaled((self.scaled.to_float * factor).round_to_int) ]
  /  (divisor Float) [ Money.from_scaled((self.scaled.to_float / divisor).round_to_int) ]
  == (other Money)   [ self.scaled == other.scaled ]
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

StringAndLiteralExamples {
  interpolated "Hello, {default_user_name}!\nEscaped \"quotes\" and \{braces\}."
  regex_pat    r"^[^@]+@[^@]+\.[^@]+$"
  nested_json  r1"{"query": "SELECT * FROM \"users\"", "interp": "{ignored}"}"1
  nested_code  r2"Embeds " and "1 and # without lexer conflict!"2

  // `Type"..."` is 100% syntax sugar for `Type("...")`:
  money_val Money"199.99",   currency_val Usd"500.00",         date_val Date"2026-03-30"
  time_val  Time"14:30:00",  duration_val Duration"5m30s",     coord_pair Pair"1920x1080"
  color_val Hex"#2A9D8F",    ip_addr Ip"192.168.0.1:8080",     version_val Version"1.4.2-beta"
  web_url   Url"https://razomy.org", local_file Path"./data/users.json", tax_rate 0.155
}


# 2. TUPLES `()`, FORMS `{}`, MAPS & TYPE ALGEBRA (`&`, `|`, `/`) #

Point2D   (Float, Float)
origin    (0.0, 0.0)
Status    'draft' | 'published' | 'archived'
Primitive String | Int | Float | Bool | Byte | Money

Identifiable { id String }
Auditable    { created_at Int, updated_at Int }
User         Identifiable { id String, name String, age Int, role 'admin' | 'manager' | 'user' | 'guest', password_hash String }
Cloud        Identifiable { id String, name String, region String }

PublicUser        User / User.password_hash                      // Remove field by path
AnonymousUsr      PublicUser / 'name'                            // Remove field by symbol
UserAndCloud      User & Cloud                                   // Merge (collides on `id`, `name`)
UserWithCloudName UserAndCloud / User.name                       // Disambiguates `.name` -> `Cloud.name`
CloudWithUserName UserAndCloud / Cloud.name                      // Disambiguates `.name` -> `User.name`
NoNameEntity      UserAndCloud / 'name'                          // Removes `name` from all ancestors
RestoredUser      UserAndCloud / Cloud                           // Removes entire `Cloud` ancestor slice
GuestUser         PublicUser / PublicUser.role & { role 'guest' }

Map(K, V) { (K) V, get(m Map(K, V), key K) [ (m, key) . native_map_get ] }
Set(T)    { (T) Bool }

config_map     { "host" "localhost", "port" "8080" }
updated_config config_map & { "port" "9090", "tls" "true" } / "host"


# 3. UPDATING DATA: DEEP IN-PLACE REBINDING (PERCEUS O(1)) vs `&` MERGE #

Order      { id String, amount Int }
Department { title String, budget Int }
Company    { name String, departments Tuple(Department) }

update_order_demo() [
  raw_order Order { id "o1", amount 100 }
  counter 0
  counter (counter + 1)                                          // 1. Direct variable rebind (space before `(`!)
  raw_order.amount 14                                            // 2. Field path in-place mutation
  corp Company { name "Acme", departments (Department { title "R&D", budget 1000 }) } // Auto-lifts single tuple item
  corp.departments.0.budget 2500                                 // 3. Deep lens path + index in-place mutation
  raw_order.1 20                                                 // 4. Positional field index mutation
  next_order (raw_order & { amount 25 })                         // 5. Functional `&` merge
  next_order (next_order .& { amount 50 })                       // 6. Enrich pipe `.&` rebind
]


# 4. THE 7 CANONICAL COMBINATIONS OF `&` & UFCS EXTENSION METHODS #

// 1. Form & Form (Trait impl / Inheritance):
Printable     { format() String }
PrintableUser User & Printable { format() [ "User({self.name}, id={self.id})" ] }

// 2. Type & Stream (Refinement Type):
PositiveInt Int & [ _ > 0 ]
AdultUser   User & [ _.age >= 18 ]

// 3. Function & Form (Partial Application / DI Override):
connect(host String "localhost", port Int 8080, tls Bool false) [ "Connecting to {host}:{port} (TLS: {tls})" ]
secure_connect connect & { port 9090, tls true }

// 4. Stream & Stream (Sequential Composition):
clean_str [ _ . trim ] & [ _ . to_upper ]

// 5. Value/Member & TagForm (Metadata / Visibility):
Public {}, Private {}
secret_key "xyz987" & Private

// 6. Type & Trait(Self) (F-Bounded Polymorphism):
Comparable(Other) { < (other Other) Bool }
SortableUser User & Comparable(User) { < (other User) [ self.age < other.age ] }

// 7. Type & Bypass (Short-Circuit Outcome Marker for Law 3):
None     {} & Bypass
NotFound { entity String, id String } & Bypass

// Open UFCS Extension Method:
greet(User) [ "Welcome, {user.name}!" ]


# 5. EAGER `()` vs LAZY `[]` FIELDS & OPERATOR OVERLOADING #

MathOps(a Int, b Int) {
  add (a + b), sub (a - b), mul (a * b), div (a / b), mod (a % b), pow (a ** b)
  lazy_div [ self.a / self.b ] // 0-byte static inline getter, keeps RefCount == 1
  bit_and (a . bit_and(b)), bit_or (a . bit_or(b)), bit_xor (a . bit_xor(b)), shift_l (a << 2), shift_r (a >> 2)
}

Vec2 {
  x Float, y Float
  +  (o Vec2)  [ Vec2 { x (self.x + o.x), y (self.y + o.y) } ]
  -  (o Vec2)  [ Vec2 { x (self.x - o.x), y (self.y - o.y) } ]
  *  (s Float) [ Vec2 { x (self.x * s),   y (self.y * s) } ]
  /  (s Float) [ Vec2 { x (self.x / s),   y (self.y / s) } ]
  == (o Vec2)  [ (self.x == o.x) && (self.y == o.y) ]
}

Shape { vertices Tuple(Vec2), / (cutter Shape) [ (self, cutter) . native_csg_subtract ] }

LogicOps(x Bool, y Bool, val Int) {
  and_op (x && y), or_op (x || y), not_op (!x), eq (val == 42), neq (val != 0)
  gt (val > 10), gte (val >= 10), lt (val < 100), lte (val <= 100), in_range (val == (1..100))
}

string_ops(first String, last String) [ full "{first} {last}", full . trim . to_upper . split(" ") ]


# 6. IMPERATIVE MAPPING (GUARD ROUTERS & EARLY `Break` IN `fold`) #
// if/switch -> `x . { Pat [A], _ [B] }` | loop/break -> `fold` + `Break` | while -> tail-recursion | null/try -> `Union & Bypass`

Break(T) { value T, label Symbol 'default' }

classical_patterns_demo(numbers Tuple(Int), limit Int) [
  limit . {
    [ _ <= 0 ] [ 0 ]
    _ [
      numbers . fold(0, (acc Int, n Int) [
        (acc + n) . {
          [ _ > limit ] [ Break { value _ } ]
          _             [ _ ]
        }
      ])
    ]
  }
]


# 7. SLOTS, DEFAULTS, DI & PIPELINES (`.`, `.>`, `.&`, `.?`, `.|`) #

Shell { log(msg String) [ msg . native_print ], exec(cmd String) [ cmd . native_exec ] }
Cart  { cart_id String, amount Int }

Db {
  find_user(id String) User | NotFound [ User { id id, name "Alice", age 25, role 'user', password_hash "xyz" } ]
  find_cart(Identifiable)              [ Cart { cart_id "c_for_{identifiable.id}", amount 2500 } ]
}

default_conn   connect()
partial_conn   connect("db.internal")
named_call     connect(port 9090, tls true)
custom_connect connect & { port 9090, tls true }
active_conn    custom_connect()

FindOptions { case_sensitive false, limit 0, last false }
find(collection, target, opts FindOptions) [ collection . native_find(target, opts.case_sensitive, opts.limit, opts.last) ]
search_users(names) [ names . find("Alice", FindOptions & { case_sensitive true, limit 5 }) ]

finalize_bill(ctx User & Cart) [
  ctx & {
    discount    [ self.amount . { [ _ >= 2000 ] [ 300 ], [ _ >= 1000 ] [ 100 ], _ [ 0 ] } ]
    final_price [ self.amount - self.discount ]
  }
  .> [ "Receipt for {_1.name}: {_1.final_price} USD (Discount: {_1.discount})" . Shell.log ]
]

process_order_pipeline(user_id String, db Db) [
  user_id . db.find_user .? [ _.age >= 18 ] .& db.find_cart . finalize_bill
]

// Multi-query `do`-notation via `.&` (short-circuits on `NotFound` via Law 3):
load_user_and_friend(uid1 String, uid2 String) [
  { first_id uid1, second_id uid2 }
  .& [ _.first_id . Db.find_user . [ { u1 _ } ] ]
  .& [ _.second_id . Db.find_user . [ { u2 _ } ] ]
  .  [ "{_.u1.name} is friends with {_.u2.name}" ]
]


# 8. REFINEMENT TYPES, ROUTERS, COLLECTIONS & LINEAGE MIGRATION #

is_adult(User)   [ user.age >= 18 ]
is_minor(User)   [ user.age < 18 ]
is_manager(User) [ user.role == ('admin' | 'manager') ]
non_empty        [ _.length > 0 ]
EmailStr         String & [ _ . Regex.matches(r"^[^@]+@[^@]+\.[^@]+$") ]

Adult   User & is_adult
Minor   User & is_minor
Manager User & is_manager

classify_account(account User) [
  account . {
    Manager [ "Dashboard: {manager.name}" . Shell.log ]
    Adult   [ "Adult: {adult.name}" . Shell.log ]
    Minor   [ "Minor: {minor.name}" . Shell.log ]
    _       [ "Unknown" . Shell.log ]
  }
]

Circle { radius Float }, Rect { width Float, height Float }
Geometry Circle | Rect
area(g Geometry) [ g . { Circle [ 3.14159 * circle.radius * circle.radius ], Rect [ rect.width * rect.height ] } ]

Item { name String, price Int, in_stock Bool }
order_list (
  Item { name "Laptop", price 1200, in_stock true }
  Item { name "Mouse",  price 25,   in_stock false }
)

calculate_total_in_stock(items Tuple(Item)) [
  items . filter([ _.in_stock == true ]) . map([ _.price ]) . fold(0, (acc Int, p Int) [ acc + p ])
]

format_numbered_catalog(items Tuple(Item), discounts Tuple(Int)) [
  items . zip(discounts) . enumerate . map((idx Int, (item Item, disc Int)) [ "#{idx}: {item.name} -> {item.price - disc}" ])
]

fizz_buzz(limit Int) [
  (1..limit) . map((n Int) [ (n % 3, n % 5) . { (0, 0) [ "FizzBuzz" ], (0, _) [ "Fizz" ], (_, 0) [ "Buzz" ], _ [ "{n}" ] } ])
]

// Law 2 Lineage Disambiguation & Migration:
UserOrder      User & Order
UserOrderClean UserOrder / Order.id
process_combined(ctx) [
  ctx . { UserOrder [ "User ID: {user_order.User.id}, Order ID: {user_order.Order.id}" . Shell.log ] }
]

ClientAccount { account_id String, name String, is_adult Bool }
migrate_user(User) [ ClientAccount { account_id user.id, name user.name, is_adult (user.age >= 18) } ]


# 9. REACTIVE FORMS (`self`), SIGNALS & ONTOLOGICAL TYPESTATES (LAW 3 & 4) #

Invoice {
  price Money"100.00", quantity 3.0, tax_rate 0.20
  subtotal [ self.price * self.quantity ]
  tax      [ self.subtotal * self.tax_rate ]
  total    [ self.subtotal + self.tax ]
}
order_1 Invoice & { quantity 5.0 } // order_1.total == Money"600.00"

count_sig   Signal(2)
doubled_sig [ count_sig.get() * 2 ]
run_signal_demo() [ Effect([ "Count: {count_sig.get()}, Doubled: {doubled_sig()}" . Shell.log ]), count_sig.set(10) ]

// Typestates & Negative Constraints (`Order / Cancelled`):
Reserved { reserved_at Int }, Paid { transaction_id String }, Cancelled { cancelled_at Int }
OutOfStock { reason String } & Bypass, NoMoney { reason String } & Bypass

reserve_stock(o Order / Cancelled) [
  o.amount . { [ _ > 0 ] [ o & Reserved { reserved_at 1620000000 } ], _ [ OutOfStock { reason "Empty" } ] }
]
charge_payment(o Order & Reserved) [
  o.amount . { [ _ < 50000 ] [ o & Paid { transaction_id "tx_001" } ], _ [ NoMoney { reason "No funds" } ] }
]
execute_fulfillment(initial_order Order / Cancelled) [ initial_order . reserve_stock . charge_payment ]
ship_to_customer(order Order & Reserved & Paid)      [ "Shipping {order.id} ({order.transaction_id})" . Shell.log ]
audit_any_outcome(outcome (Order & Reserved & Paid) | OutOfStock | NoMoney) [ "Audit: {outcome}" . Shell.log ]


# 10. 1ST/2ND/3RD-ORDER GENERICS (HKT), RECURSIVE `Self` & CATAMORPHISMS #

Zero {}, Succ(N) { prev N }, One Succ(Zero), Two Succ(One), FixedVector(T, N) { data Tuple(T), length N }
Json String | Int | Float | Bool | None | Tuple(Self) | Map(String, Self)

Cons(T) { head T, tail List(T) }, List(T) None | Cons(T)
RoseTree(T) { value T, children Tuple(Self) () }
Leaf {}, Node(T) { value T, left Tree(T), right Tree(T) }, Tree(T) Leaf | Node(T)

sort(items List(Comparable))                  [ items . native_sort_by((a, b) [ a < b ]) ]
sort_users(users List(User & Comparable(User))) [ users . sort ]

NestedBox(T) Box(Box(T))
Functor(F(_)) { fmap(A, B, fa F(A), f (A) [B]) F(B) }
Monad(M(_)) & Functor(M) { pure(A, a A) M(A), flat_map(A, B, ma M(A), f (A) [M(B)]) M(B) }

Some(T) { value T }, Option(T) Some(T) | None
OptionMonad Monad(Option) {
  fmap(A, B, fa Option(A), f (A) [B])            [ fa . { Some [ Some { value (some.value . f) } ], None [ None ] } ]
  pure(A, a A)                                   [ Some { value a } ]
  flat_map(A, B, ma Option(A), f (A) [Option(B)])[ ma . { Some [ some.value . f ], None [ None ] } ]
}

OptionT(M(_), A)      { run M(Option(A)) }
MonadTrans(T(_(_), _)) { lift(M(_) Monad, A, ma M(A)) T(M, A) }

first_adult(users List(User)) [ users . take_while([ _.age > 0 ]) . find([ _.age >= 18 ]) ]
list_map(A, B, l List(A), action (A) [B]) List(B) [
  l . { None [ None ], Cons [ Cons { head (cons.head . action), tail (cons.tail . list_map(action)) } ] }
]

Num(T) { value T }, Add(L, R) { left L, right R }, Mul(L, R) { left L, right R }
Expr   Num(Int) | Add(Expr, Expr) | Mul(Expr, Expr)
cata(e Expr, alg) [
  e . {
    Num [ alg.on_num(num.value) ]
    Add [ alg.on_add(add.left . cata(alg), add.right . cata(alg)) ]
    Mul [ alg.on_mul(mul.left . cata(alg), mul.right . cata(alg)) ]
  }
]
CalcAlgebra { on_num(v Int) [ v ], on_add(a Int, b Int) [ a + b ], on_mul(a Int, b Int) [ a * b ] }


# 11. CONCURRENCY (`.|`, CHANNELS, ATOMS, FSM), RESOURCES & HTTP ROUTING #

load_dashboard_data(user_id String) [
  user_id .| {
    profile [ Url"https://api.ex/users/{user_id}" . Http.get ]
    recs    [ Url"https://api.ex/recs/{user_id}" . Http.get ]
    ads     [ Url"https://api.ex/ads" . Http.get . timeout(Duration"2s") ]
  }
]

Channel(T) { send(val T) [ (self, val) . native_chan_send ], recv() [ self . native_chan_recv ] }
worker_loop(jobs Channel(String), results Channel(Int)) [
  jobs.recv() . { None [ "Closed" . Shell.log ], Some [ some.value . length . results.send, worker_loop(jobs, results) ] }
]

CounterState { count Int }, Increment { by Int }, Decrement { by Int }, Reset {}
CounterMsg   Increment | Decrement | Reset
update_counter(s CounterState, msg CounterMsg) [
  msg . { Increment [ CounterState { count (s.count + increment.by) } ], Decrement [ CounterState { count (s.count - decrement.by) } ], Reset [ CounterState { count 0 } ] }
]
Atom(T) { value T, read() [ self . native_atom_read ], swap(tr) [ (self, tr) . native_atom_swap ] }
global_counter Atom(CounterState) { value CounterState { count 0 } }
modify_global_counter() [ global_counter.swap((s CounterState) [ update_counter(s, Increment { by 5 }) ]) ]

Disconnected { reason String }, Connecting { host String, attempt Int }, Connected { host String, session_id String }
ConnState Disconnected | Connecting | Connected
ConnectCmd { host String }, HandshakeOk { session_id String }, NetworkError { reason String } & Bypass, DisconnectCmd {}
ConnEvent ConnectCmd | HandshakeOk | NetworkError | DisconnectCmd

step_connection(state ConnState, event ConnEvent) ConnState [
  (state, event) . {
    (Disconnected, ConnectCmd) [ Connecting { host connect_cmd.host, attempt 1 } ]
    (Connecting, HandshakeOk)  [ Connected { host connecting.host, session_id handshake_ok.session_id } ]
    (Connecting, NetworkError) [ Disconnected { reason network_error.reason } ]
    (Connected, DisconnectCmd) [ Disconnected { reason "User closed" } ]
    _                          [ state ]
  }
]

// Scoped Resource Management & Ports:
FileHandle { fd Int, path Path }, IoError { msg String } & Bypass
Fs {
  open(path Path)   [ path . native_fs_open ]
  read(FileHandle)  [ file_handle.fd . native_fs_read ]
  close(FileHandle) [ file_handle.fd . native_fs_close ]
  with_file(path Path, action) [ path . Fs.open . { FileHandle [ file_handle . action .> [ file_handle . Fs.close ] ] } ]
}
read_config(path Path) [ Fs.with_file(path, (f FileHandle) [ f . Fs.read . Json.parse ]) ]
sync_remote_backup()   [ Url"https://api.razomy.org/v1/users" . Port.read . Port.write(Path"./backup/users.json") ]

// Deep Pattern Routing:
HttpRequest  { method 'GET' | 'POST' | 'PUT' | 'DELETE', path Tuple(String), headers Map(String, String), body String }
HttpResponse { status Int, body String }
Unauthorized { reason String } & Bypass

auth_middleware(req HttpRequest) [
  req.headers . Map.get("Authorization") . {
    Some [ req & { auth_user (some.value . Db.find_user_by_token) } ]
    None [ Unauthorized { reason "Missing Auth" } ]
  }
]
api_router(req HttpRequest) [
  req . {
    { method 'GET', path ("health") }    [ HttpResponse { status 200, body "OK" } ]
    { method 'GET', path ("users", id) } [
      req . auth_middleware . {
        Unauthorized [ HttpResponse { status 401, body unauthorized.reason } ]
        HttpRequest  [ HttpResponse { status 200, body (id . Db.find_user . Json.encode) } ]
      }
    ]
    _ [ HttpResponse { status 404, body "Not Found" } ]
  }
]


# 12. BUILT-IN DIALECTS: `Parse` (PEG STREAMS) & `Ui` (VIEW / DRAW) #

Parse {
  end 'eof', match(pat) [ _ . native_parse_match(pat) ], one_or_more(r) [ _ . native_parse_one_or_more(r) ]
  repeat(b, r) [ _ . native_parse_repeat(b, r) ], skip(n Int) [ _ . native_parse_skip(n) ]
  to(delim) [ _ . native_parse_until(delim) ], copy(f Symbol, t) [ _ . native_parse_capture(f, t) ]
  run(input String, rules) [ input . rules ]
}
digit "0".."9", alpha "a".."z" | "A".."Z"
EmailGrammar String & [ _ . Parse.run(Parse.one_or_more(alpha) & Parse.match("@") & Parse.one_or_more(alpha) & Parse.match(".") & Parse.repeat(2..6, alpha)) ]
LogEntry { date String, level String, msg String }
parse_log_line(line String) [
  line . Parse.run(Parse.copy('date', Parse.to(" ")) & Parse.skip(1) & Parse.match("[") & Parse.copy('level', Parse.to("]")) & Parse.skip(2) & Parse.copy('msg', Parse.to(Parse.end))) . { LogEntry [ log_entry ] }
]

Ui { view(spec) [ spec . native_ui_view ], draw(children Tuple) [ children . native_ui_draw ] }
counter_app(state Atom(CounterState)) [
  Ui.view({
    title "Razomy Counter", size Pair"1920x1080"
    body (
      heading("Count: {state.read().count}")
      row(
        button("Inc",   [ state.swap((s CounterState) [ update_counter(s, Increment { by 1 }) ]) ]),
        button("Reset", [ state.swap((s CounterState) [ update_counter(s, Reset {}) ]) ])
      )
      canvas(Pair"200x100", [ Ui.draw(pen(Hex"#FF0000"), fill_pen(Hex"#00FF00"), circle((50, 50), 40)) ])
    )
  })
]


# 13. METAPROGRAMMING (`Compile &`), `Meta.rewrite`, IMPORTS, TESTS & AI AGENT #

Meta {
  type_of(e)          [ e . native_meta_type_of ]            & Compile
  doc_of(e)           [ e . native_meta_doc_of ]             & Compile
  fields_of(e)        [ e . native_meta_fields_of ]          & Compile
  steps_of(s)         [ s . native_meta_steps_of ]           & Compile
  map_fields(f, m)    [ (f, m) . native_meta_map_fields ]    & Compile
  compose(s)          [ s . native_meta_eval_parens ]        & Compile
  rewrite(node, rules) [
    walked (node . {
      Form   [ node . Meta.map_fields((k, v) [ (k, v . Meta.rewrite(rules)) ]) ]
      Tuple  [ node . map([ _ . Meta.rewrite(rules) ]) ]
      Stream [ node . Meta.steps_of . map([ _ . Meta.rewrite(rules) ]) . fold([ _ ], (acc, step) [ acc & step ]) ]
      _      [ node ]
    })
    walked . (rules & { _ [ walked ] })
  ] & Compile
}

build_multiplier(factor Int) [ Meta.compose([ _ * (factor * 10) ]) ] & Compile
trace_execution(block) [ block . Meta.steps_of . map((s) [ s & [ _ .> [ "Step: {_}" . Shell.log ] ] ]) . fold([ _ ], (a, s) [ a & s ]) ] & Compile

derive_json(TargetForm) [
  TargetForm & {
    to_json() [
      inner (self . Meta.fields_of . map((f) [ "\"{f.name}\": {f.value . Json.format_value}" ]) . join(", "))
      "\{ {inner} \}"
    ]
  }
] & Compile

# Catalog product with auto-derived JSON and homoiconic doc-comments. #
Product (Compile & [ { id Int, title String } . derive_json ])

inspect_entity(entity) [
  "Type: {entity . Meta.type_of} (Doc: {entity . Meta.doc_of})" . Shell.log
  entity . Meta.fields_of . for_each((f) [ "Field {f.name} ({f.doc}): {f.type} = {f.value}" . Shell.log ])
] & Compile

// Homoiconic AST & Scene Rewriting (`{(K) V}` wraps compound AST/path keys in parens):
UpgradedSystem (Compile & [
  ComplexSystem . Meta.rewrite({
    (calc(_))   [ (x Float) [ x * 1.20 ] ]
    (Shell.log) [ Audit.write ]
  })
])

refactor_id_to_my_id(tree) [
  tree . Meta.rewrite({
    { i_id_attribute val } [ { my_id (MyAttributeForCurrentContext & val) } ]
    (self.i_id_attribute)  [ self.my_id ]
    (_.i_id_attribute)     [ _.my_id ]
    i_id_attribute         [ my_id ]
  })
] & Compile

Cuboid { size (1.0, 1.0, 1.0), anchor (0.5, 0.5, 0.5), position (0.0, 0.0, 0.0), pivot (0.5, 0.5, 0.5), rotation (0.0, 0.0, 0.0), scale (1.0, 1.0, 1.0) }
PlayButton     Cuboid & { label "Play" }
FlatPlayButton (PlayButton / Cuboid) & { canvas_2d_rect (0.5, 0.35, 0.15, 0.15) }
RoundButton    Cuboid & { name String, children {} }
NativeMobileButton { id String, rect Any, asset Any }

optimize_graph(root) [
  root . Meta.rewrite({
    { children { background { image bg, rounded r1 }, foreground { image fg, rounded r2 }, icon ico } } [
      { children { baked_layer { combined_images (bg, fg), rounded r1 }, icon ico } }
    ]
  })
] & Compile

render_for_mobile(graph) [
  graph . Meta.rewrite({
    (RoundButton & { name n, size s, position p, children { icon { icon_name ico } } }) [
      NativeMobileButton { id n, rect (p, s), asset ico }
    ]
  })
] & Compile

build_screen(raw_tree) [ raw_tree . refactor_id_to_my_id . optimize_graph . render_for_mobile ] & Compile

// Imports & `/ Private` Encapsulation:
Base root.base, (String, Int) root.base, _ root.base, _ base, _ Path"../../../base", Http std.net.http, Sqlite std.db.sqlite
AuthInternal {
  salt "x9f2a" & Private
  hash(raw String)         [ "{raw}_{self.salt}" . native_sha256 ] & Private
  verify(u User, p String) [ (p . self.hash) == u.password_hash ] & Public
}
AuthModule AuthInternal / Private

// Testing & Capstone Autonomous AI Agent (showing `_0`, `_1`, `_2` depth indexing):
TestPassed { name String }, TestFailed { name String, expected Any, actual Any }, TestResult TestPassed | TestFailed
assert_eq(name String, exp, act) [ (exp == act) . { true [ TestPassed { name name } ], false [ TestFailed { name name, expected exp, actual act } ] } ]

CompileError { code Int, log String } & Bypass, BuildSuccess { status String, artifact String }
BuildResult  BuildSuccess | CompileError
FatalError   CompileError & [ _.code >= 500 ], Warning CompileError & [ _.code < 500 ]
AiQuery      { system_prompt String, user_prompt String }, AiResponse { diff String, explanation String }
AgentState   { err CompileError, retries Int, applied_patches Tuple(String) () }
Ai           { call(q AiQuery) AiResponse [ q . native_ai_llm_request ] }
run_build()  BuildResult [ "cargo build --release" . Shell.exec . native_parse_build_result ]

agent_loop(state AgentState) [ // `_0` = root stream
  state.retries . {
    [ _ == 0 ] [ "Retry limit reached." . Shell.log ]
    _ [                        // `_1` = router branch stream
      ai_resp (
        AiQuery { system_prompt "Fix error with git diff.", user_prompt "Log: {state.err.log}" }
        . Ai.call
        .> [ "AI solution: {_2.explanation}" . Shell.log ] // `_2` (or `_`) = tap stream
        .> [ "git apply {_2.diff}" . Shell.exec ]
      )
      run_build() . {
        BuildSuccess [ "Build succeeded!" . Shell.log ]
        CompileError [
          agent_loop(state & { err compile_error, retries (state.retries - 1), applied_patches (state.applied_patches . append(ai_resp.diff)) })
        ]
      }
    ]
  }
]

main() [
  run_build() . {
    FatalError   [ agent_loop(AgentState { err fatal_error, retries 3 }) ]
    Warning      [ "cargo fix --allow-no-vcs" . Shell.exec ]
    BuildSuccess [ "Ready: {build_success.artifact}" . Shell.log ]
  }
]


# 14. COMPILER ERROR CATALOG (`E01`..`E07`) & CANONICAL SOLUTIONS #

// • E01_AMBIGUOUS_CALL_OR_BIND: `area (Circle { radius 5.0 })` -> `area` is a Function in scope, cannot bind or call with space!
//   Fix: `area(Circle { radius 5.0 })` (tight call) or `circle_area (Circle { radius 5.0 } . area)` (new var bind).
// • E02_DUPLICATE_UNNAMED_TYPE: `transfer(Account, Account, Money)` or `fn(User & Cart)` -> Law 1c only binds single atomic types!
//   Fix: Name duplicate/compound slots: `transfer(src Account, dst Account, Money)` (`Money` still auto-binds `money`).
// • E03_TYPE_PARAM_SHADOWS_TYPE: `Wrapper(User) { item User }` -> `User` is an existing Type, not an unbound Type Param!
//   Fix: Use unbound identifier `Wrapper(Elem) { item Elem }` or bounded slot `UserWrapper(U User, cur U) { active U cur }`.
// • E04_UNHANDLED_UNION_BRANCH: Piping `User | Company | NotFound` into `send_email(User)` -> `NotFound` (`<: Bypass`) bypasses,
//   but `Company` is a domain type (NOT `& Bypass`)! Fix: Route explicitly `. { User [...], Company [...] }`.
// • E05_AMBIGUOUS_LINEAGE_FIELD: Reading `entity.name` on `User & Cloud` -> `name` collides in both ancestors!
//   Fix: Disambiguate at read site `entity.User.name` or subtract at type level `entity / Cloud.name`.
// • E06_IMMUTABLE_TYPE_BINDING: `Car.max_speed 500` -> `PascalCase` Prototypes/Types are strictly immutable!
//   Fix: Instantiate into `snake_case` first: `my_car (Car {})`, then mutate `my_car.max_speed 500`.
// • E07_STATIC_ASSERT_FAILED: Enforcing compile-time architectural invariants via `Compile &`:
VerifyPublicUserSafety (Compile & [
  (PublicUser . Meta.fields_of . map([ _.name ]) . contains("password_hash")) . {
    true  [ Compile.error("SECURITY VIOLATION: PublicUser leaked password_hash!") ]
    false [ true ]
  }
])