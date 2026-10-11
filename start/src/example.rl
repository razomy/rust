# ==============================================================================
# ОБЩИЕ ПРАВИЛА ЯЗЫКА (GLOBAL RULES)
# ==============================================================================
# • Синтаксическая основа: лаконичный синтаксис на базе выражений без ключевых слов end/точек с запятой (Rust, Kotlin, F#).
# • Литералы vs Блоки: { k: v } / { a, b } — запись; { a; b } — блок; { x, } — запись из одного поля; { x } — блок (JS, Rust, Python).
# • Обновление записей: obj with { field: new_val } без мутации оригинала (OCaml, F#, C#).
# • Функции: однострочники через '=' (Kotlin, Scala); многострочные через '{ }' (Rust, Swift); '=>' — только в типах, лямбдах и match.
# • Логические операторы: строго словами `and`, `or`, `not` (Python).
# • Null-safety & Ошибки: Option/Result, операторы '?', '?.', '??' (Rust, Kotlin, Swift, TypeScript).
# • Конвейеры: явный pipe '|>', композиция '>>', плейсхолдер '_' и member accessor '.field' (F#, Elixir, Scala, Swift).
# • Скобки в заголовках: литералы записей внутри if/while/for/match берутся в скобки: (Type { ... }) во избежание неоднозначности блока (Rust).
# ==============================================================================

# 1. Импорты и видимость
use console                                 # Простой импорт модуля — Based on: Rust, Python
use math = std.math                         # Импорт модуля с алиасом — Based on: Rust (as), Python
use { log, warn } = std.logger              # Деструктурирующий селективный импорт — Based on: JS/TypeScript, Rust

pub use std.fs                              # Реэкспорт всего модуля — Based on: Rust (pub use)
pub use { info, error } = std.logger        # Публичный реэкспорт конкретных символов — Based on: Rust, TypeScript

# 2. Константы, переменные, Bool, комментарии
##
  Многострочный
  комментарий
##                                          # Многострочный блочный комментарий — Based on: Julia (##), Dylan

pub const VERSION = "1.0.0"                 # Публичная константа уровня модуля — Based on: Rust, Zig
const DEFAULT_LIMIT Int = 10                # Типизированная константа — Based on: Go, Rust
const PI = 3.14159                          # Константа с выводом типа — Based on: Go, Rust, Swift

let name = "Алиса"                          # Неизменяемая переменная (вывод типа) — Based on: Rust, Swift
let age = 25                                # Целочисленная неизменяемая переменная — Based on: Rust, Kotlin
let city Str = "Киев"                       # Переменная с явным типом после имени — Based on: Go, Kotlin, Scala

let enabled Bool = true                     # Булев тип со значением true — Based on: Go, Rust
let disabled = false                        # Булев литерал со значением false — Based on: Python, Rust

# Условный оператор с логическим and и not — Based on: Python, Rust (условие без скобок)
if enabled and not disabled {
    console.print("Система активна")
}

# Условный оператор с логическим or — Based on: Python, Rust
if enabled or disabled {
    console.print("Хотя бы один флаг установлен")
}

# Объекты, деструктуризация и чтение полей
let ab = { a: 1, b: 2 }                     # Литерал анонимной записи (структуры) — Based on: TypeScript, OCaml
let { a, b } = ab                           # Деструктуризация полей записи в переменные — Based on: JS/TypeScript
let c = ab.a                                # Доступ к полю через точку — Based on: JS, Python, Rust

# Мутабельность (mut)
let mut counter = 0                         # Явное объявление изменяемой переменной — Based on: Rust
counter = counter + 1                       # Переприсваивание переменной — Based on: Rust, C
counter += 1                                # Составное присваивание с инкрементом — Based on: C, Python, Rust

# 3. Варианты объявления функций, return, Unit и лямбды
fn square1(x Int) = x * x                   # Однострочная функция с выводом типа возврата — Based on: Kotlin, Scala
fn square2(x Int, b Int) = x * b            # Однострочная функция с двумя аргументами — Based on: Kotlin
fn square3(x Int, b Int) Int = x * b        # Однострочник с явным возвращаемым типом — Based on: Scala, Kotlin

# Функция с блочным телом и неявным возвратом последнего выражения — Based on: Rust
fn square4(x Int, b Int) Int {
    x * b
}

# Функция с ветвлением и ранним выходом через return — Based on: Rust, Go
fn abs(x Int) Int {
    if x < 0 {
        return -x                           # Досрочный возврат значения — Based on: Rust, C
    }

    x                                       # Неявный возврат хвостового выражения — Based on: Rust, Ruby
}

fn add(a Int, b Int = 1) Int = a + b        # Параметры со значениями по умолчанию — Based on: Python, Kotlin, TypeScript

# Функция, возвращающая тип Unit (пустой результат) — Based on: Kotlin, Scala, Rust
fn log_msg(msg Str) Unit {
    console.print(msg)
}

# Функция без указания возвращаемого типа (Unit по умолчанию) — Based on: Kotlin, Go
fn log_msg_short(msg Str) {
    console.print(msg)
}

# Обобщенная (generic) функция — Based on: Rust, TypeScript
fn identity<T>(let T) T = let

# Обобщенная функция с ограничением типа (type constraint) — Based on: Rust, Go, C#
fn calc<T Number>(x T, b T) = x * b
fn calc2<T Number>(x T, b T) T = x * b      # Generic-функция с ограничением и явным возвратом — Based on: Rust

# Функция высшего порядка, принимающая функцию как аргумент — Based on: TypeScript, Rust
fn apply(x Int, f (Int) => Int) Int =
    f(x)

# Функция, возвращающая функциональный тип — Based on: Haskell, TypeScript, Kotlin
fn make_multiplier(k Int) (Int) => Int =
    x => x * k                              # Возврат лямбда-функции — Based on: JS/TypeScript, Python

# Лямбды
let double = (x) => x * 2                   # Лямбда с аргументом в скобках — Based on: TypeScript, Arrow function
let double_short = x => x * 2               # Короткая лямбда с одним параметром — Based on: ES6, C#
let applied = apply(10, double)             # Передача функции первого класса в аргумент — Based on: JS, Kotlin

# Вызов с аргументом по умолчанию: add(10) == 11
let eleven = add(10)                        # Вызов с подстановкой default value — Based on: Python, Kotlin

# Частичное применение — только явно через '_'
let add10 = add(10, _)                      # Частичное применение правого аргумента через '_' — Based on: Scala, Clojure
let fifteen = add10(5)                      # Вызов частично примененной функции — Based on: Scala, Haskell

let divide_by_2 = divide(_, 2)              # Частичное применение левого аргумента — Based on: Scala

# Блок как выражение
let block_letue = {                         # Выражение-блок, возвращающее результат — Based on: Rust
    let left = 10
    let right = 20
    left + right                            # Последняя строка вычисляется в значение переменной — Based on: Rust
}

type UserId = Int                           # Псевдоним примитивного типа (type alias) — Based on: TypeScript, Rust
type MaybeUser = User?                      # Опциональный тип через постфиксный '?' — Based on: Kotlin, TypeScript, Swift
type UserResult = Result<User, Str>         # Параметризованный Result-псевдоним — Based on: Rust
type PublicUser = Omit<User, "password">    # Utility type для исключения полей — Based on: TypeScript
type ReadonlyUser = Readonly<User>          # Utility type иммутабельности — Based on: TypeScript
type UserKeys = keyof User                  # Оператор извлечения типов ключей структуры — Based on: TypeScript
type NameOrAge = "name" | "age"             # Union строковых литеральных типов — Based on: TypeScript

# 4. Записи (Records), видимость полей и Алгебраические типы (ADTs)
# Объявление номинативного типа записи с модификаторами полей — Based on: Rust (struct), Go (struct)
pub type User {
    pub name Str                            # Публичное поле структуры — Based on: Rust
    pub age Int                             # Публичное поле целого типа — Based on: Rust
    password Str?                           # Приватное опциональное поле — Based on: Rust, TypeScript
}

# Алгебраический тип (ADT/Sum Type) со структурированными вариантами — Based on: Rust (enum), Swift
enum Role {
    Admin                                   # Вариант без полезной нагрузки — Based on: Rust, Swift
    Member { id Int }                       # Вариант с анонимной записью — Based on: Rust
    Guest                                   # Простой единичный вариант — Based on: Rust
}

# Обобщенное/публичное перечисление с payload-значениями — Based on: Rust, Swift
pub enum State {
    Idle                                    # Состояние простоя — Based on: Rust
    Loading                                 # Состояние загрузки — Based on: Rust
    Success(User)                           # Вариант с кортежным типом значения — Based on: Rust, Swift
    Error(Str)                              # Вариант с полезной строковой нагрузкой — Based on: Rust, Swift
}

# Создание экземпляра структуры с именованными полями — Based on: Rust, Zig
let alice User = User { name: "Алиса", age: 25, password: None }

# Shorthand-поля (запятая обязательна, если поле одно: { alice, })
let tag = "new"
let tagged_user = { alice, tag }            # Record field punning (shorthand) — Based on: JS/TypeScript, OCaml
let single_field = { tag, }                 # Shorthand-запись из одного поля с trailing comma — Based on: Python tuples: (x,)

# Immutable update записи
let older_alice = alice with { age: 26 }    # Иммутабельное обновление полей записи — Based on: OCaml, F#, C# (with)

# 5. Обобщенные структуры, объединения и generic defaults
# Параметризованный тип записи — Based on: Rust (struct<T>), Go
type Container<T> {
    letue T                                 # Поле обобщенного типа T — Based on: Rust
    tag Str                                 # Строковое поле метаданных — Based on: Rust
}

# Обобщенная пара из двух типов — Based on: Rust, C++, Haskell
type Pair<T, U> {
    first T
    second U
}

# Перечисление с generic constraint и типом по умолчанию — Based on: Rust, TypeScript
enum Response<T Number = Int> {
    Data(T)                                 # Значение обобщенного типа — Based on: Rust
    Empty                                   # Пустой вариант — Based on: Rust
}

# Конструктор контейнера с выводом типов полей — Based on: Rust, TypeScript
fn wrap<T>(let T, tag Str) Container<T> =
    Container { letue: let, tag }

# 6. Traits/typeclasses, impl и методы
# Определение интерфейса поведения (Trait/Typeclass) — Based on: Rust (trait), Haskell (typeclass)
trait Display {
    fn to_str(self) Str                     # Метод экземпляра с параметром self — Based on: Rust, Python
}

# Трейт клонирования с возвратом специального Self-типа — Based on: Rust (Clone), Swift
trait Clone {
    fn clone(self) Self                     # Возврат текущего типа реализации Self — Based on: Rust, Swift
}

# Трейт бинарных математических операций — Based on: Rust, Haskell
trait Number {
    fn add(self, other Self) Self
    fn mul(self, other Self) Self
}

# Блок реализации собственных методов для типа — Based on: Rust (impl)
impl User {
    fn is_adult(self) Bool =                # Метод-предикат над экземпляром — Based on: Rust, Kotlin
        self.age >= 18

    fn rename(self, name Str) User =        # Метод с возвратом обновленной копии — Based on: F#, Rust
        self with { name }

    fn without_password(self) User =        # Метод очистки приватного поля — Based on: F#, Rust
        self with { password: None }
}

# Реализация трейта Display для конкретного типа User — Based on: Rust (impl Trait for Type)
impl Display for User {
    fn to_str(self) Str =
        "${self.name}, ${self.age}"         # Строковая интерполяция свойств self — Based on: Kotlin, Swift, JS
}

# Функция с обобщенным ограничением типа через трейт — Based on: Rust (T: Display), C#
fn print_any<T Display>(letue T) {
    console.print(letue.to_str())           # Вызов метода из трейта Display — Based on: Rust
}

# Функция с множественным ограничением трейтов через '+' — Based on: Rust (Display + Clone)
fn debug_print<T Display + Clone>(letue T) {
    let copy = letue.clone()                # Вызов метода трейта Clone — Based on: Rust
    console.print(copy.to_str())
}

# 7. Отсутствие Null: None, Option ?, safe access, coalescing
# Функция, возвращающая опциональное значение (Option/Maybe) — Based on: Rust, Swift
fn find_user(id Int) User? =
    if id == 1 {
        Some(User { name: "Алиса", age: 25, password: None }) # Оборачивание в Some — Based on: Rust, Scala
    } else {
        None                                # Вариант отсутствия значения — Based on: Rust, Python
    }

let target = find_user(1)

# Сопоставление с образцом по Some/None — Based on: Rust (match Option)
let resolved_name = match target {
    Some(u) => u.name,                      # Распаковка значения из Some — Based on: Rust
    None => "Неизвестный"                   # Обработка отсутствия — Based on: Rust
}

# Безопасная навигация и оператор null-coalescing — Based on: Kotlin, Swift, TypeScript (?. и ??)
let safe_name = target?.name ?? "Неизвестный"

# Досрочный возврат через оператор '?' для Option — Based on: Rust (operator ?)
fn get_user_name(id Int) Str? {
    let user = find_user(id)?               # Возврат None, если результат равен None — Based on: Rust
    Some(user.name)                         # Возврат завернутого значения — Based on: Rust
}

# 8. Обработка ошибок: Result, Ok, Err и оператор ?
# Функция с результатом Result<T, E> — Based on: Rust (Result<T, E>)
fn divide(a Int, b Int) Result<Int, Str> =
    if b == 0 {
        Err("Деление на ноль")              # Оборачивание ошибки в Err — Based on: Rust
    } else {
        Ok(a / b)                           # Успешный результат в Ok — Based on: Rust
    }

# Обработка цепочек ошибок с преобразованием Option в Result — Based on: Rust
fn require_user(id Int) Result<User, Str> {
    let user = find_user(id).ok_or("Пользователь не найден")? # Преобразование и проброс ошибки — Based on: Rust
    Ok(user)
}

# Проброс ошибки через Result-оператор '?' — Based on: Rust (? operator)
fn load_name(id Int) Result<Str, Str> {
    let user = require_user(id)?            # Досрочный выход при Err — Based on: Rust
    Ok(user.name)
}

# 9. Управление потоком: if-выражение, match, guards, rest .., wildcard
let x = 10
let y = 20
let max = if x > y { x } else { y }          # Тернарный if-блок как выражение — Based on: Rust, Kotlin

# If-else блок с побочными эффектами и финальным выражением — Based on: Rust
let block_res = if x > 5 {
    console.print("Значение валидно")
    "Ok"
} else {
    "Fail"
}

# В заголовке if литерал записи берётся в скобки для устранения коллизии блока — Based on: Rust
if alice == (User { name: "Алиса", age: 25, password: None }) {
    console.print("Это Алиса")
}

let current_state = Success(User { name: "Боб", age: 30, password: None })

# Сопоставление структуры, вложенный паттерн-матчинг и guards — Based on: Rust, Swift
let state_desc = match current_state {
    Idle => "Ожидание...",
    Loading => "Загрузка...",
    Success(User { name, age }) if age > 18 => "Взрослый: ${name}", # Guard-условие 'if' — Based on: Rust, Swift
    Success(User { name, .. }) => "Пользователь: ${name}",         # Rest-паттерн '..' для игнора полей — Based on: Rust
    Error(err) => "Ошибка: ${err}",
    _ => "Неизвестное состояние"                                   # Wildcard-ветка по умолчанию — Based on: Rust
}

let code = 404

# Паттерн-матчинг примитивов с альтернативами '|' и условиями — Based on: Rust, Kotlin (when)
let code_desc = match code {
    200 => "ok",
    400 | 404 => "client error",             # Объединение условий через '|' — Based on: Rust, Ruby
    n if n >= 500 => "server error",         # Привязка переменной с guard — Based on: Rust, Scala
    _ => "unknown"                           # Ветка по умолчанию — Based on: Rust
}

# Деструктуризация записи в ветках match — Based on: Rust
let matched_user = match alice {
    User { name: "Алиса", age, .. } => "Alice age ${age}", # Матчинг литерала поля и пропуск остальных — Based on: Rust
    User { name, .. } => name
}

# Ветка сопоставления с телом-блоком из нескольких операторов — Based on: Rust
let verbose_desc = match code {
    404 => {
        console.print("Не найдено")
        "not found"                         # Возвращаемое значение ветки — Based on: Rust
    },
    _ => "other"
}

# 10. Списки, индексация, срезы и диапазоны
let names List<Str> = ["Алиса", "Боб", "Ева"]        # Литерал типизированного динамического списка — Based on: Python, Rust
let numbers List<Int> = [1, 2, 3, 4, 5, 6]           # Список целых чисел — Based on: Python, Kotlin
let empty_numbers List<Int> = []                     # Пустой типизированный список — Based on: Rust, TypeScript

let first_name = names[0]                            # Доступ к элементу списка по индексу — Based on: Python, C, Rust
let names_middle = names[1..3]                       # Срез полуоткрытого диапазона [1, 3) — Based on: Rust, Python (1:3)
let names_from_start = names[..2]                    # Срез от начала коллекции — Based on: Rust, Python (:2)
let names_to_end = names[1..]                        # Срез до конца коллекции — Based on: Rust, Python (1:)
let names_copy = names[..]                           # Полный срез (копия) коллекции — Based on: Rust, Python (:)

# Цикл по полуоткрытому диапазону '..' (без правой границы) — Based on: Rust, Swift (1..<5)
for i in 1..5 {
    console.print(i)
}

# Цикл по замкнутому диапазону '..=' (включая границу) — Based on: Rust (..=), Swift (...)
for i in 1..=5 {
    console.print(i)
}

# Итерация for-in по элементам коллекции — Based on: Rust, Python, Swift
for n in names {
    console.print(n)
}

let mut count = 3

# Цикл while с булевым условием — Based on: Rust, Python, C
while count > 0 {
    console.print(count)
    count = count - 1
}

# Паттерн-матчинг списков с rest-паттерном '..' — Based on: Haskell, Rust, Elixir
let list_desc = match numbers {
    [] => "empty",                                   # Матчинг пустого списка — Based on: Haskell, Rust
    [one] => "one item",                             # Матчинг ровно одного элемента — Based on: Haskell, Rust
    [first, ..] => "many, first is ${first}"         # Голова списка + хвост (head/tail rest) — Based on: Rust, Elixir
}

# 11. Числа, строки, escaping и приведение типов
let int_num Int = 42                                 # Десятичное целое — Based on: C, Rust
let float_num Float = 3.14                           # Число с плавающей точкой — Based on: C, Rust
let hex_num = 0xff                                   # Шестнадцатеричный литерал — Based on: C, Rust, JS
let bin_num = 0b1010                                 # Двоичный числовой литерал — Based on: Rust, Python
let big_num = 1_000_000                              # Числовой литерал с разделителями разрядов — Based on: Rust, Ruby, Java
let negative_num = -10                               # Отрицательное число — Based on: C, Rust

let escaped = "Строка с переносом\nи кавычкой \""    # Строка с escape-последовательностями — Based on: C, JS, Rust
let raw_regex = r"\d+\s+\w+"                         # Raw-строка без экранирования — Based on: Rust, Python (r"...")

let int_as_float = int_num as Float                  # Явное приведение примитивного типа через 'as' — Based on: Rust
let parsed_int = Int.parse("123")                    # Статический метод парсинга типа — Based on: C#, Rust

# 12. Конвейерная обработка: методы, pipe, композиция, field accessor
fn trim(letue Str) Str = letue.trim()
fn lower(letue Str) Str = letue.lower()
fn add_prefix(letue Str, prefix Str) Str =
    "${prefix}${letue}"

# Композиция функций вперед через оператор '>>' — Based on: F#, Haskell, Elm
let normalize = trim >> lower

# Конвейерная обработка через '|>' с плейсхолдером '_' — Based on: Elixir, F#, Hack
let normalized_name = " Алиса "
    |> normalize                                     # Передача левого значения первым аргументом — Based on: Elixir, F#
    |> add_prefix(_, "user:")                        # Подстановка значения в указанный слот '_' — Based on: Scala, Clojure

# Цепочка вызовов методов через точку (fluent API) — Based on: Kotlin, Java, Rust
let sum_even = numbers
    .filter((n) => n % 2 == 0)
    .map(n => n * 10)
    .sum()

# Вызовы методов и шортхонд геттера поля '.fieldName' — Based on: Swift (KeyPath \.name), Kotlin
let adult_names = [alice, older_alice]
    .filter(u => u.is_adult())
    .map(.name)                                      # Синтаксический сахар для (x => x.name) — Based on: Swift, Clojure

# 13. Деструктуризация в параметрах и лямбдах
# Деструктуризация структуры прямо в сигнатуре функции — Based on: Rust, Clojure, TypeScript
fn greet(User { name, age, .. }) Str =
    "${name}: ${age}"

let greeted = greet(alice)

# Деструктуризация аргумента внутри лямбды (в скобках) — Based on: Rust, JS/TypeScript
let only_names = [alice, older_alice]
    .map((User { name, .. }) => name)

# 14. Комплексная логика и точка входа
# Сопоставление перечисления с guard-условием и генерацией Result — Based on: Rust
fn check_access(user User, role Role) Result<Container<User>, Str> =
    match role {
        Admin => Ok(wrap(user.without_password(), "admin")),
        Member { .. } if user.age >= 18 => Ok(wrap(user.without_password(), "verified_user")), # Pattern + Guard — Based on: Rust
        Member { .. } => Err("Пользователь слишком молод"),
        Guest => Err("Гостям вход запрещен")
    }

# Публичная точка входа программы main — Based on: Rust, C, Kotlin
pub fn main() {
    console.print("Версия: ${VERSION}")

    # Создание списка структур — Based on: Rust, Kotlin
    let users = [
        User { name: "Алиса", age: 25, password: None },
        User { name: "Боб", age: 16, password: None },
        User { name: "Ева", age: 22, password: None }
    ]

    # Обработка потока данных через функции высшего порядка — Based on: Kotlin, Rust, JS
    users
        .map((u) => check_access(u, Member { id: 1 }))
        .map((res) => match res {                    # Вложенный match внутри итератора — Based on: Rust
            Ok(container) => "✅ Доступ: ${container.letue.name} [${container.tag}]",
            Err(err) => "❌ Ошибка: ${err}"
        })
        .for_each((msg) => console.print(msg))       # Терминальная операция итератора с side-effect — Based on: Kotlin, Java

    let config_path Str? = None

    # Распаковка Option через match с дефолтным значением — Based on: Rust
    let path = match config_path {
        Some(p) => p,
        None => "/default/path.conf"
    }

    console.print("Используется путь: ${path}")

    let maybe_admin = find_user(1)

    # Безопасное чтение поля и coalescing к fallback-строке — Based on: TypeScript, Kotlin, Swift
    let admin_name = maybe_admin?.name ?? "anonymous"

    console.print("Админ: ${admin_name}")
}