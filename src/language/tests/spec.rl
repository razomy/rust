// =============================================================================
// RAZOMY LANGUAGE V4.7: HOMOICONIC AXIOMATICS OF THREE BRACKETS
// =============================================================================
// 1. STRICT SEPARATION OF THREE BRACKETS (NO ROLE OVERLAP):
// {} — Form   (Key-Value: Data structures, Routing tables, Pattern Matching, Modules, Maps).
//             Parens on a key position `{ (K) V }` define a parameterized/computed Map key.
// [] — Stream (Function body: Lazy sequence of operations, Predicates, Code blocks, Dialects).
// () — Tuple  (Ordered array of data, Tuples, Incoming argument slots, Value isolation in `{}`).
//             • Functions & Lambdas: `Name(Args) [Body]` defines a named function (with optional
//               default values `Name(K Default)`), `(Args) [Body]` defines an anonymous lambda,
//               and `Name(Args) ReturnType` declares a method contract signature inside a Form.
//             • Tuple-Argument Duality: An incoming argument list `(A, B, C)` IS a Tuple `()`.
//               A function expecting a single `Tuple` slot `Fn(Children Tuple)` directly accepts
//               multiple comma-separated arguments `Fn(A, B, C)` without double parens `(())`.
//
// 2. PAIRED QUOTES, THE SEPARATOR RULE & SPACE-DELIMITED LEXER:
// "..." — Interpolated String: evaluates `{Expr}` inside ("Hello, {Name}") and supports `\` escapes
// '...' — Paired Symbol Literal: exact constant tag/field-key without interpolation ('draft', 'Name')
// ','   — Inline Separator: separates sibling elements/arguments on the same line
// '\n'  — Line Separator: separates sibling fields in `{}` or sequential statements in `[]`
// '#'   — Block Comment (when followed by whitespace/newline) or Hex Color literal (`#2A9D8F`)
// ' '   — Pair & Token Separator:
//         (a) Universal Binding Axiom (`Key Value` or Tuple destructuring `(A, B) Source`):
//             binds a field in `{}`, a module member at file root, or a local variable inside `[]`.
//         (b) Space-Delimited Lexer Rule: All binary operators (`+`, `-`, `*`, `/`, `%`, `**`,
//             `<<`, `>>`, `==`, `!=`, `>`, `<`, `>=`, `<=`, `&&`, `||`, `.`, `&`, `|`) MUST be
//             surrounded by spaces (`A - B`, `A / B`, `A . B`), while unary `!` prefixes tightly (`!X`).
//             Any contiguous token without spaces (`2026-03-30`, `1920x1080`, `https://api.ex/{Id}`,
//             `%./path`, `$199.99`) is parsed in O(1) as a single self-typed literal (supporting `{Expr}`
//             interpolation inside contiguous URL/File literals)!
//
// 3. COMPLETE OPERATOR & ALGEBRAIC OVERLOADING REFERENCE:
// 'A.B'     (no spaces)   — Tight Path Access: field, module, ancestor `A.Parent.Field`, or index `A.0`, `A.-1`
// 'A . B'   (with spaces) — Dataflow Pipeline: passes `A` left-to-right (`A . B` is sugar for `B(A)`,
//                           and `A . B(C, D)` is UFCS sugar for `B(A, C, D)`)
// 'A .> B'                — Tap Pipeline: executes side-effect `B(A)` and passes original `A` further
// 'A .& B'                — Enrich Pipeline: calls `B(A)` and merges `&` result with left context `A`
// 'A .? B'                — Safe/Filter Pipe: passes `A` further only if predicate/type `B` matches, else `None`
// 'A .| {}'               — Parallel Pipeline: concurrent fan-out execution of branches in `{}`, merged via `&`
// 'A & B'                 — Algebraic Intersection / Merge:
//                           • Form & Form     -> Structural merge with full Lineage preservation
//                           • Type & Stream   -> Refinement Type (type constrained by predicate `[]`)
//                           • Stream & Stream -> Sequential function composition (`[ A . B ]`)
//                           • Any & Form      -> Metadata/visibility trait attachment (`Val & Private`)
//                                                or function named-argument override (`Fn & { Port 9090 }`)
// 'A | B'                 — Union: algebraic sum of types, patterns, or outcomes
// 'A / B'                 — Algebraic Subtraction (for Forms/Maps/Types) or Division (for Numbers):
//                           • Form / FieldPath -> Removes specific field (`User / User.PasswordHash`)
//                           • Form / 'Key'     -> Removes field key from all ancestors (`Combined / 'Name'`)
//                           • Map  / KeyVal    -> Removes entry key from a Map (`ConfigMap / "host"`)
//                           • Form / Ancestor  -> Removes entire ancestor slice (`UserAndCloud / Cloud`)
//                           • Union / Type     -> Removes branch from Union or restricts slot (`Order / Cancelled`)
// 'A..B'                  — Range: sequence generator (`1..10`, `"a".."z"`) or inclusion predicate `Val . (A..B)`
// '_'                     — Current Stream Context: references the active piped value in `[]` or default branch in `{}`
// 'Self'                  — Current Form Context: references the enclosing Form `{}` instance inside its methods
// '{K [E]}'               — Reactive Field Sugar: a parameterless stream `[E]` (without free `_`) inside `{}`
//                           auto-recomputes its formula whenever sibling/source fields change
//
// 4. THREE IRON LAWS OF RAZOMY:
// - LAW 1 (SCOPED AUTO-UNPACKING & `Self` vs `_`):
//   Fields of a Form are automatically projected into scope ONLY when the context is anonymous/single:
//   (a) Inside Form definitions `{}`: reactive fields and methods reference sibling fields directly by name,
//       while the enclosing Form instance itself is referenced via `Self` (keeping `_` free for streams).
//   (b) Upon anonymous Type/Structural/AST pattern match in a router `{ Circle [ 3.14 * Radius * Radius ] }`,
//       including variable capture inside destructured Tuple patterns `{ Path ("users", Id) [ Id ] }`.
//   (c) In an unnamed typed function slot `Fn(Order & Paid) [ TransactionId ]`.
//   Named parameters `Fn(U User, O Order)` NEVER auto-unpack (`U.Name`, `O.Amount`), preventing shadowing!
// - LAW 2 (LINEAGE NAMESPACING): Upon name collision during merge `A & B`, unique fields are accessed
//   flatly, while colliding fields require specifying the ancestor path (`A.Name`, `B.Name`) until resolved via `/`.
// - LAW 3 (OUTCOME BYPASS / RAILWAY-FREE): If a union type `A | B` (including `None`) is piped into a
//   function expecting `A`, branch `B` automatically bypasses the function without invocation, accumulating
//   in the final Union return type of the pipeline.
// =============================================================================


#
0. CORE AXIOMS & LINEAGE (INHERITANCE & TYPE SPECIFICATION HISTORY)
#

// Single line comments
#
Multiline / block comments (when '#' is followed by whitespace or newline)
#

// Axiom 1: [UniqName] [VALUE]
Name String

// Axiom 2: [UniqName] { [KEY] [VALUE] }
Name {
  First String
}

// Axiom 3: [UniqName] { [KEY] [VALUE] } & [OTHER1] & [OTHER2]
HasParentName {
  ParentName String
}

Name {
  First String
} & HasParentName

// Axiom 4: [UniqName] [BaseType] { [KEY] [Override_VALUE] }
Jonn String

JonnName HasParentName {
  ParentName Jonn
}

JonnDoy Jonn

JonnDoyName1 JonnName {
  ParentName JonnDoy
}

JonnDoyName2 JonnName {
  ParentName JonnDoy
} & { ParentName "JonnDoyBoy" }

// Tight dot '.' access: Active Values vs Ancestor Specification (Lineage)
JonnDoyName2.ParentName                        // "JonnDoyBoy" (active overridden value)
JonnDoyName2.JonnName.ParentName               // JonnDoy (inner value of JonnName ancestor)
JonnDoyName2.JonnName.HasParentName.ParentName // Jonn (deeper ancestor value)
JonnDoyName2.HasParentName.ParentName          // String (base specification type)

JonnDoyName3 JonnDoyName2 & { D "d" }
JonnDoyName3.ParentName                        // "JonnDoyBoy"
JonnDoyName3.JonnDoyName2.JonnName             // Access to entire ancestor slice

JonnDoyName3.0                                 // Access by field index 0 (ParentName)
JonnDoyName3.-1                                // Access by field index -1 from end (D)


#
1. PRIMITIVES, PAIRED SYMBOLS & RICH HUMAN LITERALS
#
// Because binary operators require surrounding spaces, contiguous tokens without spaces
// are parsed in O(1) as first-class self-typed literals (eliminating stringly-typed code).

String      ""
Symbol      'symbol'
Int         0
Float       0.0
Bool        false
Byte        0x00
Money       $0.00
File        %./
Any         _

HumanLiterals {
  MoneyVal    $199.99              // Fixed-precision Money literal
  CurrencyVal USD$500.00           // ISO Currency literal
  PercentVal  15.5%                // Percentage literal ($100.00 * 15.5% == $15.50)
  DateVal     2026-03-30           // Native ISO Date (no spaces around `-`)
  TimeVal     14:30:00             // Native Time
  DurationVal 5m30s                // Native Duration
  CoordPair   1920x1080            // 2D Pair / Resolution literal
  ColorVal    #2A9D8F              // Hex Color literal (RGB/RGBA)
  IpAddr      192.168.0.1:8080     // IPv4/Port network literal
  VersionVal  1.4.2-beta           // Semantic Version literal (1.4.2 > 1.3.0)
  EmailAddr   admin@razomy.org     // Native Email literal
  WebUrl      https://razomy.org   // Native URL literal (no spaces around `/`)
  LocalFile   %./data/users.json   // Native File path literal
}


#
2. TUPLES (), FORMS {}, MAPS & TYPE ALGEBRA (&, |, /)
#

// 1. Tuples are expressed via parens ()
Point2D (Float, Float)
Origin  (0.0, 0.0)

// 2. Type Unions (Sum Types) with paired symbols '...'
Status    'draft' | 'published' | 'archived'
Primitive String | Int | Float | Bool | Byte | Money

// 3. Forms {} (Structures) & Lineage Inheritance
Identifiable { Id String }
Auditable    { CreatedAt Int, UpdatedAt Int }

User Identifiable {
  Id           String
  Name         String
  Age          Int
  Role         'admin' | 'manager' | 'user' | 'guest'
  PasswordHash String
}

Cloud Identifiable {
  Id     String
  Name   String
  Region String
}

// 4. Advanced Type Subtraction ('/') & Lineage Disambiguation:
// Case A: Remove field by path or symbol key
PublicUser   User / User.PasswordHash
AnonymousUsr PublicUser / 'Name'

// Case B: Merge causing field collisions on `Id` and `Name`
UserAndCloud User & Cloud

// Case C: Subtracting a specific ancestor's colliding field resolves the collision!
UserWithCloudName  UserAndCloud / User.Name   // `.Name` now resolves directly to `Cloud.Name`!
CloudWithUserName  UserAndCloud / Cloud.Name  // `.Name` now resolves directly to `User.Name`!

// Case D: Subtracting a symbol key removes `Name` from ALL merged ancestors (`User` and `Cloud`)
NoNameEntity       UserAndCloud / 'Name'

// Case E: Subtracting an entire ancestor type removes all fields originating from `Cloud`
RestoredUser       UserAndCloud / Cloud

// Case F: Subtracting and overriding a field in one expression
GuestUser          PublicUser / PublicUser.Role & { Role 'guest' }

// 5. Generic Key-Value Map, Set & Dictionary Manipulation (parens `(K)` define parameterized key)
Map(K, V) { (K) V }
Set(T)    { (T) Bool }

ConfigMap {
  "host" "localhost"
  "port" "8080"
}

UpdatedConfig ConfigMap & { "port" "9090", "tls" "true" } / "host"


#
3. PRIMITIVE OPERATIONS, LOCAL BINDINGS & OPERATOR OVERLOADING
#

// Inside `{}`, parens `()` isolate infix math from space-separated Key-Value pairs.
MathOps(A Int, B Int) [
  {
    Add (A + B)
    Sub (A - B)
    Mul (A * B)
    Div (A / B)
    Mod (A % B)
    Pow (A ** B)

    BitAnd (A . bit_and(B))
    BitOr  (A . bit_or(B))
    BitXor (A . bit_xor(B))
    ShiftL (A << 2)
    ShiftR (A >> 2)
  }
]

// Custom Operator Overloading inside Forms (Law 1a projects `X` and `Y` into scope):
Vec2 {
  X Float
  Y Float

  +(Other Vec2)  [ Vec2 { X (X + Other.X), Y (Y + Other.Y) } ]
  -(Other Vec2)  [ Vec2 { X (X - Other.X), Y (Y - Other.Y) } ]
  *(Scale Float) [ Vec2 { X (X * Scale),   Y (Y * Scale) } ]
  ==(Other Vec2) [ (X == Other.X) && (Y == Other.Y) ]
}

LogicOps(X Bool, Y Bool, Val Int) [
  {
    And     (X && Y)
    Or      (X || Y)
    Not     (!X)
    Eq      (Val == 42)
    Neq     (Val != 0)
    Gt      (Val > 10)
    Gte     (Val >= 10)
    Lt      (Val < 100)
    Lte     (Val <= 100)
    InRange (Val . (1..100))
  }
]

// Universal Binding Axiom: `Full "{First} {Last}"` binds a local variable inside `[]`
// using the exact same `Name Value` syntax as file root and `{}`!
StringOps(First String, Last String) [
  Full "{First} {Last}"

  Full
  . Trim
  . ToUpper
  . Split(" ")
]

// Stream composition via `&` combines multiple transformations into a single reusable pipeline:
CleanAndUpper (Trim & ToUpper)

StringOpsComposed(First String, Last String) [
  "{First} {Last}"
  . CleanAndUpper
  . Split(" ")
]


#
4. FUNCTION ARGUMENTS: NAMED vs ANONYMOUS SLOTS, DEFAULTS & REBINDING
#

Shell {
  Log(Msg) [ Msg . native_print ]
  Exec     [ _ . native_exec ]
}

Cart { CartId String, Amount Int }

Db {
  // 1. Named Slot (Id) captures the incoming value without unpacking
  FindUser(Id String) [
    User { Id Id, Name "Alice", Age 25, Role 'user', PasswordHash "xyz" }
  ]

  // 2. Anonymous Typed Slot (Identifiable): triggers Law 1c (auto-unpacks `Id`)
  // and Law 3 (bypasses `None` or non-conforming types automatically)
  FindCart(Identifiable) [
    Cart { CartId "c_for_{Id}", Amount 2500 }
  ]
}

// 3. Default Arguments, Partial Application & Named Overrides via `&`:
Connect(Host "localhost", Port 8080, Tls false) [
  "Connecting to {Host}:{Port} (TLS: {Tls})"
]

DefaultConn   Connect()                                 // Uses all defaults
PartialConn   Connect("db.internal")                    // Fixes 1st arg, returns function of (Port, Tls)
SecureConnect (Connect & { Port 9090, Tls true })       // Rebinds specific default arguments by name
ActiveConn    SecureConnect()                           // Invokes the customized function

// 4. Function Refinements via Option Forms:
FindOptions { Case false, Limit 0, Last false }
Find(Collection, Target, Opts FindOptions) [
  Collection . native_find(Target, Opts.Case, Opts.Limit, Opts.Last)
]
SearchUsers(Names) [
  Names . Find("Alice", FindOptions & { Case true, Limit 5 })
]


#
5. PIPELINE OPERATORS (., .>, .&, .?, .|) & CONTEXT ENRICHMENT
#

// Unnamed slot `(User & Cart)` auto-unpacks `Name` and `Amount` directly into scope!
FinalizeBill(User & Cart) [
  _ & {
    Discount [
      Amount . {
        [ _ >= 2000 ] [ 300 ]
        [ _ >= 1000 ] [ 100 ]
        _             [ 0 ]
      }
    ]
    FinalPrice [ Amount - Discount ]
  }
  .> [ "Receipt for {Name}: {FinalPrice} USD (Discount: {Discount})" . Shell.Log ]
]

ProcessOrderPipeline(UserId String) [
  UserId
  .  Db.FindUser          // `.`  Standard Pipe: returns User (which inherits Identifiable)
  .? [ _.Age >= 18 ]      // `.?` Filter Pipe: returns User | None
  .& Db.FindCart          // `.&` Enrich Pipe: `None` bypasses via Law 3; User merges with Cart -> (User & Cart) | None
  .  FinalizeBill         // `.`  Processes (User & Cart), while `None` bypasses to final output
]


#
6. REFINEMENT TYPES & PATTERN ROUTING (LAW 1: AUTO-UNPACKING)
#

IsAdult   [ _.Age >= 18 ]
IsMinor   [ _.Age < 18 ]
IsManager [ _.Role . ('admin' | 'manager') ]
NonEmpty  [ _.Length > 0 ]
EmailStr  String & [ _ . Regex.Matches("^[^@]+@[^@]+\\.[^@]+$") ]

Adult   User & IsAdult
Minor   User & IsMinor
Manager User & IsManager

ClassifyAccount(Account) [
  Account . {
    // Anonymous Type match in `{}` automatically projects `Name` into `[]` (Law 1b)
    Manager [ "Dashboard: {Name}" . Shell.Log ]
    Adult   [ "Adult account: {Name}" . Shell.Log ]
    Minor   [ "Child account: {Name}" . Shell.Log ]
    _       [ "Unknown profile" . Shell.Log ]
  }
]

Circle { Radius Float }
Rect   { Width Float, Height Float }
Shape  Circle | Rect

Area(S Shape) [
  S . {
    // Inside `[]`, math expressions do not require outer parens `()`
    Circle [ 3.14159 * Radius * Radius ]
    Rect   [ Width * Height ]
  }
]


#
7. COLLECTIONS IN PARENS (), STREAM ITERATION & TUPLE MATCHING
#
// Ordered lists of data use parens `()`, keeping `[]` exclusively for executable streams!

Item { Name String, Price Int, InStock Bool }

OrderList (
  Item { Name "Laptop", Price 1200, InStock true }
  Item { Name "Mouse",  Price 25,   InStock false }
  Item { Name "Screen", Price 400,  InStock true }
)

CalculateTotalInStock(Items) [
  Items
  . Filter([ _.InStock == true ])
  . Map([ _.Price ])
  . Fold(0, (Acc, Price) [ Acc + Price ])
]

FizzBuzz(Limit Int) [
  (1..Limit) . Map((N) [
    (N % 3, N % 5) . {
      (0, 0) [ "FizzBuzz" ]
      (0, _) [ "Fizz" ]
      (_, 0) [ "Buzz" ]
      _      [ "{N}" ]
    }
  ])
]


#
8. LINEAGE COLLISION RESOLUTION (LAW 2), ERROR CASES & TYPE MIGRATION
#

Order { Id String, Amount Int }

// 1. Merging User and Order causes a collision on the `Id` field
UserOrder User & Order

ProcessCombined(CombinedContext) [
  CombinedContext . {
    UserOrder [
      // Unique fields (`Name`, `Amount`) are unpacked and accessed flatly:
      "Client Name: {Name}, Order Amount: {Amount}" . Shell.Log

      // ❌ COMPILE ERROR (Law 2 Violation): Ambiguous field `Id` (present in both `User` and `Order`)
      // "ID: {Id}" . Shell.Log

      // ✅ VALID: Colliding fields are accessed via explicit Lineage path:
      "User ID: {User.Id}, Order ID: {Order.Id}" . Shell.Log
    ]
  }
]

// 2. Resolving collision permanently via Subtraction `/`:
UserOrderClean UserOrder / Order.Id // Removes Order's Id; `Id` now unambiguously refers to `User.Id`!

// 3. Declarative Type Migration:
ClientAccount { AccountId String, Name String, IsAdult Bool }

// Anonymous slot `(User)` unpacks `Id`, `Name`, `Age` for clean mapping:
MigrateUser(User) [
  ClientAccount {
    AccountId Id
    Name      Name
    IsAdult   (Age >= 18)
  }
]


#
9. REACTIVE FORMS (EXCEL-STYLE DATAFLOW REACTIVITY)
#
// Parameterless streams `[]` inside a Form `{}` that do not consume a free pipe input `_`
// act as reactive dataflow formulas (Law 1a). Whenever source fields are overridden via `&`,
// dependent fields recompute automatically upon access.

Invoice {
  Price    $100.00
  Quantity 3
  TaxRate  20%

  Subtotal [ Price * Quantity ]
  Tax      [ Subtotal * TaxRate ]
  Total    [ Subtotal + Tax ]
}

Order1 Invoice & { Quantity 5 }
// Order1.Total automatically evaluates to $600.00


#
10. ONTOLOGICAL TYPING (TYPESTATE) & OUTCOME COLLECTION (LAW 3: RAILWAY-FREE)
#
// Instead of exceptions or Result<T, E> monads, the pipeline collects outcomes into a Union.

Reserved   { ReservedAt Int }
Paid       { TransactionId String }
Cancelled  { CancelledAt Int }

OutOfStock { Reason String }
NoMoney    { Reason String }

// Named slot `(O Order)` keeps `O` bound as a variable; we access `O.Amount` explicitly.
// Type subtraction `Order / Cancelled` ensures cancelled orders cannot enter fulfillment!
ReserveStock(O Order / Cancelled) [
  O.Amount . {
    [ _ > 0 ] [ O & Reserved { ReservedAt 1620000000 } ]
    _         [ OutOfStock { Reason "Warehouse empty" } ]
  }
]

ChargePayment(O Order & Reserved) [
  O.Amount . {
    [ _ < 50000 ] [ O & Paid { TransactionId "tx_001" } ]
    _             [ NoMoney { Reason "Insufficient funds" } ]
  }
]

// Inferred return type: (Order & Reserved & Paid) | OutOfStock | NoMoney
// If ReserveStock returns OutOfStock, it automatically bypasses ChargePayment (Law 3)!
ExecuteFulfillment(InitialOrder Order / Cancelled) [
  InitialOrder
  . ReserveStock
  . ChargePayment
]

// Unnamed slot `(Order & Reserved & Paid)` auto-unpacks `Id` and `TransactionId`:
ShipToCustomer(Order & Reserved & Paid) [
  "Shipping order {Id}. Transaction {TransactionId} confirmed." . Shell.Log
]

// ❌ COMPILE ERROR Example (Typestate Violation):
// RawOrder Order { Id "o1", Amount 100 }
// RawOrder . ShipToCustomer // Error: Expected `Order & Reserved & Paid`, got `Order`


#
11. TYPE-LEVEL COMPUTATIONS, GENERIC CONTRACTS & HKT
#

// 1. Type-level Peano numbers & Fixed-Length Vector
Zero    {}
Succ(N) { Prev N }
One     Succ(Zero)
Two     Succ(One)
Three   Succ(Two)

Tuple(T) ()

FixedVector(T, N) {
  Data   Tuple(T)
  Length N
}

// 2. Recursive Generics: List and Binary Tree
None {}
Cons(T) {
  Head T
  Tail List(T)
}
List(T) None | Cons(T)

Leaf {}
Node(T) {
  Value T
  Left  Tree(T)
  Right Tree(T)
}
Tree(T) Leaf | Node(T)

// 3. F-Bounded Polymorphism (replaces `T extends Comparable<T>`)
// `Self` automatically binds `Other` to the implementing type:
Comparable(Other Self) {
  <(Other) Bool // Contract specification: operator `<` taking `T` and returning `Bool`
}

// Accepts a List of any elements that fulfill Comparable:
Sort(Items List(Comparable)) [
  Items . native_sort_by((A, B) [ A < B ])
]

// Intersection `&` restricts elements to be both `User` and `Comparable(User)`:
SortUsers(Users List(User & Comparable(User))) [
  Users . Sort
]

// 4. Multi-level Nested Generics & Higher-Kinded Types (HKT)
Box(T)         { Item T }
NestedBox(T)   Box(Box(T))
ApplyHKT(F, V) [ F { Item V } ]


#
12. HIGHER-ORDER FUNCTIONS, EARLY EXIT (`Done`) & CATAMORPHISMS
#

Some(T)   { Value T }
Option(T) Some(T) | None

// 1. Functional Early Exit in `Fold` via `Done` (No imperative `break`/`return` needed)
Done(T) { Value T }

SumUntilLimit(Numbers List(Int), Max Int) [
  Numbers . Fold(0, (Acc, N) [
    (Acc + N) . {
      [ _ > Max ] [ Done { Value _ } ] // Immediately halts Fold and unwraps Value!
      _           [ _ ]                // Continues iteration
    }
  ])
]

// Lazy stream short-circuiting via `TakeWhile` and `Find`:
FirstAdult(Users List(User)) [
  Users
  . TakeWhile([ _.Age > 0 ])
  . Find([ _.Age >= 18 ]) // Returns User | None immediately upon first match
]

// 2. Functor over recursive List (Head and Tail unpack automatically on anonymous Cons match)
ListMap(L, Action) [
  L . {
    None [ None ]
    Cons [
      Cons {
        Head (Head . Action)
        Tail (Tail . ListMap(Action))
      }
    ]
  }
]

// 3. Catamorphism (Generic AST Folder with decoupled Evaluation Algebras)
Num(T)    { Value T }
Add(L, R) { Left L, Right R }
Mul(L, R) { Left L, Right R }
Expr      Num(Int) | Add(Expr, Expr) | Mul(Expr, Expr)

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


#
13. CONCURRENCY (PARALLEL PIPE .|, FAULT ISOLATION, CSP CHANNELS) & ATOMS
#

// 1. Parallel Fan-Out Operator '.|' with Fault Isolation & Duration Timeouts:
// All branches in '{}' execute concurrently. If `Ads` times out or returns `HttpError`,
// Law 3 isolates the failure inside the `.Ads` field (`AdData | TimeoutError`) without
// crashing or canceling `Profile` and `Recs`!
FetchProfile(Id String)         [ "https://api.ex/users/{Id}" . Http.Get ]
FetchRecommendations(Id String) [ "https://api.ex/recs/{Id}" . Http.Get ]
FetchAds()                      [ https://api.ex/ads . Http.Get . Timeout(2s) ]

LoadDashboardData(UserId String) [
  UserId .| {
    Profile [ _ . FetchProfile ]
    Recs    [ _ . FetchRecommendations ]
    Ads     [ FetchAds() ]
  }
]

// 2. CSP Channels & Tail-Recursive Actor Loop (`Self` references the Channel instance via Law 1a)
Channel(T) {
  Send(Val T) [ (Self, Val) . native_chan_send ]
  Recv()      [ Self . native_chan_recv ]
}

WorkerLoop(Jobs Channel(String), Results Channel(Int)) [
  Jobs.Recv() . {
    None [ "Channel closed, terminating worker." . Shell.Log ]
    Some [
      Value
      . Length
      . Results.Send

      WorkerLoop(Jobs, Results)
    ]
  }
]

// 3. Pure State Reducer & Transactional Atom
// Named parameter `State CounterState` requires `State.Count` (Law 1),
// while anonymous match `Increment` unpacks `By` without conflict!
CounterState { Count Int }
Increment    { By Int }
Decrement    { By Int }
Reset        {}
CounterMsg   Increment | Decrement | Reset

UpdateCounter(State CounterState, Msg CounterMsg) [
  Msg . {
    Increment [ CounterState { Count (State.Count + By) } ]
    Decrement [ CounterState { Count (State.Count - By) } ]
    Reset     [ CounterState { Count 0 } ]
  }
]

Atom(T) {
  Value            T
  Read()           [ Self . native_atom_read ]
  Swap(Transition) [ (Self, Transition) . native_atom_swap ]
}

GlobalCounter Atom(CounterState) {
  Value CounterState { Count 0 }
}

ModifyGlobalCounter() [
  GlobalCounter.Swap((State) [ UpdateCounter(State, Increment { By 5 }) ])
]


#
14. RESOURCE MANAGEMENT, SCOPED LIFETIMES & UNIFIED PORTS
#

FileHandle { Fd Int, Path File }
IoError    { Msg String }

Fs {
  Open(Path File)     [ Path . native_fs_open ] // Returns FileHandle | IoError
  Read(FileHandle)    [ Fd . native_fs_read ]   // Anonymous slot unpacks `Fd`
  Close(FileHandle)   [ Fd . native_fs_close ]

  // Bracket pattern: `Handle` binds the matched `FileHandle` instance so `Fs.Close`
  // closes the file after `Action` while returning `Action`'s result (`IoError` bypasses via Law 3)!
  WithFile(Path File, Action) [
    Path . Fs.Open . {
      FileHandle [
        Handle _
        Handle . Action
        .> [ Handle . Fs.Close ]
      ]
    }
  ]
}

ReadConfig(Path File) [
  Fs.WithFile(Path, (F) [
    F . Fs.Read . Json.Parse
  ])
]

// Unified Port Protocol: URLs and Files share identical read/write verbs
SyncRemoteBackup() [
  https://api.razomy.org/v1/users
  . Port.Read
  . Port.Write(%./backup/users.json)
]


#
15. DEEP PATTERN ROUTING (STRUCTURAL & TUPLE DESTRUCTURING)
#
// Routers `{}` can simultaneously match Form fields and destructure nested Tuples `()`.

HttpRequest {
  Method  'GET' | 'POST' | 'PUT' | 'DELETE'
  Path    Tuple(String)
  Headers Map(String, String)
  Body    String
}

HttpResponse { Status Int, Body String }
Unauthorized { Reason String }

AuthMiddleware(Req HttpRequest) [
  Req.Headers . Map.Get("Authorization") . {
    Some [ Req & { AuthUser (Value . Db.FindUserByToken) } ]
    None [ Unauthorized { Reason "Missing Authorization Header" } ]
  }
]

ApiRouter(Req HttpRequest) [
  Req . {
    // Matches exact Form field AND single-element Tuple
    { Method 'GET', Path ("health") } [
      HttpResponse { Status 200, Body "OK" }
    ]

    // Matches Method 'GET' AND extracts variable `Id` from the 2nd Tuple element!
    { Method 'GET', Path ("users", Id) } [
      Req . AuthMiddleware . {
        Unauthorized [ HttpResponse { Status 401, Body Reason } ]
        HttpRequest  [
          JsonStr (Id . Db.FindUser . Json.Encode)
          HttpResponse { Status 200, Body JsonStr }
        ]
      }
    ]

    _ [ HttpResponse { Status 404, Body "Not Found" } ]
  }
]


#
16. BUILT-IN DIALECTS: PARSE (STREAM GRAMMARS) & VIEW/DRAW (DECLARATIVE UI)
#

// 1. Parse Dialect: PEG grammar stream where cursor state flows via `.` through rules
Digit "0".."9"
Alpha "a".."z" | "A".."Z"

EmailGrammar String & [
  _ . Parse([
    Some(Alpha) . Match("@") . Some(Alpha) . Match(".") . Repeat(2..6, Alpha)
  ])
]

LogEntry { Date String, Level String, Msg String }

ParseLogLine(Line String) [
  Line . Parse([
    Copy('Date', To(" "))
    . Skip(1)
    . Match("[")
    . Copy('Level', To("]"))
    . Skip(2)
    . Copy('Msg', To(End))
  ]) . {
    LogEntry [ _ ]
  }
]

// 2. View & Draw Dialect: Declarative UI in Forms `{}` & Tuples `()`, canvas commands in `[]`
// Tuple-Argument Duality: Because incoming argument slots `()` ARE a Tuple, a function
// expecting `Children Tuple` directly accepts comma-separated elements without double parens `(())`!
UI {
  View(Spec)           [ Spec . native_ui_view ]
  Draw(Children Tuple) [ Children . native_ui_draw ]
}

CounterApp(State Atom(CounterState)) [
  UI.View({
    Title "Razomy Counter"
    Size  1920x1080
    Body  (
      Heading("Current Count: {State.Read().Count}")
      Row(
        Button("Increment", [ State.Swap((S) [ UpdateCounter(S, Increment { By 1 }) ]) ]),
        Button("Reset",     [ State.Swap((S) [ UpdateCounter(S, Reset) ]) ])
      )
      Canvas(200x100, [
        UI.Draw(
          Pen(#FF0000),
          FillPen(#00FF00),
          Circle((50, 50), 40)
        )
      ])
    )
  })
]


#
17. METAPROGRAMMING, REFLECTION, COMPOSE & DEEP TREE REWRITING
#

// 1. Algebraic Stream Composition
Step1 [ _ + 10 ]
Step2 [ _ * 2 ]
CombinedPipeline Step1 & Step2

// 2. Partial Evaluation (`Compose`): evaluates only parenthesized `()` expressions inside `[]`
BuildMultiplier(Factor Int) [
  Compose([ _ * (Factor * 10) ]) // Returns stream `[ _ * 50 ]` when Factor == 5
]

// 3. Aspect-Oriented Stream Manipulation (`Meta.StepsOf`)
TraceExecution(CodeBlock) [
  CodeBlock
  . Meta.StepsOf
  . FlatMap((Step) [
    Step & [ _ .> [ "Step result: {_}" . Shell.Log ] ]
  ])
]

// 4. Compile-time AST Macro (`DeriveJson`) & Runtime Reflection (`Meta.FieldsOf`)
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

// Sequential statements inside `[]` separated by newline `\n`:
InspectEntity(Entity) [
  TypeName (Entity . Meta.TypeOf)
  "Inspecting type: {TypeName}" . Shell.Log

  Entity . Meta.FieldsOf . ForEach((F) [
    "Field {F.Name} has type {F.Type} and value {F.Value}" . Shell.Log
  ])
]

// 5. Deep Tree Rewriting (`Meta.Rewrite`):
ComplexSystem {
  Name "Core"
  Calc [ _ * 1.15 ]
  SubModule {
    Nested {
      Logger [ _ . Shell.Log ]
      Calc   [ _ * 1.15 ]
    }
  }
}

UpgradedSystem ComplexSystem . Meta.Rewrite({
  Calc(_)   [ [ _ * 1.20 ] ]   // Replaces all `Calc` functions at every depth
  Shell.Log [ Audit.Write ]    // Replaces `Shell.Log` inside any nested stream
})


#
18. IMPORTS, MODULE ENCAPSULATION (`& Public`, `/`) & NATIVE WRAPPERS
#

// 1. Standard & Relative Imports (Native C/Rust/JS libraries are wrapped as normal `.rz` modules)
Base          root.base          // Named module import
(String, Int) root.base          // Element destructuring import via Tuple `()`
_             root.base          // Wildcard import (all)
_             base               // Relative module import
_             %../../../base     // Deep relative path import via File literal `%`
Http          std.net.http       // Standard library wrapper module
Sqlite        std.db.sqlite      // Native wrapper module (no FFI pollution in user code)

// 2. Module Visibility & Encapsulation via Algebraic `& Public` or Subtraction `/`:
Public  { }
Private { }

AuthInternal {
  Salt         "x9f2a" & Private
  Hash(Raw)    [ "{Raw}_{Salt}" . native_sha256 ] & Private
  Verify(U, P) [ (P . Hash) == U.PasswordHash ] & Public
}

// Expose only Public members, or explicitly subtract internal keys/paths via `/`:
AuthModule  AuthInternal & Public
CleanModule AuthInternal / 'Salt' / 'Hash'


#
19. TESTING AND PROPERTY-BASED ASSERTIONS
#

TestPassed { Name String }
TestFailed { Name String, Expected Any, Actual Any }
TestResult TestPassed | TestFailed

AssertEq(Name String, Expected, Actual) [
  (Expected == Actual) . {
    true  [ TestPassed { Name Name } ]
    false [ TestFailed { Name Name, Expected Expected, Actual Actual } ]
  }
]

RunTestSuite() [
  MigratedBob (User { Id "1", Name "Bob", Age 20, Role 'user', PasswordHash "x" } . MigrateUser)

  (
    AssertEq("Math Addition", 4, (2 + 2))
    AssertEq("Area of Rect", 20.0, (Rect { Width 4.0, Height 5.0 } . Area))
    AssertEq("User Migration", true, MigratedBob.IsAdult)
  )
  . ForEach((Result) [
    Result . {
      TestPassed [ "[PASS] {Name}" . Shell.Log ]
      TestFailed [ "[FAIL] {Name}: Expected {Expected}, got {Actual}" . Shell.Log ]
    }
  ])
]


#
20. CAPSTONE: RECURSIVE AUTONOMOUS AI AGENT & MAIN ENTRY POINT
#

CompileError { Code Int, Log String }
BuildSuccess { Status String, Artifact String }
BuildResult  BuildSuccess | CompileError

FatalError CompileError & [ _.Code >= 500 ]
Warning    CompileError & [ _.Code < 500 ]

AiQuery    { SystemPrompt String, UserPrompt String }
AiResponse { Diff String, Explanation String }

Ai {
  Call(Query AiQuery) [ Query . native_ai_llm_request ]
}

RunBuild() [ "cargo build --release" . Shell.Exec ]

// Named parameter `Err CompileError` uses `Err.Log`, avoiding any shadowing with `AiResponse`!
AgentLoop(Err CompileError, Retries Int) [
  Retries . {
    [ _ == 0 ] [ "Retry limit reached. Human intervention required." . Shell.Log ]

    _ [
      AiQuery {
        SystemPrompt "You are an engineer. Fix the compilation error with a git diff patch."
        UserPrompt   "Error log: {Err.Log}. Write a patch."
      }
      . Ai.Call
      .> [ "AI suggests solution: {_.Explanation}" . Shell.Log ]
      .> [ "git apply {_.Diff}" . Shell.Exec ]

      RunBuild()
      . {
        BuildSuccess [
          "Build succeeded after applying patch!" . Shell.Log
        ]

        // Anonymous match: `_` is the new `CompileError` instance
        CompileError [
          _
          .> [ "Patch did not help ({Log}). Starting next attempt..." . Shell.Log ]
          . AgentLoop(Retries - 1)
        ]
      }
    ]
  }
]

// Sequential statements separated by `\n`, pipelines connected via `.`
Main [
  "Starting test suite..." . Shell.Log
  RunTestSuite()

  "Starting project build process..." . Shell.Log
  RunBuild() . {
    FatalError [
      // `Code` is auto-unpacked (Law 1b), `_` is the whole `FatalError` passed to AgentLoop
      _
      .> [ "Critical compilation failure ({Code}). Launching agent..." . Shell.Log ]
      .  AgentLoop(3)
    ]

    Warning [
      "Compiler warnings: {Log}" . Shell.Log
      "cargo fix --allow-no-vcs" . Shell.Exec
      "Automatic linting completed." . Shell.Log
    ]

    BuildSuccess [
      "Project is completely clean ({Artifact}). Ready for release!" . Shell.Log
    ]
  }
]