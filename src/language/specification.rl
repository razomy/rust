name "Razomy language"
version "9.0"
extension ".rl"
tags (
  "ONTOLOGICAL"
  "HOMOICONIC"
  "AXIOMATIC"
  "FRACTAL"
  "SYMMETRIC"
  "CONCATENATIVE"
  "ALGEBRAIC-LINEAGE"
  "STAGE-POLYMORPHIC"
  "LINEAR-PERCEUS"
  "UNIVALENT-EMERGENT"
)
description r3"
================================================================================
PART I. SYSTEM ARCHITECTURE & AXIOMATIC STANDARD
================================================================================

1. SYSTEM COMPOSITION (6 CANONICAL ENTITIES):
The entire Razomy language, type system, module graph, memory model, reactive field topology, and homoiconic compiler AST
are built from 6 foundational concepts. Razomy is a Meta-Ontological Specification Language with zero-loss semantic lowering
into target systems (Rust, C#, Native).
There are no reserved keywords (`struct`, `class`, `fn`, `let`, `if`, `match`, `while`, `return`, `try`, `defer`, `self`, `import`).
Each concept has 1 canonical name and replaces legacy overlapping terms:

• `Form` (`{}`)
  - What it is : Named key-value structure, type prototype, pattern router, module namespace, Ambient Topos (Coeffect
                 environment, reactive `& Watch` topology & Noether conservation boundary), and homoiconic AST node.
  - Replaces   : Struct, Class, Interface, Trait, Module, Object, Dictionary, Switch/Match, Select, Signal/Effect, DI Container, AST.

• `Tuple` (`()`)
  - What it is : Ordered positional collection, parameter slot list, and eager evaluation group.
  - Replaces   : Array, Homogeneous Slice, Fixed Tuple, Parameter List, Argument Pack.

• `Stream` (`[]`)
  - What it is : Lazy executable pipeline of steps, invertible bijection, reactive observer, and tail-call-optimized attractor loop.
  - Replaces   : Function Body, Lambda, Closure, Computed Getter, Refinement Predicate, Generator, Loop Body, Lens, Dialect Block.

• `Lineage` (`..`, `&`, `/`)
  - What it is : Complete ancestor history tree preserved across algebraic compositions and subtractions with zero runtime overhead.
  - Replaces   : Class Inheritance, Trait Implementation, Mixins, Row Polymorphism, Visibility Modifiers (`public`/`private`).

• `Hole` (`_`, `_T`, `_a`)
  - What it is : Polymorphic potential placeholder and Coeffect Requirement slot constrained inline via `&`.
  - Replaces   : Generic Type Parameter `<T>`, Wildcard, Higher-Kinded Slot `F<_>`, Implicit Fold/Map Argument, Context Slot.

• `Bypass` (`& Bypass`)
  - What it is : Scoped union outcome tag (`None`, `Error`, `Break`, `Return`) that automatically short-circuits pipelines.
  - Replaces   : Exceptions, Try/Catch, Null/Nil, Early Return Statements, Goto/Break Keywords, Algebraic Throw Effects.

2. THE 5 ONTOLOGICAL AXIOMS -> ENGINEERING LAWS -> COMPILER GUARANTEES:

• AXIOM I: THE AXIOM OF JUXTAPOSITION (`Target Source` — SPACE IS THE UNIVERSAL `Bind`)
  - Philosophical Axiom (Keywordless Relation & Univalence):
    A space between two entities (`Target Source`) is the sole relational primitive (`RazomySpec.Compiler.Bind`).
    Declaration, typing, inheritance, assignment, destructuring, and Univalent Isomorphism (HoTT path equivalence)
    are not separate syntax constructs—they are the context-aware juxtaposition of a Target and a Source.
  - Engineering Laws:
    1. Context-Exact Universal Bind Matrix :
       • `TypeA TypeB`              -> Type Alias or Subtype Specialization (`BrandName String`, `Car Vehicle { ... }`)
       • `name expr`                -> Pure Immutable Value Binding (`counter 0`, `my_car (Car {})`)
       • `name Type default`        -> Form Field with Default inside `{}` (`max_speed Int 180`; stream field uses `&`: `act [ . + 1 ] & Stream`)
       • `name [ ... ]`             -> Zero-Cost Lazy Getter inside `{}` (participates in `==` & `Meta.fields_of`) or Stream Binding
       • `name [ ... ] & Watch`     -> Named Stigmergic Observer inside `{}` (auto-triggers when referenced sibling fields update)
       • `name(args) RetType [...]` -> Named Function/Method with Return Type (or Univalent Constructor Path `Meters(Kilometers) [...]`)
       • `name Type (expr)`         -> Guarded Local Binding inside `[]` (`fh FileHandle (path . Fs.open)` — narrows `Type` & auto-bypasses `& Bypass`)
       • `(a, ..rest) expr`         -> Symmetric Tuple/Form Destructuring (`{ name, ..other } user_form`)
    2. Tight Call vs Spaced Bind    : `fn(x)` without space invokes `fn`; `a (x)` with space binds `a` to `(x)`.
    3. 1-Tuple vs Eager Grouping    : `(expr)` groups an eager calculation; `(expr,)` with a trailing comma constructs a 1-element `Tuple`.
    4. Symmetric Spread (`..`)      : In a Source (`{ ..Vehicle }`, `(0, ..a)`), `..` expands elements; in a Target
                                      (`(first, ..rest) nums`), `..` captures the remaining slice; at a pipeline step (`. ..`),
                                      `..` recursively loops the enclosing `[...]` stream (Fractal Attractor Loop).
  - Compiler Guarantees:
    ◦ `E01_AMBIGUOUS_CALL_OR_BIND` : `area(Circle { radius 5.0 })` calls `area`. Placing a space after a 1-argument callable
      in expression position or without an explicit type guard (`area (Circle { radius 5.0 })`) when `area` accepts `Circle`
      is rejected; use tight call `area(...)`, pipeline `... . area`, or typed bind `val Float (...)`.
    ◦ `E09_DANGLING_BIND_NEWLINE`  : `,` and `\n` separate elements across `{}`, `[]`, `()`, except before a binary/pipe
      operator. Placing `\n` between a bind target and its opening bracket (`discount \n [ amount - 10 ]`) is rejected.
    ◦ `E10_MULTIPLE_REST_PATTERNS` : Multiple `..spreads` in a Source `(0, ..a, ..b)` are valid; multiple `..rest`
      captures in a Target `(first, ..r1, ..r2) nums` are rejected.

• AXIOM II: THE AXIOM OF THE FRACTAL TRIAD (`()` ORDER <-> `{}` STRUCTURE <-> `[]` ACTION)
  - Philosophical Axiom (Scale Invariance, Stigmergy & Topos Coeffects):
    Every entity from a hardware register and AST node up to a module or UI tree exists as 3 dual projections:
    Positional Order `()`, Named Structure `{}`, and Lazy Action `[]`. Conversely, an enclosing `{}` acts as a Morphogenetic
    Topos: it hosts lazy computed fields (`[]`), named reactive laws (`& Watch`), and supplies Ambient Coeffects to nested
    streams—eliminating external `Signal`/`Effect` primitives.
  - Engineering Laws:
    1. Canonical Triad      : `Name(Tuple) {Form} [Stream]` projects any declaration into `()`, `{}`, and `[]`.
                              Slotless `name [...]` inside `{}` is a lazy field; `name [...] & Watch` is a reactive observer
                              triggered on sibling update; slotted `name() [...]` is an explicitly invoked method.
    2. Constructor Duality  : Declaring `Name { ... }` automatically synthesizes a positional constructor `Name(...)`
                              for its fields in declaration order (fields with defaults are optional trailing slots:
                              `ProfileCard { id, name }` <-> `ProfileCard("u1", "Alice")`).
                              A bare `snake_case` key without a value inside `{}` (`Box { item }`) is always an open slot `item _T`.
    3. Direct Presence      : Inside `{}` and `[]`, there is no `self`, `this`, or `ctx`:
       • (a) Sibling Scope  : Sibling fields and slot parameters are directly visible by name inside `{}` and `[...]`.
       • (b) Type & Lineage Projection: A single concrete `PascalCase` Type (or subtracted lineage `(Type / ...)`)
                              in `FormName { ... }`, `fn(User)`, or `{ User [...] }` automatically projects its root
                              `snake_case` binding (`User` -> `user`), and projects each composed ancestor's `snake_case`
                              binding (`Comparable` -> `comparable`) inside methods inherited from that ancestor.
       • (c) Disambiguation : Multiple identical types in the same or enclosing scope (e.g., method `equals(other User)` inside `User`)
                              require explicit names (`fn(src Account, dst Account)`), direct field access, destructuring
                              (`fn({ name } WithName)`), or parameter holes (`[ _a + _b ]`).
  - Compiler Guarantees:
    ◦ `E02_DUPLICATE_UNNAMED_TYPE`      : `transfer(src Account, dst Account, Money)` is valid (`Money` -> `money`);
      `transfer(Account, Account, Money)` or `User { equals(User) [...] }` with duplicate unnamed `Account`/`User` is rejected.
    ◦ `E11_OVERLOAD_SIGNATURE_CONFLICT` : Multiple declarations sharing a `PascalCase` or `snake_case` name merge into
      one multi-dispatched callable only if their `()` parameter types are distinct; duplicate/overlapping signatures fail.

• AXIOM III: THE AXIOM OF ALGEBRAIC MEMORY, CONSERVATION & DUALITY (`&`, `|`, `/`, `!`)
  - Philosophical Axiom (Unforgetting Set Algebra, Noether Conservation & Reversible Computation):
    One boolean algebra (`&`, `|`, `/`, `!`) governs types, forms, functions, streams, invariants, protocols, and visibility.
    Composition (`&`) merges capabilities and attaches Noether conservation laws while preserving the full ancestor tree
    (`Lineage`); subtraction (`/`) surgically strips fields, ancestors, or `& Watch` laws; negation (`!`) encodes closed-world
    complement, negative typestate, protocol session duality, and bidirectional stream inversion.
  - Engineering Laws:
    1. Universal Intersection (`&`), Zero-Cost Lineage & Transactional Noether Conservation Laws:
       • `FormA & FormB`       -> Merges fields/traits. Historical ancestor defaults (`MyCar.SportsCar.Car.brand`) reside in
                                  compile-time type metadata (0 runtime bytes); runtime instances store flat active fields (plus
                                  distinct offsets for colliding ancestor fields like `user_order.User.id` and `user_order.Order.id`).
       • `OverloadA & OverloadB`-> Implicitly merges distinct same-name constructors/functions (`Color(Byte)` & `Color(String)`).
       • `Function & Form`     -> Partial application, Default override, OR Ambient Coeffect Handler (`connect & { port 9090 }`,
                                  or `fn & { Shell MockShell }` to intercept ambient `PascalCase` capabilities inside `fn`).
       • `StreamA & StreamB`   -> Sequential point-free pipeline composition (`trim_str & upper_str`).
       • `Type & [Predicate]`  -> Single-value refinement (`Positive Int & [ .> 0 ]`), Pattern Guard `(a, b) & [ a > b ]`,
                                  or Multi-Field Noether Conservation Invariant on a Form (`Range { start _T, end _T } & [ start <= end ]`).
                                  Every `.=` / `&=` executes atomically: (1) fields update, (2) affected `& Watch` observers
                                  fire to fixed-point when their referenced sibling fields change (`old != new`), and
                                  (3) Form `& [Invariant]` laws verify (rolling back on fault)!
       • `Entity & Directive`  -> Attaches stage, memory, reactivity, or visibility metadata (`val & Private`, `fh & Linear`, `rule & Watch`).
    2. Exact Disambiguation of Subtraction (`/`) vs Runtime Division:
       • Compile-Time Type/Form Subtraction: Triggered when the right operand is a `PascalCase` Type/Directive (`User / Private`,
         `Result / Error`, `ElectricCar / Vehicle`, `Reactor / Watch`), a qualified ancestor field (`User / User.password_hash`),
         or a `'symbol'` key (`UserAndCloud / 'name'`, `Reactor / 'auto_cool'`).
       • Runtime Value Division / Map Removal: Triggered when both operands are `snake_case` runtime values or expressions
         (`a / b`, `shape / cutter`, `config_map / "host"`), invoking the overloaded `/` method.
    3. Closed-World Negation, Protocol Duality & Stream Inversion (`!`):
       • On booleans: logical NOT (`!flag`); on types: closed-world relative complement (`Order & !Cancelled` == `Order / Cancelled`, `Never !Any`).
       • On Session Protocols (`Channel` / `Protocol`): `!ClientProto` synthesizes the dual Opponent protocol (swapping `send` <-> `recv`).
       • On Bijective Streams: `!stream` symbolically synthesizes the inverse computation by reversing steps (`[ s1, s2 ]`
         <-> `[ !s2, !s1 ]`, e.g. `c_to_f [ (. * 1.8) + 32 ]` <-> `!c_to_f [ (. - 32) / 1.8 ]`) and inverting structural
         isomorphisms (`{ id .user_id, name .full_name }` <-> `{ user_id .id, full_name .name }`).
  - Compiler Guarantees:
    ◦ `E05_AMBIGUOUS_LINEAGE_FIELD` : Accessing `entity.name` on `User & Cloud` when both ancestors define `name` is
      rejected until qualified by ancestor (`entity.User.name`) or stripped (`(entity / Cloud.name).name`).
    ◦ `E13_CONSERVATION_LAW_VIOLATION` : Constructing or rebinding (`.=`, `&=`) a Form equipped with `& [Invariant]` when the
      invariant evaluates to `false` at compile time (or without handling `InvariantFault & Bypass` at runtime) is rejected.
    ◦ `E14_NON_INVERTIBLE_STREAM` : Applying `!` to a non-bijective stream (`![ . * 0 ]` or lossy projection) fails compilation.

• AXIOM IV: THE AXIOM OF ORTHOGONAL FLOW, FRACTAL ATTRACTORS & PERCEUS VALUE SEMANTICS
  - Philosophical Axiom (Global Immutability, Continuous Flow & Perceus Local Rebind):
    Razomy has 100% Pure Value Semantics: there are no mutable references, no shared mutable state, and passing a value to a
    function can never mutate the caller's data. All operators are partitioned into 4 orthogonal, non-overlapping layers:
    (1) Pure Math & Algebra (no `.`, no `=`), (2) Tight Relative Focus inside `[...]` (leading `.`), (3) Spaced Dataflow
    Pipelines (spaced `.`, no `=`), and (4) Ergonomic Local Value Rebind (always ending with `=`: `.=`, `+=`, `-=`, `&=`),
    which desugars to pure immutable `Bind` + `&` while Perceus RC executes it in O(1) in-place when uniquely owned (`RC == 1`).
  - Engineering Laws:
    1. Tight Dot & Relative Focus (`a.b` / `.b` / `.op` — Layer 2):
       • `a.b`, `a.Parent.b`, `a.0`, `a.-1` : Tight path, historical ancestor, or positional tuple/form index access.
       • `.field`, `.0`, `.+ 1`, `.> 0`, `.`: Relative getter/operator/input targeting the innermost enclosing `[...]` stream.
                                              When passed directly as a call argument or router key outside `[]`, `.field`
                                              desugars to `[ .field ]` (Sugar #3).
    2. Spaced Pipeline Dot (`a . b`, Emergent Arity Routing & Fractal Loop `. ..` — Layer 3):
       • `a . b`   (UFCS / Univalent Pipe) : Passes `a` as first argument (`a . fn(b)` == `fn(a, b)`). If `a` is `Tuple(_T)`
                                             flowing into an inline stream `[...]`:
                                             - 1-Hole Stream `(1, 2, 3) . [ .+ 10 ]` (or `[_a * 2]`) maps element-wise -> `(11, 12, 13)`.
                                             - 2-Hole Stream `(1, 2, 3) . [ _a + _b ]` emergently folds/reduces -> `6`
                                               (on an empty `Tuple(_T)`, yields the archetype default `_T {}` such as `0` or `""`,
                                               or `None & Bypass` if `_T` has no default).
       • `. ..`    (Fractal Attractor Loop): Inside `[...]`, piping into `..` (`step . ..`) tail-recursively feeds the current value
                                             back into the start of the enclosing `[...]` stream until exited via `.? Break(val)`.
       • `a . {…}` (Form Router)    : Matches `a` against Type keys, Literals, `'symbols'`, `{ k v }`, `(x, y) & [x > y]`, `(expr)`, or `.op v`.
       • `a .? b`  (Positive Gate)  : Triggers on TRUE — filters `a` by `b` (yielding `None`), executes `[...]` on truthy `a`, or exits if `b <: Bypass`.
       • `a .! b`  (Negative Gate)  : Triggers on FALSE/None — asserts `a`, or short-circuits stream with `b` (`& Bypass`).
       • `a .> b`  (Tap Pipe)       : Executes side-effect `b(a)` on success and passes original `a` forward (inside `[]`, write `. .> b`).
       • `a .~ b`  (Finalize Pipe)  : Guaranteed `defer` — executes cleanup `b` on BOTH normal values and in-flight `& Bypass`,
                                      preserving the original outcome (intermediate steps borrow `Linear` values; `.~` consumes them).
       • `a .& b`  (Enrich Pipe)    : Evaluates `b(a)` and purely intersects result with `a` via `&` without mutating `a`.
       • `a ./ b`  (Strip Pipe)     : Purely subtracts `b` from `a` via `/` without mutating `a`.
       • `a .| {}` (Parallel Pipe)  : Structured concurrency — runs branches in `{}` concurrently on immutable snapshots and merges via `&`;
                                      if any branch emits `Error & Bypass`, sibling branches are automatically cancelled.
    3. Perceus Value Rebind (`.=`, `+=`, `-=`, `*=`, `/=`, `&=` — Layer 4), Conservation & Linear/Weak Safety:
       • Every local rebind operator ends with `=` and desugars to a pure immutable rebind of the local `snake_case` binding:
         `a += b` ==> `a (a + b)`; `a &= { k v }` ==> `a (a & { k v })`; `a.b.c .= v` ==> `a (a & { b.c v })` (Sugar #6).
       • Because values are never aliased mutably, Perceus RC performs O(1) in-place memory reuse whenever `RC == 1` and lossless
         Copy-on-Write whenever `RC > 1`, atomically verifying Form conservation invariants (`& [...]`) and firing `& Watch` laws.
       • `Weak(_T)` breaks reference cycles; `Linear` types must be consumed exactly once before scope exit.
    4. Scoped `Bypass` Short-Circuiting:
       • `Break & Bypass` is caught by its enclosing `[ ... . .. ]` attractor boundary (yielding `Break.value`).
       • `Return & Bypass` is caught by its enclosing named function boundary (yielding `Return.value`).
       • Domain outcomes (`None`, `Error & Bypass`) bypass incompatible steps `fn(Param)` and widen the function output to `Out | B`
         until explicitly handled in a Form Router `. { Error [...] }`.
  - Compiler Guarantees:
    ◦ `E04_UNHANDLED_UNION_BRANCH` : If `B` in `A | B` is not marked with `& Bypass` (`User | Company | NotFound` flowing
      into `fn(User)`), compilation fails until `Company` is explicitly routed.
    ◦ `E08_AMBIGUOUS_ROUTER_KEY`   : Bare `snake_case` identifiers in router keys (`x . { target_val [ 10 ] }`) are
      rejected to prevent accidental capture; use `.== target_val [ 10 ]` or dynamic key `(target_val) [ 10 ]`.
    ◦ `E12_UNCONSUMED_LINEAR_VALUE`: Letting a binding whose type is marked `& Linear` (`FileHandle`) go out of scope or
      bypass without consumption (e.g. via `.~ [ fh . Fs.close ]`) fails compilation.

• AXIOM V: THE AXIOM OF ONTOLOGICAL ISOMORPHISM (`PascalCase` ARCHETYPE <-> `snake_case` INCARNATION <-> `_` POTENTIAL)
  - Philosophical Axiom (Stage Polymorphism & Case Ontology):
    Types, runtime values, modules, and AST nodes are isomorphic Forms. There is no wall between compile time and runtime:
    Streams `[...]` refine types, Types route runtime values, and `Compile & [...]` rewrites code as data (`Meta.rewrite`)
    while preserving lexical hygiene and `SourceSpan` locations. An identifier's case directly encodes its ontological phase.
  - Engineering Laws:
    1. `PascalCase` (Archetype)  : Concrete Type, Form Prototype, Union Variant, or Module (Fixed at runtime;
                                   acronyms follow standard `PascalCase`: `Http`, `Ui`, `Json`, `Db`).
    2. `snake_case` (Incarnation): Runtime Value, Variable, Field Key, or Function (Pure bind `a b`; local rebind via `.=`, `+=`, `&=`).
    3. `_` Prefix   (Potential)  : Polymorphic Hole accepting inline `&` constraints (Ancestor, Structural, or Stream):
       • `_`           (Anonymous Hole)       : Wildcard `Any`, higher-kinded slot `_F(_)`, or constrained `_ & Vehicle`.
       • `_PascalCase` (Named Type Hole)      : Links positions to the exact same Type (`Pair { a _T, b _T }`).
       • `_snake_case` (Anonymous Param Hole) : Binds positional stream inputs in lexical order of first appearance
                                                (`[ _a + _b ]`, `[ _acc & _step ]`, `[ (_a & Int) * 2 ]`).
       • `'symbol'` in AST Routers            : Inside `Meta.rewrite`, a `'symbol'` router key matches `Identifier` AST
                                                nodes of the same lexical name without hygiene capture.
  - Compiler Guarantees:
    ◦ `E03_UNKNOWN_CONCRETE_TYPE`  : Unbound `PascalCase` identifiers without `_` (`Wrapper { item Elem }` where `Elem`
      is not a defined Type) are rejected; use an open slot `{ item }` or Type Hole `_Elem`.
    ◦ `E06_IMMUTABLE_TYPE_BINDING` : Rebinding a `PascalCase` Archetype in-place (`Car.max_speed .= 500`) is rejected;
      instantiate a `snake_case` value first (`my_car (Car {})`, then `my_car.max_speed .= 500`) or derive a new Type via `&`.
    ◦ `E07_STATIC_ASSERT_FAILED`   : A compile-time stage block (`Compile & [...]`) invoking `Meta.error(msg)` or
      exceeding the totality step budget halts compilation with a source-mapped diagnostic.

3. COMBINATORIAL REFERENCE TABLES (COMPLETE LISTS):

A. THE 12 TRIAD & OVERLOAD VARIATIONS (`Name(Tuple) {Form} [Stream]`):
  1. `Name { ... }`                        -> Concrete Form Prototype / Module / Reactive Topos (`PascalCase`)
  2. `Name(A) [...], Name(B, C) [...]`     -> Overloaded Constructors (`PascalCase`), Univalent Paths, or Multi-Dispatched Functions
  3. `Box { item }` or `Some(value)`       -> Auto-Generic Open Slot (untyped `snake_case` field specializes to `_T`)
  4. `Range { start _T, end _T }`          -> 1st-Order Generic Form (`_T` links `start` and `end` to the same Type)
  5. `Functor(_F(_)) { ... }`              -> 2nd-Order Generic Form / HKT (`_F(_)` is a Type Constructor `* -> *`)
  6. `MonadTrans(_T(_(_), _)) { ... }`     -> 3rd-Order Generic Form (`_T` has Kind `(* -> *) -> * -> *`)
  7. `Categorized(_TC & Category, c _TC)`  -> Bounded Generic Form (`_TC & Category` constrains `_TC <: Category`)
  8. `UserWithAge(Age) { new_age age }`    -> Auto-Projected Parameterized Form (single `Age` projects `age` via Axiom II.3b)
  9. `name { a Int, b Int 0 } [Body]`      -> Named Function with Form slots (callable via `name { a 1 }` or `name(1, 0)`)
 10. `name(args) ReturnType [Body]`        -> Named Function with explicit return type (or parameterless method `name() [Body]`)
 11. `(args) [Body]`, `[.x]`, `[_a + _b]`  -> Explicit Lambda `(args) [Body]`, Relative Map `[ .+ 1 ]`, or Emergent Fold `[ _a + _b ]`
 12. `name(args) ReturnType` / `(_A) [_B]` -> Abstract Method Contract inside a Trait Form `{}` / Function Type Signature

B. POLYMORPHIC HOLES & CONSTRAINTS (`_`, `_T`, `_a` + `&`):
  1. Anonymous Hole (`_`):
     - Unconstrained        : `_` (matches `Any` value/type, or kind slot in `_F(_)`)
     - Ancestor Constraint  : `_ & Vehicle` (any Type that inherits from `Vehicle`)
     - Structural Constraint: `_ & { id String }` (any Form possessing field `id String`)
     - Stream Constraint    : `_ & Int & [ .> 0 ]` (any `Int` satisfying `[ .> 0 ]`)
  2. Named Type Hole (`_T`):
     - Unconstrained        : `Pair { a _T, b _T }` (links multiple positions to the exact same Type `_T`)
     - Ancestor Constraint  : `_T & Category` or F-bounded `_T & Comparable(_T)`
     - Structural Constraint: `_T & { id String, name String }`
     - Stream Constraint    : `_T & String & [ .byte_len > 0 ]`
  3. Anonymous Parameter Hole (`_a`, `_b`):
     - Unconstrained        : `[ _a + _b ]` (desugars to positional lambda `(_a, _b) [ _a + _b ]` or emergent fold on `Tuple`)
     - Ancestor Constraint  : `[ (_a & Vehicle).brand ]`
     - Structural Constraint: `[ (_a & { price Money }).price ]`
     - Stream Constraint    : `[ (_a & Int & [ .> 0 ]) * 2 ]`

C. THE 7 INTERSECTION (`&`) COMBINATIONS:
  1. `FormA & FormB`       -> Lineage Inheritance, Trait Implementation, Same-Name Overload Merge, or Structural Merge
  2. `Type & Stream / !T`  -> Refinement (`Int & [ .> 0 ]`), Form Conservation Law (`Account & [ bal >= 0 ]`), or Negative Guard (`Order & !Cancelled`)
  3. `Hole & Constraint`   -> Constrained Hole (`_ & Vehicle`, `_T & Category`, `_ & { id String }`, `_a & Int`)
  4. `Function & Form`     -> Partial Application, Default Override, or Ambient Coeffect Handler (`fn & { Shell MockShell }`)
  5. `StreamA & StreamB`   -> Sequential Pipeline Composition (`trim_str & upper_str`)
  6. `Entity & Directive`  -> Stage, Memory, Reactive Watch, FFI, or Visibility Attachment (`val & Private`, `fh & Linear`, `rule & Watch`, `Compile & [...]`)
  7. `Type & Bypass`       -> Marks Failure/Empty/Control Union Outcome (`NotFound & Bypass`) for Axiom IV short-circuiting

D. THE 6 SUBTRACTION (`/`) COMBINATIONS (vs Runtime Value Division):
  1. `Form / Form.field`   -> Removes specific ancestor field (`User / User.password_hash`)
  2. `Form / 'key'`        -> Removes field or `& Watch` observer by key (`UserAndCloud / 'name'`, `Reactor / 'auto_cool'`)
  3. `Form / AncestorForm` -> Removes entire ancestor slice from Form Lineage (`ElectricCar / Vehicle`)
  4. `Form / TagForm`      -> Removes all members tagged with `TagForm` (`AuthInternal / Private`, `Reactor / Watch`)
  5. `Union / Type`        -> Removes branch from a Union (`Result / Error`)
  6. `map_val / key_val`   -> On runtime `Map`, removes entry key (`config_map / "host"`); on numeric/CSG values, invokes overloaded `/`

E. QUOTES, COMPUTED KEYS & COMPLETE OPERATOR LIST (4 ORTHOGONAL SYMMETRIC LAYERS):
  - `'symbol'` (Compile-Time Identity): Interned `Gpr(8)` O(1) constant tag (`'draft'`, `'GET'`). Zero allocation, no `{}` interpolation.
  - `"string"` (Runtime UTF-8 Text)   : Multiline `Mem(16)` UTF-8 text (`O(1)` `.byte_len`, lazy `.chars` / `.graphemes`) with `{expr}` interpolation.
  - `r"..."`   (Raw Verbatim String)  : Multiline raw text (`r0"..."0`, `r1"..."1`, `r2"..."2`) with zero escapes.
  - `(K) V`    (Dynamic/Type Key)     : Evaluates `K` as a type hole `Map { (_K) _V }` or dynamic value key `x . { (target_val) [ 10 ] }`.
  - `# ...` & `#\n...\n#`             : Homoiconic line and block comments (attached to AST via `Meta.doc_of`).
  - Layer 1 — Pure Math, Logic & Type Algebra (no leading `.`, no trailing `=`):
    `a + b`, `a - b`, `a * b`, `a / b`, `a % b`, `a ** b`, `a << b`, `a >> b`, `a == b`, `a != b`, `a > b`, `a >= b`, `a < b`, `a <= b`, `a && b`, `a || b`, `a & b`, `a | b`, `!x`, `-x`.
  - Layer 2 — Tight Path & Relative Operators inside `[...]` (tight leading `.`, no trailing `=`):
    `a.b`, `a.Parent.field`, `a.0`, `a.-1`, `.field`, `.0`, `.+ b`, `.- b`, `.* b`, `./ b`, `.> b`, `.>= b`, `.< b`, `.<= b`, `.== b`, `.!= b`, `..x`.
  - Layer 3 — Spaced Dataflow Pipelines (spaced leading `.`, no trailing `=`):
    • `a . b`  : UFCS, Univalent Auto-Transport & Emergent Map/Fold Dataflow Pipeline (and `. ..` Fractal Attractor Loop).
    • `a .& b` : Enrich Pipeline (evaluates `b(a)` and purely merges result with `a` via `&` without mutating `a`).
    • `a ./ b` : Strip Pipeline (purely subtracts `b` from `a` via `/` without mutating `a`).
    • `a .> b` : Tap Pipeline (executes side-effect `b(a)` on success and passes original `a` forward; inside `[]`, write `. .> b`).
    • `a .~ b` : Finalize / Defer Pipeline (executes cleanup `b` on BOTH success and `& Bypass`, preserving outcome).
    • `a .? b` / `a .! b` : Positive Filter/Branch/Exit Gate (on TRUE) <-> Negative Guard/Assert Gate (on FALSE/None).
    • `a .| {}`: Structured Parallel Pipeline (concurrent fan-out of branches in `{}`, merged via `&`).
  - Layer 4 — Ergonomic Perceus Value Rebind Operators (always end with `=`):
    • `a .= b` : Direct or deep path (`corp.departments.0.budget .= 2500`) local value rebind (O(1) in-place when `RC == 1`; COW when `RC > 1`).
    • `a += b`, `a -= b`, `a *= b`, `a /= b` : Arithmetic value rebind on `snake_case` bindings (`counter += 1`).
    • `a &= b` : Structural enrichment rebind on `snake_case` bindings (`raw_order &= { amount 25 }`; verifies Form `& [...]` & triggers `& Watch`).

F. SYNTACTIC SUGAR EQUIVALENCES:
  1. Tagged String Sugar     : `Type"..."`          ==> `Type("...")` (e.g. `Money"199.99"` ==> `Money("199.99")`)
  2. UFCS Pipeline Sugar     : `a . fn(b)`          ==> `fn(a, b)` (and `a.fn(b)` ==> `fn(a, b)`)
  3. Point-Free Relative     : `.field` / `.op v`   ==> `[ .field ]` / `[ .op v ]` ONLY when passed directly as a call argument or Router key
  4. Auto-Generic Open Slot  : `Box { item }`       ==> `Box(_T) { item _T }`
  5. Anonymous Hole Lambda   : `[ _a + _b ]`        ==> `(_a, _b) [ _a + _b ]` (and on `Tuple` pipe `tuple . [ _a + _b ]` ==> `tuple . fold([ _a + _b ])`)
  6. Perceus Value Rebind    : `a.p .= v`           ==> `a (a & { p v })` (and `a += b` ==> `a (a + b)`, `a &= b` ==> `a (a & b)` with O(1) reuse when `RC == 1`)
"3

# ==============================================================================
# PART II. APPLIED GUIDE & UNIQUE SYNTAX PATTERNS (SECTIONS 1–6)
# ==============================================================================

# ------------------------------------------------------------------------------
# SECTION 1. AXIOM I & II IN ACTION: THE FRACTAL TRIAD (`()`, `{}`, `[]`),
#            PRIMITIVES, CONSTRUCTOR OVERLOADING, MULTI-DISPATCH & OPERATORS
# ------------------------------------------------------------------------------

# Built-in Directives:
BuildInExample {
  Compile {}                   # Forces total compile-time evaluation (`Compile & [...]` or `fn(...) [...] & Compile`)
  Bypass  {}                   # Marks Union failure/empty/control branches (`None`, `Error`, `Break`, `Return`) that short-circuit via Axiom IV
  Linear  {}                   # Enforces exactly-once consumption before scope exit (`FileHandle & Linear` — E12)
  Watch   {}                   # Marks a named stream inside `{}` as a stigmergic reactive observer triggered on sibling rebind (Axiom II.1)
  Public  {}                   # Public visibility marker
  Private {}                   # Internal visibility marker (stripped via `Form / Private` under Axiom III)
  Extern(abi String "c") {}    # Binds a function signature to a zero-overhead foreign ABI symbol (`& Extern("c")`)
}
# 1. Core Primitives & Quote Symmetry (`'symbol'` vs `"string"` vs `r"raw"`):
String ""
Symbol 'symbol'
Int    0
Float  0.0
Bool   false
Byte   0x00
Any    _

BrandName         String
default_user_name "Alice"

QuoteAndStringSymmetry {
  # Paired Single Quotes `'...'` -> Compile-Time Interned Symbol (`Gpr(8)`, O(1) comparison, zero interpolation):
  method_tag   'POST'
  status_tag   'published'

  # Paired Double Quotes `"..."` -> Runtime UTF-8 String (`Mem(16)`, O(1) `.byte_len`, supports `{expr}` interpolation):
  interpolated "Hello, {default_user_name}!
This string spans multiple lines natively
and escapes \"quotes\" and \{braces\}."

  # Raw Verbatim Strings `r"..."` (`r1"..."1`, `r2"..."2`) -> Zero escapes, zero interpolation:
  regex_pat    r"^[^@]+@[^@]+\.[^@]+$"
  nested_json  r1"{"query": "SELECT * FROM \"users\"", "interpolated": "{ignored}"}"1
  nested_code  r2"Can freely embed both " and "1 without any lexer conflict!"2
}

# 2. Same-Name Constructor Overloading (`Money` and `Color` — Axiom II.2 & E11):
# Multiple declarations sharing the same `PascalCase` name and distinct `()` parameter types merge into one Type!

# 2a. Exact Fixed-Point `Money` (4 decimal places stored as integer micro-units: 1.0000 == 10000):
Money {
  scaled Int 0

  +  (other Money)   [ Money(scaled + other.scaled) ]
  -  (other Money)   [ Money(scaled - other.scaled) ]
  *  (factor Float)  [ Money((scaled.to_float * factor).round_to_int) ]
  /  (divisor Float) [ Money((scaled.to_float / divisor).round_to_int) ]
  == (other Money)   [ scaled == other.scaled ]
}

# Overload 1: Construct `Money` from a formatted String (powers `Money("199.99")` and `Money"199.99"`):
Money(raw String) [
  Money { scaled (raw . Decimal.parse_scale4) }
]

# Overload 2: Construct `Money` from whole dollars and cents `(Int, Int)`:
Money(dollars Int, cents Int) [
  Money { scaled ((dollars * 10000) + (cents * 100)) }
]

# 2b. `Color` with Form fields, grayscale overload, RGB overload, and validated Hex overload:
InvalidColor   { raw String }    & Bypass
NegativePrice  { val Int }       & Bypass
InvariantFault { reason String } & Bypass

Color {
  r Byte 0x00
  g Byte 0x00
  b Byte 0x00
  a Byte 0xFF
}

# Overload 1: Grayscale `Color(Byte)` from single intensity byte:
Color(level Byte) [
  Color { r level, g level, b level, a 0xFF }
]

# Overload 2: Opaque RGB `Color(Byte, Byte, Byte)` from 3 bytes:
Color(r Byte, g Byte, b Byte) [
  Color { r r, g g, b b, a 0xFF }
]

# Overload 3: Validated Smart Constructor `Color(String)` from Hex string (returns `Color | InvalidColor`):
Color(code String) Color | InvalidColor [
  (code . starts_with("#")) && (code.byte_len == 7) .! InvalidColor { raw code }
  Color {
    r (code . byte_slice(1, 3) . Byte.parse_hex)
    g (code . byte_slice(3, 5) . Byte.parse_hex)
    b (code . byte_slice(5, 7) . Byte.parse_hex)
  }
]

# 2c. Validated Smart Constructor on `ProductSku`:
ProductSku {
  code  String
  price Money
}

ProductSku(raw_code String, raw_price Money) ProductSku | NegativePrice [
  raw_price.scaled >= 0 .! NegativePrice { val raw_price.scaled }
  ProductSku {
    code  (raw_code . trim . to_upper)
    price raw_price
  }
]

# 2d. Domain Tagged Literals (Sugar #1: `Type"..."` ==> `Type("...")`):
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

constructor_overload_demo() [
  m_default (Money {})                         # Form default -> scaled 0
  m_scaled  Money(1999900)                     # Positional slot -> scaled 1999900
  m_parts   Money(199, 99)                     # Overload (Int, Int) -> 199.99
  m_str     Money("199.99")                    # Overload (String)
  m_sugar   Money"199.99"                      # Tagged string sugar -> Money("199.99")

  c_default (Color {})                         # Form default -> (0, 0, 0, 255)
  c_named   (Color { r 0x2A, g 0x9D })         # Named Form fields
  c_rgba    Color(0x2A, 0x9D, 0x8F, 0xFF)      # 4-arg positional constructor
  c_rgb     Color(0x2A, 0x9D, 0x8F)            # 3-arg RGB overload
  c_gray    Color(0x80)                        # 1-arg Grayscale overload
  c_hex     Color"#2A9D8F"                     # Validated String overload via `Type"..."` sugar
  sku       ProductSku("  item-42 ", Money"199.99")
]

# 3. Form-Tuple Constructor Duality, Function Slots, Defaults & Partial Application (`Function & Form`):
ProfileCard {
  id       String                # Typed Open slot 0
  name     String                # Typed Open slot 1
  meta     _T                    # Generic Open slot 2 (Type Hole `_T`)
  role     Symbol 'user'         # Has default -> optional in positional `()` call
  is_admin [ role == 'admin' ]   # Slotless lazy field `[]` -> participates in `==` & `Meta.fields_of` (Axiom II.1)
}

card_by_form  ProfileCard { id "u1", name "Alice", meta 99 }
card_by_tuple ProfileCard("u1", "Alice", 99)

connect { host String "localhost", port Int 8080, tls Bool false } [
  "Connecting to {host}:{port} (TLS: {tls})"
]

default_conn   connect()
pos_conn       connect("10.0.0.1", 9000, true)
form_conn      connect { port 9090, tls true }
partial_conn   connect("db.internal")
secure_connect (connect & { port 9090, tls true })
active_conn    secure_connect()

# 4. Ordering Trait (`Comparable`), Constrained `Range` & Custom Operators:
Comparable(_Other) {
  <  (other _Other) Bool
  <= (other _Other) Bool [ (comparable < other) || (comparable == other) ]
  >  (other _Other) Bool [ !(comparable <= other) ]
  >= (other _Other) Bool [ !(comparable < other) ]
}

Int    & Comparable(Int)
Float  & Comparable(Float)
Byte   & Comparable(Byte)
String & Comparable(String)

Range {
  start (_T & Comparable(_T))
  end   (_T & Comparable(_T))
  contains(val _T) [ (val >= start) && (val <= end) ]
} & [ start <= end ]

Vec2 {
  x Float
  y Float

  +  (other Vec2)  [ Vec2 { x (x + other.x), y (y + other.y) } ]
  -  (other Vec2)  [ Vec2 { x (x - other.x), y (y - other.y) } ]
  *  (scale Float) [ Vec2 { x (x * scale),   y (y * scale) } ]
  /  (scale Float) [ Vec2 { x (x / scale),   y (y / scale) } ]
  == (other Vec2)  [ (x == other.x) && (y == other.y) ]
}

# Dual of `/` on runtime values: invokes overloaded `/` method (CSG boolean geometry subtraction — Axiom III.2):
Shape {
  vertices Tuple(Vec2)
  / (cutter Shape) [ (vertices, cutter.vertices) . native_csg_subtract ]
}


# ------------------------------------------------------------------------------
# SECTION 2. AXIOM V & IV.3 IN ACTION: CASE PARTITION (`PascalCase` vs `snake_case`),
#            CONSTRAINED HOLES (`_`, `_T`, `_a`), HKT, PERCEUS `.=` & `Weak` / `Linear`
# ------------------------------------------------------------------------------

# 1. Polymorphic Holes (`_`, `_T`, `_a`) & Inline Constraints (`&` — Axiom V.3):
Category { title String }
Vehicle  { brand BrandName, max_speed Int 180, tag "standard" }

# 1a. Auto-Generic Open Slot (`item` specializes to `_T` on construction via Axiom II.2 / Sugar #4):
Box      { item }
PairSame { first _T, second _T }

# 1b. Constrained Type Holes (Ancestor Constraint `_TC & Category` and Structural Constraint `_T & { ... }`):
CategorizedUser(_TC & Category, active_cat _TC) {
  backup_cat _TC
  current    active_cat
}

NamedEntityCard(_T & { id String, name String }) {
  item  _T
  label [ "{item.id}: {item.name}" ]
}

# 1c. Anonymous Hole `_` with Ancestor or Structural Constraints:
accept_any_vehicle(v _ & Vehicle) [
  "Vehicle brand: {v.brand}, max speed: {v.max_speed}"
]

extract_id(entity _ & { id String }) [
  entity.id
]

# 1d. Anonymous Parameter Holes (`_a`, `_b`) with Inline Constraints (`&`):
add_anon          [ _a + _b ]                                  # Desugars to `(_a, _b) [ _a + _b ]`
add_anon_typed    [ (_a & Int) + (_b & Int) ]                  # Parameter holes constrained to `Int`
double_positive   [ (_a & Int & [ .> 0 ]) * 2 ]                # Parameter hole constrained by refinement stream
get_vehicle_speed [ (_a & Vehicle).max_speed ]                 # Parameter hole constrained by ancestor Form
get_priced_amount [ (_a & { price Money }).price ]             # Parameter hole constrained by structural Form

# 2. Recursive ADTs, Weak References, F-Bounded Polymorphism & Higher-Kinded Types (HKT):
None       {} & Bypass
Some       { value _T }
Option(_T) Some(_T) | None

# Weak reference for cycle-free Perceus RC graphs (DOM trees, doubly-linked lists, caches):
Weak(_T) {
  upgrade() Option(_T) [ weak . native_weak_upgrade ]
}

Cons       { head _T, tail List(_T) }
List(_T)   None | Cons(_T)

RoseTree {
  value    _T
  parent   Weak(RoseTree(_T))
  children Tuple(RoseTree(_T)) ()
}

Map {
  (_K) _V
  get(key _K) [ (map, key) . native_map_get ]
  /  (key _K) [ (map, key) . native_map_remove ]
}
Set(_T) { (_T) Bool }

Json String | Int | Float | Bool | None | Tuple(Json) | Map(String, Json)

# F-Bounded Polymorphism (`_T & Comparable(_T)`):
sort(items List(_T & Comparable(_T))) [
  items . native_sort_by([ _a < _b ])
]

# 2nd-Order & 3rd-Order Generics (`Functor`, `Monad`, `OptionT`, `MonadTrans`):
Functor(_F(_)) {
  fmap(fa _F(_A), f (_A) [_B]) _F(_B)
}

Monad(_M(_)) {
  ..Functor(_M)
  pure(a _A)                           _M(_A)
  flat_map(ma _M(_A), f (_A) [_M(_B)]) _M(_B)
}

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
  lift(_M(_) & Monad, ma _M(_A)) _T(_M, _A)
}

# 3. Pure Binding (`a b`) vs Ergonomic Perceus Value Rebind (`.=`, `+=`, `-=`, `&=`) & 1-Tuple `(x,)` (Axiom I.3 & IV.3):
Order      { id String, amount Int }
Department { title String, budget Int }
Company    { name String, departments Tuple(Department) }

update_order_demo() [
  raw_order (Order { id "o1", amount 100 })

  # Ergonomic Local Rebind Operators (`+=`, `-=`, `.=` — desugar to pure Bind, O(1) in-place when `RC == 1`):
  counter 0
  counter += 1
  counter -= 1
  counter .= 10

  # Deep Path & Positional Index Updates (Perceus O(1) in-place reuse when uniquely owned, COW when shared):
  raw_order.amount .= 14
  raw_order.1      .= 20

  # Note `(Department { ... },)` with trailing comma constructs a 1-element `Tuple(Department)` (Axiom I.3):
  corp (Company { name "Acme", departments (Department { title "R&D", budget 1000 },) })
  corp.departments.0.budget .= 2500

  # Local Structural Enrich Rebind (`&= { ... }`) vs Pure Functional Merge (`&`):
  raw_order &= { amount 25 }
  next_order (raw_order & { amount 50 })
]


# ------------------------------------------------------------------------------
# SECTION 3. AXIOM II.3 & IV.2 IN ACTION: DIRECT LEXICAL PROJECTION, EMERGENT FOLD,
#            STIGMERGIC `& Watch` TOPOS REACTIVITY & STRUCTURED CONCURRENCY
# ------------------------------------------------------------------------------

Shell {
  log(msg String)  [ msg . native_print ]
  exec(cmd String) [ cmd . native_exec ]
}

# 1. All 4 Ways Values Enter `{}` and `[]` Without `self`, `this`, or `ctx` (Axiom II.3 & IV.1):
Age      Int
WithName { name String }

UserWithAge(Age) {
  new_age age                                          # Axiom II.3b: Single `Age` auto-projects `age`
}

ProjectionDemo {
  name "Razomy"
  get_name()           [ name ]                        # Axiom II.3a: Direct lexical access to sibling field `name`
  get_via_projection() [ projection_demo.name ]        # Axiom II.3b: `ProjectionDemo` auto-projects `projection_demo`
  get_type_name()      [ Meta.name_of(ProjectionDemo) ]# Compile-time type name reflection via `Meta`
}

read_via_type(WithName)                   [ with_name.name ]                  # Axiom II.3b: `WithName` -> `with_name`
join_two(first WithName, second WithName) [ "{first.name} & {second.name}" ]  # Axiom II.3c: Explicit names for 2+ same types
read_unpacked({ name } WithName)          [ name ]                            # Axiom II.3c: Form field destructuring `{ name }`
sum_pair_pipe(pair (Int, Int))            [ pair . [ .0 + .1 ] ]              # Axiom IV.1: `.0` and `.1` target innermost `[]`

# Tuple & Form Spread `..` in Source vs Rest Capture `..` in Target (Axiom I.4):
origin     (0.0, 0.0)
extra_dims (1.0, 2.0)
point_4d   (..origin, ..extra_dims)

unpacking_demo(nums (Int, Int, Int, Int), user_form WithName) [
  (first_num, second_num, ..rest_nums) nums
  { name, ..other_fields }             user_form
  expanded (0, ..rest_nums, 99)
  "{name}: {first_num + second_num}, count={expanded.length}"
]

# 2. Collection Pipelines & Emergent Arity Routing (1-Hole `[ .+ 10 ]` Map vs 2-Hole `[ _a + _b ]` Fold — Axiom IV.2):
Item { name String, price Int, in_stock Bool }

order_list (
  Item { name "Laptop", price 1200, in_stock true }
  Item { name "Mouse",  price 25,   in_stock false }
  Item { name "Screen", price 400,  in_stock true }
)

calculate_total_in_stock(items Tuple(Item)) [
  items
  .? .in_stock                                         # Positive Gate filters collection by `.in_stock`
  .  [ .price ]                                        # 1-Hole Stream on Tuple -> Element-wise Map
  .  [ _a + _b ]                                       # 2-Hole Stream on Tuple -> Emergent Fold/Reduce (0 on empty Tuple(Int))!
]

# 3. Stigmergic Reactive Forms (`& Watch` Observers) & Lazy Getters (Replaces external `Signal` & `Effect` — Axiom II.1):
Invoice {
  price    Money"100.00"
  quantity 3.0
  tax_rate 0.20

  subtotal [ price * quantity ]
  tax      [ subtotal * tax_rate ]
  total    [ subtotal + tax ]
}

# Stigmergic Self-Organizing Reactor: named `& Watch` streams resonate automatically when `temp` or `pressure` updates!
ThermalReactor {
  temp          Float 20.0
  pressure      Float 1.0
  cooling_valve Bool  false
  alarm_active  Bool  false

  # Named Reactive Observer 1: opens `cooling_valve` via Perceus rebind whenever `temp` exceeds 100.0
  auto_cool [
    (temp > 100.0) .? [ cooling_valve .= true ]
  ] & Watch

  # Named Reactive Observer 2: triggers alarm if both `temp` and `pressure` reach critical thresholds
  pressure_guard [
    ((temp > 150.0) && (pressure > 5.0)) .? [ alarm_active .= true ]
  ] & Watch
}

# Axiom III.2 Subtraction `/` can surgically strip `& Watch` observers for isolated unit testing!
PassiveTestReactor (ThermalReactor / Watch)
NoAlarmReactor     (ThermalReactor / 'pressure_guard')

reactor_demo() [
  core (ThermalReactor {})
  core.temp .= 120.0                                   # Automatically resonates `auto_cool` -> `core.cooling_valve` becomes `true`!
]

ReactiveCounter {
  count   Int 2
  doubled [ count * 2 ]
  on_tick [ "Count: {count}, Doubled: {doubled}" . Shell.log ] & Watch
}

run_reactive_demo() [
  sig (ReactiveCounter {})
  sig.count .= 10                                      # Triggers `on_tick` -> logs "Count: 10, Doubled: 20"
  sig.count += 5                                       # Triggers `on_tick` -> logs "Count: 15, Doubled: 30"
]

# 4. Structured Concurrency (`.| {}`), Session-Dual Channels (`!Protocol`) and Lock-Free `Atom`:
fetch_profile(id String)         [ Url"https://api.ex/users/{id}" . Http.get ]
fetch_recommendations(id String) [ Url"https://api.ex/recs/{id}" . Http.get ]
fetch_ads()                      [ Url"https://api.ex/ads" . Http.get . timeout(Duration"2s") ]

# Structured Fan-Out `.| {}`: if any branch fails with `Error & Bypass`, sibling branches are cancelled automatically:
load_dashboard_data(user_id String) [
  user_id .| {
    profile [ user_id . fetch_profile ]
    recs    [ user_id . fetch_recommendations ]
    ads     [ fetch_ads() ]
  }
]

Channel(_T) {
  send(val _T)   [ (channel, val) . native_chan_send ]
  recv()         [ channel . native_chan_recv ]
  select(routes) [ routes . native_chan_select ]
}

# Tail-Call-Optimized CSP worker loop & channel multiplexing (`Channel.select`):
worker_loop(jobs Channel(String), quit Channel(Bool), results Channel(Int)) [
  Channel.select {
    (jobs.recv()) [
      . {
        None      [ "Jobs channel closed, terminating worker." . Shell.log ]
        Some(job) [
          job.byte_len . results.send
          worker_loop(jobs, quit, results)
        ]
      }
    ]
    (quit.recv()) [ "Quit signal received." . Shell.log ]
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


# ------------------------------------------------------------------------------
# SECTION 4. AXIOM III IN ACTION: ALGEBRAIC LINEAGE (`..`, `&`, `/`), NOETHER
#            CONSERVATION LAWS, BIJECTIVE STREAM INVERSION (`!`) & MODULES / DI
# ------------------------------------------------------------------------------

# 1. Form Composition via Prefix Spread `..Ancestor` or Binary `&` & Zero-Cost Lineage History:
HasBattery { battery_kwh Int 75 }

ElectricVehicle {
  ..Vehicle
  ..HasBattery
  charging_port String "Type2"
}

Car       Vehicle   { brand "GenericMotor", max_speed 200 }
SportsCar Car       { brand "Porsche",      max_speed 300 }
MyCar     SportsCar { brand "CustomPorsche" }

# Tight dot `.` on Form Prototypes accesses active defaults OR any compile-time ancestor layer (Axiom III.1):
active_brand   MyCar.brand                   # "CustomPorsche"
sports_brand   MyCar.SportsCar.brand         # "Porsche"
base_car_brand MyCar.SportsCar.Car.brand     # "GenericMotor"
root_spec_type MyCar.Vehicle.brand           # BrandName

MyGarageCar (MyCar & { garage_slot 1 })
first_field MyGarageCar.0                    # "CustomPorsche" (positional index 0)
last_field  MyGarageCar.-1                   # 1 (positional index -1 from end)

# 2. Noether Conservation Laws on Forms (`Form & [Invariant]`) & Bijective Stream Inversion (`!stream` — Axiom III.1 & III.3):

# 2a. Double-Entry Ledger Conservation Law: `assets - liabilities == equity` is enforced across all `.=` / `&=` updates!
BalanceSheet {
  assets      Money Money"1000.00"
  liabilities Money Money"400.00"
  equity      Money Money"600.00"
} & [ (assets - liabilities) == equity ]

record_loan(sheet BalanceSheet, loan Money) [
  # Updating both fields in a single `&=` atomic rebind preserves the Noether invariant `(assets - liabilities) == equity`:
  sheet &= {
    assets      (sheet.assets + loan)
    liabilities (sheet.liabilities + loan)
  }
]

# 2b. Bijective Stream Inversion (`!stream`) & Univalent Transport Paths:
celsius_to_fahrenheit [ (. * 1.8) + 32.0 ]
fahrenheit_to_celsius !celsius_to_fahrenheit           # Synthesizes inverse stream `[ (. - 32.0) / 1.8 ]`!

UserDto   { user_id String, full_name String }
UserModel { id String,      name String }

model_to_dto (UserModel) [ UserDto { user_id user_model.id, full_name user_model.name } ]
dto_to_model !model_to_dto                             # Synthesizes inverse structural mapping `UserDto -> UserModel`!

# 3. Trait Implementation (`Form & Trait`), Stream Composition (`Stream & Stream`) & Open UFCS Methods:
Role         'admin' | 'manager' | 'user' | 'guest'
Identifiable { id String }

User Identifiable {
  id            String
  name          String
  age           Int
  role          Role
  password_hash String
}

Printable {
  format() String
}

PrintableUser User & Printable {
  format() [ "User({name}, id={id})" ]
}

# Disambiguation Rule (Axiom II.3c / E02): `other User` is named explicitly because enclosing `User` projects `user`:
SortableUser User & Comparable(User) {
  < (other User) [ age < other.age ]
}

# Sequential Stream Composition via `&` vs Inline Stream Spread `..`:
trim_str      [ .trim ]
upper_str     [ .to_upper ]
clean_str     (trim_str & upper_str)
clean_inlined [ ..trim_str, ..upper_str ]

# Open UFCS Extension Method:
greet(User) [ "Welcome, {user.name}!" ]

# 4. Structural Type Subtraction (`/`), Map Key Subtraction & Lineage Collision Resolution (Axiom III.2):
Cloud Identifiable {
  id     String
  name   String
  region String
}

PublicUser        User / User.password_hash
AnonymousUsr      PublicUser / 'name'
UserAndCloud      User & Cloud
UserWithCloudName UserAndCloud / User.name
RestoredUser      UserAndCloud / Cloud
GuestUser         (PublicUser / PublicUser.role) & { role 'guest' }

config_map { "host" "localhost", "port" "8080" }
updated_config (config_map & { "port" "9090", "tls" "true" }) / "host"

# Resolving colliding `id` fields on `User & Order` via explicit ancestor path `User.id` and `Order.id` (E05):
UserOrder      User & Order
UserOrderClean (UserOrder / Order.id)

process_combined(UserOrder) [
  "Client Name: {user_order.name}, Order Amount: {user_order.amount}" . Shell.log
  "User ID: {user_order.User.id}, Order ID: {user_order.Order.id}" . Shell.log
]

# 5. Pipeline Enrich (`.&`), Strip (`./`) & Tap (`.>`):
Cart     { cart_id String, amount Int }
UserCart User & Cart

finalize_bill(UserCart) [
  user_cart .& {
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

# 6. Imports, Prefix Lineage Mixin (`..Mod`), Parameterized DI Modules & Ambient Coeffect Handlers:
Base   root.base
Http   std.net.http
ExtMod Path"../shared/ext_mod.rl"

{ String, Int, Money } root.base
..root.prelude
..(std.net.http / 'legacy_request')

AuthInternal {
  salt "x9f2a" & Private
  hash(raw String)         [ "{raw}_{salt}" . native_sha256 ] & Private
  verify(u User, p String) [ (p . hash) == u.password_hash ]
}

AuthModule (AuthInternal / Private)

DbDriver(conn_url String, max_pool Int 10) {
  active_conns Atom(Int) { value 0 } & Private
  query(sql String) [ "Running '{sql}' on {conn_url} (pool={max_pool})" . Shell.log ]
}

primary_db (DbDriver { conn_url "postgres://prod:5432/app", max_pool 32 })

# Ambient Coeffect Handler (`Function & Form` overrides ambient capabilities like `Shell` with zero keywords):
audit_action(User) [ "Audited user: {user.name}" . Shell.log ]
silent_audit (audit_action & {
  Shell { log(msg String) [ "Captured silently: {msg}" ] }
})


# ------------------------------------------------------------------------------
# SECTION 5. AXIOM IV & V IN ACTION: FRACTAL ATTRACTOR LOOPS (`. ..`), GATES (`.?` / `.!`),
#            FINALIZER (`.~`), `Linear` RESOURCES, ROUTERS, DIALECTS & `Meta`
# ------------------------------------------------------------------------------

# 1. Symmetric Gates (`.?` <-> `.!`), Fractal Self-Loop (`. ..` inside `[]`) & Scoped `Break` / `Return`:
Break      { value _T, label Symbol 'default' } & Bypass
Return     { value _T }                         & Bypass
NotFound   { entity String, id String }         & Bypass
MinorError { name String }                      & Bypass
EmptyError {}                                   & Bypass

# 1a. Keywordless Fractal Attractor Loop (`. ..`):
# Finds the square root of `target` by feeding the stream's output back into itself (`. ..`) until convergence!
newton_sqrt(target Float) Float [
  (target / 2.0) . [
    next_guess ((. + (target / .)) / 2.0)
    ((next_guess - .).abs < 0.00001) .? Break(next_guess)
    next_guess . ..
  ]
]

# 1b. `Loop` is not a compiler keyword—any `[ ... . .. ]` is already an attractor loop, or callable via `Loop([...])`:
Loop(body Stream) [ . body . .. ]

process_checkout { user User, items Tuple(Int) } [
  user.age >= 18   .! MinorError { name user.name }
  items.length > 0 .! EmptyError {}

  total 0
  i     0
  [
    i >= items.length .? Break(total)
    total += items.get(i)
    i     += 1
    . ..
  ]
]

early_return_fold(numbers Tuple(Int), limit Int) [
  limit <= 0 .? Return(0)
  numbers . fold(0, (acc Int, n Int) [
    next_acc (acc + n)
    next_acc > limit .? Break(next_acc)
    next_acc
  ])
]

# 2. Refinement Types, Negative Type Guards (`& !State`), Guarded Local Bind & Leak-Free `Linear` Finalizers (`.~`):
Adult   User & [ .age >= 18 ]
Minor   User & [ .age < 18 ]
Manager User & [ .role .? ('admin' | 'manager') ]

classify_account(account User) [
  account . {
    Manager [ "Dashboard: {manager.name}" . Shell.log ]
    Adult   [ "Adult account: {adult.name}" . Shell.log ]
    Minor   [ "Child account: {minor.name}" . Shell.log ]
    _       [ "Unknown profile" . Shell.log ]
  }
]

Db {
  find_user(id String) User | NotFound [
    User { id id, name "Alice", age 25, role 'user', password_hash "xyz" }
  ]
  find_cart(Identifiable) [
    Cart { cart_id "c_for_{identifiable.id}", amount 2500 }
  ]
}

process_order_pipeline(user_id String, db Db) [
  user_id
  .  db.find_user
  .? [ .age >= 18 ] .! MinorError { name "underage" }
  .& db.find_cart
  .  finalize_bill
]

# Flat `do`-notation via Guarded Local Binding (`NotFound & Bypass` short-circuits automatically — Axiom I.1):
load_user_and_friend(uid1 String, uid2 String) [
  u1 User (uid1 . Db.find_user)
  u2 User (uid2 . Db.find_user)
  "{u1.name} is friends with {u2.name}"
]

# Typestate & Closed-World Negative Type Guard (`Order & !Cancelled` — Axiom III.3):
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

# Linear Resource Lifetimes (`FileHandle & Linear`) & Guaranteed Finalize Pipe (`.~` — Axiom IV.2 & E12):
FileHandle { fd Int, path Path } & Linear
IoError    { msg String }        & Bypass

Fs {
  open(path Path)   FileHandle | IoError [ path . native_fs_open ]
  read(FileHandle)  String | IoError     [ file_handle.fd . native_fs_read ]
  close(FileHandle)                      [ file_handle.fd . native_fs_close ]

  # `fh . action` borrows linear `fh`; `.~` runs `[ fh . Fs.close ]` on BOTH success and `IoError & Bypass`, consuming `fh`:
  with_file(path Path, action) [
    fh FileHandle (path . Fs.open)
    fh . action
    .~ [ fh . Fs.close ]
  ]
}

# 3. Pattern Guards, Finite State Machines (FSM) & Deep Structural HTTP Routing:
# Pattern Guards `(Pattern) & [Predicate]` inside a Form Router (Axiom III.1 & IV.2):
compare_pair(pair (Int, Int)) [
  pair . {
    (a, b) & [ a > b ]  [ "First is larger: {a} > {b}" ]
    (a, b) & [ a == b ] [ "Both are equal: {a}" ]
    (a, b)              [ "Second is larger: {b} > {a}" ]
  }
]

# Finite State Machine via Tuple Pattern Router `(state, event) . { ... }`:
Disconnected  { reason String }
Connecting    { host String, attempt Int }
Connected     { host String, session_id String }
ConnState     Disconnected | Connecting | Connected

ConnectCmd    { host String }
HandshakeOk   { session_id String }
NetworkError  { reason String } & Bypass
DisconnectCmd {}
ConnEvent     ConnectCmd | HandshakeOk | NetworkError | DisconnectCmd

step_connection(state ConnState, event ConnEvent) ConnState [
  (state, event) . {
    (Disconnected, ConnectCmd) [ Connecting { host connect_cmd.host, attempt 1 } ]
    (Connecting, HandshakeOk)  [ Connected { host connecting.host, session_id handshake_ok.session_id } ]
    (Connecting, NetworkError) [ Disconnected { reason network_error.reason } ]
    (Connected, DisconnectCmd) [ Disconnected { reason "User closed" } ]
    _                          [ state ]
  }
]

# Deep Structural & Tuple Destructuring in HTTP Router:
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
    { method 'GET', path ("health",) } [
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

# 4. Built-In Homoiconic Dialects: `Parse` (Stream PEG Grammar) & `Ui` (Declarative View/Canvas):
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

ParseError { input String, reason String } & Bypass
LogEntry   { date String, level String, msg String }

parse_log_line(line String) LogEntry | ParseError [
  line . Parse.run(
    Parse.copy('date', Parse.to(" "))
    & Parse.skip(1)
    & Parse.match("[")
    & Parse.copy('level', Parse.to("]"))
    & Parse.skip(2)
    & Parse.copy('msg', Parse.to(Parse.end))
  )
]

UiWidgets {
  heading(text String)                  { widget 'heading', text text }
  button(label String, on_click Stream) { widget 'button',  label label, on_click on_click }
  row(..children)                       { widget 'row',     children children }
  canvas(size Pair, on_draw Stream)     { widget 'canvas',  size size, on_draw on_draw }

  pen(color Hex)                        { draw_cmd 'pen',      color color }
  fill_pen(color Hex)                   { draw_cmd 'fill_pen', color color }
  circle(center (Int, Int), radius Int) { draw_cmd 'circle',   center center, radius radius }
}

ViewSpec {
  ..UiWidgets
  title String
  size  Pair
  body  Tuple(Any)
}

Ui {
  ..UiWidgets
  view(spec ViewSpec) [ spec . native_ui_view ]
  draw(..commands)    [ commands . native_ui_draw ]
}

counter_app(state Atom(CounterState)) [
  Ui.view(ViewSpec {
    title "Razomy Counter"
    size  Pair"1920x1080"
    body  (
      heading("Current Count: {state.read().count}"),
      row(
        button("Increment", [ state.swap((s CounterState) [ update_counter(s, Increment { by 1 }) ]) ]),
        button("Reset",     [ state.swap((s CounterState) [ update_counter(s, Reset {}) ]) ])
      ),
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

# 5. Compile-Time Metaprogramming (`Compile &`, `Meta.rewrite`), `Bind` Interception & Ontology Rewriting:
Meta {
  type_of(entity)          [ entity . native_meta_type_of ]            & Compile
  name_of(entity)          [ entity . native_meta_name_of ]            & Compile
  doc_of(entity)           [ entity . native_meta_doc_of ]             & Compile
  span_of(entity)          [ entity . native_meta_span_of ]            & Compile
  fields_of(entity)        [ entity . native_meta_fields_of ]          & Compile
  steps_of(stream)         [ stream . native_meta_steps_of ]           & Compile
  map_fields(form, mapper) [ (form, mapper) . native_meta_map_fields ] & Compile
  compose(stream)          [ stream . native_meta_eval_parens ]        & Compile
  gensym(prefix Symbol)    [ prefix . native_meta_gensym ]             & Compile
  error(msg String)        [ msg . native_meta_compile_error ]         & Compile
  identity                 [ .native_identity ]

  rewrite(node, rules) [
    walked (node . {
      Form   [ node . Meta.map_fields((k, v) [ (k, v . Meta.rewrite(rules)) ]) ]
      Tuple  [ node . map([ .Meta.rewrite(rules) ]) ]
      Stream [
        node . Meta.steps_of
        . map([ .Meta.rewrite(rules) ])
        . fold(Meta.identity, [ _acc & _step ])
      ]
      _      [ node ]
    })

    walked . (rules & { _ [ walked ] })
  ] & Compile
}

# 5a. Stream Step Instrumentation (`Meta.steps_of`):
trace_execution(code_block) [
  code_block
  . Meta.steps_of
  . map((step) [
    step & [ . .> ((res) [ "Step result: {res}" . Shell.log ]) ]
  ])
  . fold(Meta.identity, [ _acc & _step ])
] & Compile

# 5b. Compile-Time Trait Derivation (`derive_json` via `Meta.fields_of`):
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

# 5c. Intercepting Space Binding (`RazomySpec.Compiler.Bind`) & Rewriting AST Nodes:
Resource { kind String "Milk" }
Cat      { status String }
Factory  { + (res Resource) [ Cat { status "Fed with {res.kind}" } ] }

abstract_law [ a (b + c) ]

specialize_law(code_stream) [
  code_stream . Meta.rewrite({
    'a'     [ cat_instance ]
    'b'     [ Factory {} ]
    'c'     [ Resource { kind "Fish" } ]
    (x + y) [ x + y ]
    RazomySpec.Compiler.Bind(target, val) [
      RazomySpec.Compiler.Bind(target, val)
      & [ "Bound {Meta.name_of(target)} to {target}" . Shell.log ]
    ]
  })
] & Compile

run_cat_factory (Compile & [ abstract_law . specialize_law ])

# 5d. Distributed Ontology Projection (`ServerKernel` -> `EdgeClientKernel`):
NodeId    String & [ .byte_len > 0 ]
Timestamp Int    & [ .> 1700000000 ]

Entity     { id NodeId, version Int 1, created_at Timestamp }
Replicated { ..Entity, region Symbol 'eu_central', state 'synced' | 'drifted', secret_key Hex & Private }
Audited    { ..Entity, auditor String "daemon",    state 'verified' | 'flagged', signature Hex & Private }

LedgerNode Replicated & Audited & {
  payload    Json
  is_healthy [ (ledger_node.Replicated.state == 'synced') && (ledger_node.Audited.state == 'verified') ]
}

ConsensusFault { node_id NodeId, reason String } & Bypass

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

# By Axiom II.3b, subtracted lineage `(LedgerNode / Private) / Audited` preserves root projection `ledger_node`:
derive_edge_client(_KernelForm) [
  _KernelForm . Meta.rewrite({
    LedgerNode          [ (LedgerNode / Private) / Audited ]
    (.Replicated.state) [ .state ]
    (Storage.write)     [ RpcClient.send & [ .timeout(Duration"3s") ] ]
  })
] & Compile

EdgeClientKernel (Compile & [ ServerKernel . derive_edge_client ])

# 6. Autonomous Self-Healing Build Agent Capstone (Using Fractal Attractor `. ..`):
Circle   { radius Float }
Rect     { width Float, height Float }
Geometry Circle | Rect

area(g Geometry) [
  g . {
    Circle [ 3.14159 * circle.radius * circle.radius ]
    Rect   [ rect.width * rect.height ]
  }
]

CompileError { code Int, log String } & Bypass
BuildSuccess { status String, artifact String }
BuildResult  BuildSuccess | CompileError

FatalError   CompileError & [ .code >= 500 ]
Warning      CompileError & [ .code < 500 ]

AiQuery      { system_prompt String, user_prompt String }
AiResponse   { diff String, explanation String }
AgentState   { err CompileError, retries Int, applied_patches Tuple(String) () }

Ai { call(query AiQuery) AiResponse [ query . native_ai_llm_request ] }
run_build() BuildResult [ "cargo build --release" . Shell.exec . native_parse_build_result ]

# Self-healing agent loop using Fractal Attractor `. ..` instead of manual recursion:
agent_loop(initial_state AgentState) [
  initial_state . [
    .retries == 0 .? [
      "Retry limit reached. Human intervention required." . Shell.log
      Break(None)
    ]

    ai_resp (
      AiQuery {
        system_prompt "Fix the compilation error with a minimal git diff patch."
        user_prompt   "Error log: {.err.log}"
      }
      . Ai.call
      .> [ "AI solution: {.explanation}" . Shell.log ]
      .> [ "git apply {.diff}" . Shell.exec ]
    )

    run_build() . {
      BuildSuccess [
        "Build succeeded after applying patch!" . Shell.log
        Break(build_success)
      ]
      CompileError [
        (. & {
          err             compile_error
          retries         (.retries - 1)
          applied_patches (.applied_patches . append(ai_resp.diff))
        }) . ..
      ]
    }
  ]
]

main() [
  run_build() . {
    FatalError   [ agent_loop(AgentState { err fatal_error, retries 3 }) ]
    Warning      [ "cargo fix --allow-no-vcs" . Shell.exec ]
    BuildSuccess [ "Ready for release: {build_success.artifact}" . Shell.log ]
  }
]


# ------------------------------------------------------------------------------
# SECTION 6. COMPILER ERROR CATALOG (`E01`–`E14`) & EDGE-CASE GUARANTEES
# ------------------------------------------------------------------------------

# 1. E01_AMBIGUOUS_CALL_OR_BIND — Spaced 1-arg callable with compatible argument type without type guard (Axiom I)
edge_case_e01() [
  # ERROR `E01_AMBIGUOUS_CALL_OR_BIND`: `area (Circle { radius 5.0 })` (`area` accepts `Circle`)
  res1        area(Circle { radius 5.0 })
  res_typed   Float (Circle { radius 5.0 } . area)
  circle_area (Circle { radius 5.0 } . area)
]

# 2. E02_DUPLICATE_UNNAMED_TYPE — Multiple identical `PascalCase` types in same or enclosing scope (Axiom II)
Account { iban String, balance Money }
# ERROR `E02_DUPLICATE_UNNAMED_TYPE`: `transfer_bad(Account, Account, Money) [ ... ]`
transfer_safe(source Account, target Account, Money) [
  source.balance -= money
  target.balance += money
  (source, target)
]

# 3. E03_UNKNOWN_CONCRETE_TYPE — Unprefixed generic type identifier (`Elem` instead of open slot or `_Elem` — Axiom V)
# ERROR `E03_UNKNOWN_CONCRETE_TYPE`: `Wrapper(Elem) { item Elem }`
Wrapper      { item }
TypedWrapper { item _Elem }
UserWrapper(_U & User, current_user _U) { active _U current_user }

# 4. E04_UNHANDLED_UNION_BRANCH — Unrouted domain branch not marked with `& Bypass` (Axiom IV)
fetch_actor(id String) User | Company | NotFound [
  User { id id, name "Alice", age 30, role 'admin', password_hash "hash" }
]
send_welcome_email(User) [ "Sent email to {user.name}" ]

handle_actor_pipeline(id String) [
  # ERROR `E04_UNHANDLED_UNION_BRANCH`: `id . fetch_actor . send_welcome_email` (`Company` is not `& Bypass`)
  id . fetch_actor . {
    User    [ user . send_welcome_email ]
    Company [ "Sent corporate onboarding to {company.name}" ]
  }
]

# 5. E05_AMBIGUOUS_LINEAGE_FIELD — Unqualified access to colliding field on merged lineage (Axiom III)
resolve_diamond_demo(entity User & Cloud) [
  # ERROR `E05_AMBIGUOUS_LINEAGE_FIELD`: `entity.name . Shell.log`
  "User: {entity.User.name}, Cloud: {entity.Cloud.name}" . Shell.log
  clean_entity (entity / Cloud.name)
  clean_entity.name . Shell.log
]

# 6. E06_IMMUTABLE_TYPE_BINDING — Rebinding an immutable `PascalCase` Form Prototype in-place (Axiom V)
immutable_demo() [
  # ERROR `E06_IMMUTABLE_TYPE_BINDING`: `Car.max_speed .= 500`
  my_car (Car {})
  my_car.max_speed .= 500
]

# 7. E07_STATIC_ASSERT_FAILED — Compile-time invariant check via `Compile &` and `Meta.error` yielding verified Archetype (Axiom V)
VerifiedPublicUser (Compile & [
  fields (PublicUser . Meta.fields_of . [ .name ])
  (fields . contains("password_hash")) . {
    true  [ Meta.error("SECURITY VIOLATION: PublicUser leaked password_hash!") ]
    false [ PublicUser ]
  }
])

# 8. E08_AMBIGUOUS_ROUTER_KEY — Bare `snake_case` identifier in Router key position (Axiom IV)
router_key_demo(x Int, target_val Int) [
  # ERROR `E08_AMBIGUOUS_ROUTER_KEY`: `x . { target_val [ "Matched" ], _ [ "No" ] }`
  x . {
    .== target_val [ "Matched" ]
    (target_val)   [ "Also valid via dynamic key parens" ]
    _              [ "No" ]
  }
]

# 9. E09_DANGLING_BIND_NEWLINE — Newline between target key and opening bracket `[` / `{` / `(` (Axiom I)
newline_bind_demo(amount Int) [
  # ERROR `E09_DANGLING_BIND_NEWLINE`: `discount \n [ amount - 10 ]`
  discount [ amount - 10 ]
  discount()
]

# 10. E10_MULTIPLE_REST_PATTERNS — Multiple `..rest` captures on the left side of a destructuring bind (Axiom I)
rest_pattern_demo(nums (Int, Int, Int, Int)) [
  # ERROR `E10_MULTIPLE_REST_PATTERNS`: `(first, ..r1, ..r2) nums`
  (first, ..rest) nums
  combined (first, ..rest, ..rest)
  combined
]

# 11. E11_OVERLOAD_SIGNATURE_CONFLICT — Duplicate or overlapping parameter signature on same-name overload (Axiom II)
# ERROR `E11_OVERLOAD_SIGNATURE_CONFLICT`: Declaring `Color(intensity Byte) [...]` when `Color(level Byte) [...]` already exists.

# 12. E12_UNCONSUMED_LINEAR_VALUE — Dropping or bypassing a `& Linear` resource without finalization (Axiom IV)
linear_safety_demo(path Path) [
  # ERROR `E12_UNCONSUMED_LINEAR_VALUE`: Opening `fh FileHandle (path . Fs.open)` without consuming `fh` via `.~ [ fh . Fs.close ]`
  Fs.with_file(path, [ .Fs.read ])
]

# 13. E13_CONSERVATION_LAW_VIOLATION — Violating a Form's Noether conservation invariant `& [...]` (Axiom III)
conservation_demo() [
  # ERROR `E13_CONSERVATION_LAW_VIOLATION`: `Range { start 50, end 10 }` violates `Range & [ start <= end ]`
  valid_range (Range { start 10, end 50 })
  valid_range.end .= 100
]

# 14. E14_NON_INVERTIBLE_STREAM — Applying `!` inversion to a lossy/non-bijective stream (Axiom III)
invertibility_demo() [
  # ERROR `E14_NON_INVERTIBLE_STREAM`: `bad_inverse ![ . * 0 ]`
  temp_f (100.0 . celsius_to_fahrenheit)
  temp_c (temp_f . !celsius_to_fahrenheit)
]


# ==============================================================================
# PART III. SELF-HOSTED ONTOLOGY OF RAZOMY v9.0 (`RazomySpec` — THE LANGUAGE IN CODE)
# ==============================================================================

RazomySpec {
  # ============================================================================
  # LAYER 1. COMPILER CORE, TRIAD GRAMMAR, HOLES & OPERATORS (AXIOMS I–V)
  # ============================================================================
  Compiler {
    # 1. Primary Homoiconic AST Root, Hygiene & Lexical Tokens (Axiom I & V):
    SourceSpan    { file Symbol, start_byte Int, end_byte Int }
    Node          { span SourceSpan }           # Universal homoiconic AST unit with hygienic source location
    Token         Node                          # Atomic lexical unit
    Sep           Token                         # `,` or `\n` (splits elements; joins before binary/pipe op — E09)
    Literal       Token                         # Constant literal (`0`, `0.0`, `'sym'`, `"str"`, `r"raw"`)
    Identifier    Token                         # Named lexical token

    # 2. Case & Polymorphic Hole Partition (Axiom V):
    TypeKey         Identifier                  # `PascalCase`: Concrete Types, Form Prototypes, Modules (Fixed at runtime — E06)
    ValueKey        Identifier                  # `snake_case`: Variables, Field Keys, Functions (Rebound via `.=`, `+=`, `&=`)
    AnonHole        Token                       # `_`         : Anonymous Hole (`Any`, `_ & Vehicle`, `_F(_)`)
    TypeHole        Identifier                  # `_PascalCase` (`_T`, `_K`, `_V`, `_M`): Named Type Hole (E03)
    ParamHole       Identifier                  # `_snake_case` (`_a`, `_b`): Anonymous Parameter Hole (`[ _a + _b ]`)
    Hole            AnonHole | TypeHole | ParamHole
    ConstrainedHole Hole & Expr                 # Any hole constrained via `&`: `_ & Vehicle`, `_T & Category`, `_a & Int`

    # 3. The Canonical Triad Brackets `()` <-> `{}` <-> `[]` (Axiom II):
    Bracket       Node
    TextQuote     Bracket                       # `'sym'` (O(1) tag) <-> `"..."` (interpolated) <-> `r"..."` (raw) & `Type"..."`
    CommentBlock  Bracket                       # `# ...` (line) <-> `#\n ... \n#` (block); attached via `Meta.doc_of`

    TupleBracket  Bracket                       # `()` — Positional tuple, 1-tuple `(x,)`, slice `Tuple(_T)`, eager `(a + b)`
    FormBracket   Bracket                       # `{}` — Named key-value form: prototypes, traits, modules, routers, reactive Topos
    StreamBracket Bracket                       # `[]` — Executable stream: lazy getters, predicates, attractors, bijections

    # 4. Grammatical Composition, Lineage Spread & Fractal Self-Loop (Axiom I, II, III & IV):
    Repeat(_T)    Node
    Expr          Literal | Identifier | Hole | Tuple | Form | Stream
    OpenSlot      ValueKey | Bind(ValueKey, TypeKey | TypeHole | ConstrainedHole) # Untyped `item` or typed `id String` / `meta _T` (Axiom II.2)
    Pair          (Identifier | Tuple) & Expr   # Static key `k V` <-> Dynamic/Type key `(K) V` or Guard `(P) & [G] V` (E08)
    LineageSpread ExpandSpread & (TypeKey | TypeHole | ((TypeKey | TypeHole) & Tuple))
    AttractorStep Pipe & ExpandSpread           # `. ..` inside `[...]` -> Tail-recursive self-loop back to stream start

    Tuple         TupleBracket  & Repeat(Expr)
    Form          FormBracket   & Repeat(OpenSlot | Pair | LineageSpread)
    Stream        StreamBracket & Repeat(Expr | AttractorStep)

    # 5. Triad Declarations & Same-Name Multi-Dispatch Overloading (Axiom II — E11):
    Slots             Form | Tuple              # Named `{ a Int 0 }` <-> Positional `(a Int)`
    ParameterizedForm TypeKey  & Slots & Form   # `Name(Slots) { ... }`
    Constructor       TypeKey  & Slots & Stream # `Color(Byte) [...]` & `Color(String) [...]` (Overloaded by signature)
    Callable          Slots    & Stream         # Explicit `(args) [Body]` <-> Relative Map `[.x]` <-> Emergent Fold `[_a + _b]`
    Function          ValueKey & Slots & Stream # Named/overloaded function or method `name(args) RetType [Body]`

    # 6. Context-Exact Space Binding Matrix (`' '` == Overloadable AST Node `Bind(Target, Source)` — Axiom I):
    Bind(_Target, _Source) Node
    TypeAliasBind Bind(TypeKey, TypeKey)        # `BrandName String`
    ProtoFormBind Bind(TypeKey, Form)           # `Vehicle { ... }` or `Car Vehicle { ... }`
    FieldSpecBind Bind(ValueKey, (TypeKey | TypeHole | ConstrainedHole) & Expr) # Inside `{}`: `max_speed Int 180`
    LazyFieldBind Bind(ValueKey, Stream)        # Inside `{}`: `is_admin [ role == 'admin' ]` (or `& Watch` observer)
    VarBind       Bind(ValueKey, Expr)          # `my_car (Car {})` (Pure local binding — E01)
    TypedVarBind  Bind(ValueKey & TypeKey, Expr)# Inside `[]`: `fh FileHandle (path . Fs.open)` (Auto-bypasses `& Bypass`)
    RestUnpack    Bind(Tuple | Form, Expr)      # `(first, ..rest) nums` <-> `{ name, ..other } user` (Max 1 `..` on left — E10)

    # 7. Complete Operator Hierarchy (4 Orthogonal Symmetric Layers):
    Operator       Token
    UnaryOperator  Operator
    BinaryOperator Operator

    # 7a. Layer 2 — Tight Prefix Unary & Relative Operators inside `[...]` (Axiom I, III & IV):
    Not            UnaryOperator                # `!x`     — Logical `!flag` <-> Negative Type `!Cancelled` <-> Dual `!Proto` <-> Inverse `!stream` (E14)
    Neg            UnaryOperator                # `-x`     — Arithmetic negation (`-5`, `-(a + b)`)
    RelGet         UnaryOperator                # `.field` — Relative getter/index `.0`/method of innermost `[...]` (Axiom IV.1)
    RelOp          UnaryOperator                # `.+ x`   — Relative operator step (`.+ 1`, `.- 1`, `.> 0`, `.>= 18`, `.== target` — E08)
    ExpandSpread   UnaryOperator                # `..x`    — Spread in Source `{ ..Vehicle }` <-> Rest in Target `(first, ..rest)` <-> Loop `. ..`

    # 7b. Layer 3 — Spaced Dataflow Pipeline Operators (Start with `.`, never end with `=` — Axiom IV.2):
    PipeOperator   BinaryOperator
    TightDot       PipeOperator                 # `a.b`    — Tight path access: field, ancestor `a.Vehicle.id`, index `a.0`, call `a.fn()`
    Pipe           PipeOperator                 # `a . b`  — UFCS, Univalent auto-transport, 1-hole Map `[.+ 1]`, or 2-hole Emergent Fold `[_a + _b]`
    TapPipe        PipeOperator                 # `a .> b` — Side-effect tap on success: runs `b(a)` and passes original `a` forward
    FinalizePipe   PipeOperator                 # `a .~ b` — Guaranteed finalizer (`defer`): runs `b` on BOTH success and `& Bypass` (E12)
    EnrichPipe     PipeOperator                 # `a .& b` — Enrich pipe: evaluates `b(a)` and purely merges result with `a` via `&`
    SubtractPipe   PipeOperator                 # `a ./ b` — Strip pipe: purely removes field `A.k`, key `'k'`, tag `Private`/`Watch`, or ancestor `A`
    PositivePipe   PipeOperator                 # `a .? b` — Positive Gate (TRUE): filters `a` by `b`, runs branch `b`, or exits if `b <: Bypass`
    NegativePipe   PipeOperator                 # `a .! b` — Negative Gate (FALSE/None): asserts `a`, or short-circuits with `b` (`& Bypass`)
    ParallelPipe   PipeOperator                 # `a .| {}`— Structured concurrent fan-out of branches in `{}`, merged via `&`

    # 7c. Layer 4 — Ergonomic Perceus Value Rebind Operators (Always end with `=`; desugar to pure `Bind` + `&` — Axiom IV.3):
    RebindTarget   ValueKey | TightDot          # Only `snake_case` bindings or field/index paths can be rebound (E06)
    RebindOperator BinaryOperator & { target RebindTarget }
    AssignRebind   RebindOperator               # `a .= b` — Direct or deep path `a.b.0 .= val` rebind (`a (a & { b.0 val })`, verifies `& [...]` — E13)
    AddRebind      RebindOperator               # `a += b` — Arithmetic add rebind (`a (a + b)`)
    SubRebind      RebindOperator               # `a -= b` — Arithmetic subtract rebind (`a (a - b)`)
    MulRebind      RebindOperator               # `a *= b` — Arithmetic multiply rebind (`a (a * b)`)
    DivRebind      RebindOperator               # `a /= b` — Arithmetic divide rebind (`a (a / b)`)
    EnrichRebind   RebindOperator               # `a &= b` — Atomic structural enrichment rebind (`a (a & b)`)

    # 7d. Layer 1 — Pure Type Algebra, Conservation, Arithmetic, Comparison & Logic (No `.`, no `=` — Axiom III):
    AlgebraOp      BinaryOperator
    And            AlgebraOp                    # `a & b`  — Universal Intersection / Lineage Merge / Conservation Invariant / Directive
    Or             AlgebraOp                    # `a | b`  — Algebraic Union of types, patterns, or outcomes (Axiom IV — E04)
    Sub            AlgebraOp                    # `a / b`  — Structural Subtraction on Types/Forms/Unions/Watchers (Axiom III.2 — E05)

    MathOp         BinaryOperator
    Plus           MathOp                       # `a + b`
    Minus          MathOp                       # `a - b`
    Mult           MathOp                       # `a * b`
    Div            MathOp                       # `a / b`  — Invokes overloaded `/` on `snake_case` runtime values (dual of Type `/`)
    Mod            MathOp                       # `a % b`
    Pow            MathOp                       # `a ** b`
    ShiftL         MathOp                       # `a << b`
    ShiftR         MathOp                       # `a >> b`

    CompareOp      BinaryOperator
    Eq             CompareOp                    # `a == b` — Structural equality on all Forms `{}` (fields + lazy getters) and Tuples `()`
    Neq            CompareOp                    # `a != b`
    Gt             CompareOp                    # `a > b`
    Gte            CompareOp                    # `a >= b`
    Lt             CompareOp                    # `a < b`
    Lte            CompareOp                    # `a <= b`

    LogicOp        BinaryOperator
    LogicAnd       LogicOp                      # `a && b`
    LogicOr        LogicOp                      # `a || b`

    # 8. Compile-Time, Memory, Stigmergy, FFI, Control Flow & Visibility Directives (Axiom II, III, IV & V):
    Directive           TypeKey
    Compile             Directive               # Evaluates total expression at compile time (`Compile & [...]` — E07)
    Bypass              Directive               # Marks non-happy-path Union outcomes for automatic Axiom IV short-circuiting
    Linear              Directive               # Enforces exactly-once consumption before scope exit (`FileHandle & Linear` — E12)
    Watch               Directive               # Marks a named stream inside `{}` as a stigmergic observer on sibling rebind (Axiom II.1)
    Extern(abi String)  Directive               # Binds foreign ABI symbol (`& Extern("c")`)
    Break               Directive & Bypass & { value _T, label Symbol 'default' }
    Return              Directive & Bypass & { value _T }
    Public              Directive               # Public visibility tag
    Private             Directive               # Internal visibility tag (stripped via `Module / Private`)
  }

  # ============================================================================
  # LAYER 2. HARDWARE REGISTERS, SIZED PRIMITIVES & DOMAIN LITERALS (AXIOM I & II)
  # ============================================================================
  Primitives {
    ..Compiler

    # Top, Bottom & Total Ordering Contract:
    Any                TypeKey                  # Universal Top Type (`_`)
    None               TypeKey & Bypass         # Unit / Empty Outcome Type (auto-bypasses pipelines via Axiom IV)
    Never              !Any                     # Uninhabited Bottom Type
    Comparable(_Other) Form & { < (other _Other) Bool, <= (other _Other) Bool, > (other _Other) Bool, >= (other _Other) Bool }

    # Hardware Register & Memory Layout Descriptors:
    BuiltIn      TypeKey
    Gpr(_B)      BuiltIn & { bytes _B, reg 'general' }
    Xmm(_B)      BuiltIn & { bytes _B, reg 'vector'  }
    Mem(_B)      BuiltIn & { bytes _B, align 8, layout 'c' | 'packed' | 'auto' }

    Bool         Gpr(1)
    Byte         Gpr(1)  & Comparable(Byte)
    I8           Gpr(1)  & Comparable(I8)
    I16          Gpr(2)  & Comparable(I16)
    I32          Gpr(4)  & Comparable(I32)
    I64          Gpr(8)  & Comparable(I64)
    I128         Gpr(16) & Comparable(I128)
    U8           Gpr(1)  & Comparable(U8)
    U16          Gpr(2)  & Comparable(U16)
    U32          Gpr(4)  & Comparable(U32)
    U64          Gpr(8)  & Comparable(U64)
    U128         Gpr(16) & Comparable(U128)
    BF16         Xmm(2)  & Comparable(BF16)
    F16          Xmm(2)  & Comparable(F16)
    F32          Xmm(4)  & Comparable(F32)
    F64          Xmm(8)  & Comparable(F64)

    Int          I64
    Float        F64
    Ptr(_T)      Gpr(8)  & { target _T }
    Slice(_T)    Mem(16) & { ptr Ptr(_T), len Int }
    Simd(_T, _N) Xmm(_N) & { lane _T, lanes _N }
    Symbol       Gpr(8)                         # O(1) interned compile-time constant `'symbol'`
    String       Mem(16) & Comparable(String) & { ptr Ptr(Byte), byte_len Int } # UTF-8 text `"string"` (O(1) `.byte_len`, lazy `.chars`)

    Scalar       Bool | Byte | I8 | I16 | I32 | I64 | I128 | U8 | U16 | U32 | U64 | U128 | BF16 | F16 | F32 | F64 | Ptr(Any) | Symbol
    Primitive    Scalar | String | Slice(Any) | Simd(Any, Any)

    # Symmetric Bounded Interval (with Noether Conservation Invariant) & Tagged Domain Types:
    Range                    Form & { start (_T & Comparable(_T)), end (_T & Comparable(_T)) } & [ start <= end ]
    TaggedString(raw String) Form & { value raw }

    PortLiteral              TaggedString
    Url                      PortLiteral        # `Url"https://razomy.org"`
    Path                     PortLiteral        # `Path"./data/users.json"`
    Ip                       TaggedString       # `Ip"192.168.0.1:8080"`

    Temporal                 TaggedString
    Date                     Temporal           # `Date"2026-03-30"`
    Time                     Temporal           # `Time"14:30:00"`
    Datetime                 Temporal           # `Datetime"2026-03-30T14:30:00Z"`
    Duration                 Temporal           # `Duration"5m30s"`

    Hex                      TaggedString       # `Hex"#2A9D8F"`
    Pair                     TaggedString       # `Pair"1920x1080"`
    Version                  TaggedString       # `Version"1.4.2-beta"`
    Money                    Form & Comparable(Money) & { scaled Int 0 }
  }

  # ============================================================================
  # LAYER 3. ALGEBRAIC CONTAINERS, HKT, PERCEUS RC & METAPROGRAMMING (AXIOMS II–V)
  # ============================================================================
  Runtime {
    ..Primitives

    # 1st, 2nd & 3rd-Order Algebraic Containers:
    Some          Form & { value _T }
    Option(_T)    Some(_T) | None
    Map           Form & { (_K) _V }
    Set(_T)       Map(_T, Bool)
    Cons          Form & { head _T, tail List(_T) }
    List(_T)      None | Cons(_T)
    RoseTree      Form & { value _T, parent Weak(RoseTree(_T)), children Tuple(RoseTree(_T)) }
    Json          String | Int | Float | Bool | None | Tuple(Json) | Map(String, Json)

    Functor(_F(_))          Form & { fmap(fa _F(_A), f (_A) [_B]) _F(_B) }
    Monad(_M(_))            Functor(_M) & { pure(a _A) _M(_A), flat_map(ma _M(_A), f (_A) [_M(_B)]) _M(_B) }
    MonadTrans(_T(_(_), _)) Form & { lift(_M(_) & Monad, ma _M(_A)) _T(_M, _A) }

    # Subtyping-Driven Outcome Bypass (Axiom IV):
    Error          Form & Bypass
    Result(_T, _E) _T | (Error & _E)
    NotFound       Error & { entity String, id String }
    IoError        Error & { msg String }
    NetworkError   Error & { reason String }
    Unauthorized   Error & { reason String }
    CompileError   Error & { code Int, log String }
    InvariantFault Error & { reason String }
    OutOfMemory    Error

    # Perceus Reference Counting Duality (Unique O(1) In-Place Reuse <-> Shared Copy-on-Write <-> Weak Non-Owning):
    Unique         Form & { value _T }          # RC == 1: rebound in-place in O(1) via `.=`, `+=`, `-=`, `*=`, `/=`, `&=`
    Shared         Form & { value _T }          # RC > 1 : cloned via Copy-on-Write on rebind or pure `&` merge
    Weak           Form & { value _T }          # Non-owning pointer breaking RC cycles (`upgrade() Option(_T)`)

    # Concurrency, Attractor Loop & Homoiconic Dialects:
    Loop(body Stream) [ . body . .. ]           # Canonical attractor loop helper catching `Break & Bypass`
    Atom              Form & { value _T }       # Lock-free CAS cell (`read`, `swap`)
    Channel(_T)       Form                      # CSP message-passing channel (`send`, `recv`, `select`, dual `!Channel`)
    Parse             Form                      # Stream PEG grammar dialect (`match`, `copy`, `to`, `run`)
    Ui                Form                      # Declarative view & canvas dialect (`view`, `draw`)
    Meta              Form & Compile            # Homoiconic AST engine (`type_of`, `doc_of`, `span_of`, `fields_of`, `gensym`, `error`, `rewrite`)
  }
}
