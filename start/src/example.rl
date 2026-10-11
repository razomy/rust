# ==============================================================================
# ОБЩИЕ ПРАВИЛА И СИСТЕМНАЯ МОДЕЛЬ (GLOBAL RULES & CORE MODEL)
# ==============================================================================
# • Синтаксическая основа: лаконичный синтаксис на выражениях (Rust, Kotlin, F#).
# • Унификация функций и типов: функция — это синтаксический сахар над синглтон-
#   типом с оператором вызова `operator ()`. Любой тип может иметь состояние и
#   одновременно быть функцией (Callable Class/Object).
# • Литералы vs Блоки: { k: v } / { a, b } — запись; { a; b } — блок; { x, } — запись.
# • Обновление записей: `obj with { field: new_val }` (F#, OCaml, C#).
# • Логические операторы: строго словами `and`, `or`, `not` (Python).
# • Null-Safety & Ошибки: Option/Result, операторы '?', '?.', '??' (Rust, Kotlin, TS).
# • Конвейеры: конвейер '.>' с плейсхолдером '_', композиция '>>', геттер '.field'.
# • Система типов: номинативные типы + структурные объединения/пересечения (TS + Rust).
# ==============================================================================

# ==============================================================================
# 1. МОДУЛИ, ИМПОРТ И ВИДИМОСТЬ
# ==============================================================================

use console                                 # Простой импорт модуля — Based on: Rust, Python
use math = std.math                         # Импорт с локальным алиасом — Based on: Rust, Python
use { log, warn } = std.logger              # Селективный импорт символов — Based on: TS, Rust

pub use std.fs                              # Публичный реэкспорт всего модуля — Based on: Rust
pub use { info, error } = std.logger        # Публичный реэкспорт выбранных сущностей — Based on: TS, Rust

# ==============================================================================
# 2. ПЕРЕМЕННЫЕ, КОНСТАНТЫ И БАЗОВЫЕ ТИПЫ
# ==============================================================================

##
  Многострочный блочный комментарий
  для документации и заметок
##

pub const VERSION = "1.0.0"                 # Публичная константа уровня модуля
const DEFAULT_LIMIT Int = 10                # Константа с явным типом
const PI = 3.14159                          # Константа с выводом типа (Float)

let name = "Алиса"                          # Неизменяемая переменная (вывод Str)
let age = 25                                # Неизменяемое целое число (Int)
let city Str = "Киев"                       # Переменная с явным указанием типа
let enabled Bool = true                     # Булево значение true/false

let mut counter = 0                         # Изменяемая переменная (mut)
counter = counter + 1                       # Переприсваивание
counter += 1                                # Составное присваивание

# Условные конструкции (условия без скобок, операторы словами)
if enabled and not (counter > 10) {
    console.print("Система в норме")
} else if enabled or counter == 0 {
    console.print("Резервный режим")
}

# Числовые литералы и строки
let hex_val = 0xff                          # 16-ричный литерал
let bin_val = 0b1010                        # Двоичный литерал
let big_num = 1_000_000                     # Читаемые разделители разрядов
let raw_regex = r"\d+\s+\w+"                # Raw-строка без экранирования
let int_as_float = counter as Float         # Явное приведение примитивов

# ==============================================================================
# 3. ЕДИНАЯ СУЩНОСТЬ: ТИПЫ, КЛАССЫ, ФУНКЦИИ И КОНСТРУКТОРЫ
# ==============================================================================
# Концепция: тип может хранить поля, иметь конструктор и определяться как функция.
# Под капотом `fn foo(x)` — это синтаксический сахар над синглтон-структурой foo,
# реализующей `operator ()`.

# 3.1. Тип с конструктором и вызовом экземпляра (Callable Object / Класс-функция)
type Multiplier {
    pub factor Int                          # Состояние объекта

    # Первичный именованный конструктор с валидацией
    fn new(factor Int) Multiplier =>
        Multiplier { factor }

    # Фабрика/умный конструктор с защитой (Result)
    fn safe_new(factor Int) Result<Multiplier, Str> =>
        if factor <= 0 {
            Err("Множитель должен быть положительным")
        } else {
            Ok(Multiplier { factor })
        }

    # Оператор вызова: делает экземпляр функцией: `instance(x)`
    pub operator ()(self, x Int) Int =>
        self.factor * x
}

let tripler = Multiplier.new(3)
let calc_result = tripler(10)                # 30 (вызов объекта как обычной функции)

# 3.2. Тип, объединяющий логику валидатора и состояние
type Validator {
    min_len Int
    max_len Int

    # Конструктор со значениями по умолчанию
    fn new(min_len Int = 3, max_len Int = 20) Validator =>
        Validator { min_len, max_len }

    # Тело вызова валидатора
    pub operator ()(self, input Str) Bool => {
        let len = input.len()
        len >= self.min_len and len <= self.max_len
    }
}

let is_valid_name = Validator.new(3, 15)
let check = is_valid_name("Алиса")          # true

# 3.3. Традиционный синтаксис функций (Синтаксический сахар над синглтон-типом)
# Однострочники через '=>'
fn square(x Int) Int => x * x
fn add(a Int, b Int = 1) Int => a + b

# Многострочное блочное тело через '{ }'
fn clamp(val Int, min_val Int, max_val Int) Int {
    if val < min_val {
        return min_val                      # Ранний возврат через return
    }
    if val > max_val {
        return max_val
    }
    val                                     # Хвостовое выражение возвращается неявно
}

# Функция без возврата (None по умолчанию)
fn notify(msg Str) {
    console.print(msg)
}

# 3.4. Лямбды и замыкания
let double = (x Int) => x * 2               # Явная лямбда
let double_short = x => x * 2               # Короткая форма с выводом типа
let add_ten = add(10, _)                    # Частичное применение через плейсхолдер '_'

# ==============================================================================
# 4. СТРУКТУРЫ (RECORDS) И ИХ ОБНОВЛЕНИЕ
# ==============================================================================

pub type User {
    pub id Int
    pub name Str
    pub age Int
    password Str?                           # Опциональное приватное поле (Null-safety)
}

# Создание экземпляров
let alice = User { id: 1, name: "Алиса", age: 25, password: None }

# Field Punning (Shorthand-инициализация)
let id = 2
let name = "Боб"
let age = 30
let bob = User { id, name, age, password: None }

# Анонимные записи
let point = { x: 10, y: 20 }
let single_elem_rec = { id, }               # Запятая обязательна для записи из одного поля

# Деструктуризация полей
let { x, y } = point
let current_x = point.x

# Немутабельное обновление записи (Immutable Update)
let older_alice = alice with { age: 26 }

# ==============================================================================
# 5. ТИПЫ В ДУХЕ TYPESCRIPT: ОБЪЕДИНЕНИЯ, ПЕРЕСЕЧЕНИЯ, УТИЛИТЫ И МЕТАТИПЫ
# ==============================================================================

# 5.1. Примитивные псевдонимы (Type Aliases)
type UserId = Int
type Point2D = { x Int, y Int }

# 5.2. Объединения (Union Types '|')
type Status = "draft" | "published" | "archived"     # Литеральное объединение
type Primitive = Str | Int | Bool | Float             # Объединение типов

# Размеченное объединение (Discriminated Union в стиле TS/Flow)
type Shape =
    | { kind: "circle", radius Float }
    | { kind: "rect", width Float, height Float }
    | { kind: "square", size Float }

fn area(s Shape) Float => match s {
    { kind: "circle", radius } => 3.14159 * radius * radius,
    { kind: "rect", width, height } => width * height,
    { kind: "square", size } => size * size
}

# 5.3. Пересечения (Intersection Types '&')
type Identifiable = { id Str }
type Auditable = { created_at Int, updated_at Int }

# Составной тип, содержащий поля всех компонентов
type DatabaseUser = User & Identifiable & Auditable

# 5.4. Метаоператоры `keyof` и индексы по типу
type UserKeys = keyof User                           # "id" | "name" | "age" | "password"
type UserNameType = User["name"]                     # Str

# 5.5. Встроенные служебные типы (Utility Types)
type PartialUser = Partial<User>                     # Все поля становятся опциональными (T?)
type RequiredUser = Required<PartialUser>            # Все поля становятся обязательными
type PublicUser = Omit<User, "password">             # Исключение поля из типа
type CompactUser = Pick<User, "name" | "age">        # Выборка подмножества полей
type UsersById = Record<UserId, User>                # Ассоциативная карта (Map/Dict)
type ReadonlyPoint = Readonly<Point2D>               # Защита от мутаций

# ==============================================================================
# 6. АЛГЕБРАИЧЕСКИЕ ТИПЫ ДАННЫХ (ADTS / SUM TYPES)
# ==============================================================================

# Варианты без данных, с кортежами и со структурами
pub enum State<T> {
    Idle,
    Loading,
    Success(T),
    Failure { code Int, message Str }
}

enum Role {
    Admin,
    Member { permissions List<Str> },
    Guest
}

# ==============================================================================
# 7. ДЖЕНЕРИКИ И ТРЕЙТЫ (TRAITS & GENERICS)
# ==============================================================================

# Дженерик-функция с ограничением
fn print_id<T Identifiable>(item T) {
    console.print(item.id)
}

# Определение трейта
trait Printable {
    fn to_str(self) Str
}

trait Addable<Rhs = Self> {
    type Output
    fn add(self, rhs Rhs) Output
}

# Реализация методов для структуры
impl User {
    fn is_adult(self) Bool => self.age >= 18

    fn rename(self, new_name Str) User =>
        self with { name: new_name }
}

# Реализация трейта для User
impl Printable for User {
    fn to_str(self) Str => "${self.name} (${self.age})"
}

# Extension-методы: добавление функционала к внешним типам
impl Str {
    fn shouted(self) Str => "${self.upper()}!"
}

# ==============================================================================
# 8. ПЕРЕГРУЗКА ОПЕРАТОРОВ (OPERATOR OVERLOADING)
# ==============================================================================

type Vector2 {
    x Float
    y Float

    fn new(x Float, y Float) Vector2 => Vector2 { x, y }
}

impl Vector2 {
    # Перегрузка бинарного плюса '+'
    pub operator +(self, other Vector2) Vector2 =>
        Vector2 { x: self.x + other.x, y: self.y + other.y }

    # Перегрузка унарного минуса '-'
    pub operator -(self) Vector2 =>
        Vector2 { x: -self.x, y: -self.y }

    # Индексация массивом '[idx]'
    pub operator [](self, index Int) Float => match index {
        0 => self.x,
        1 => self.y,
        _ => panic("Индекс за пределами диапазона")
    }
}

let v1 = Vector2.new(1.0, 2.0)
let v2 = Vector2.new(3.0, 4.0)
let v3 = v1 + v2                            # Vector2 { x: 4.0, y: 6.0 }
let inv_v1 = -v1                            # Vector2 { x: -1.0, -2.0 }
let v1_x = v1[0]                            # 1.0

# ==============================================================================
# 9. NULL-SAFETY, OPTION И ОБРАБОТКА ОШИБОК
# ==============================================================================

# Option: тип T? эквивалентен Option<T>
fn find_user(id Int) User? =>
    if id == 1 {
        Some(alice)
    } else {
        None
    }

# Safe access '?.' и Coalescing '??'
let target = find_user(1)
let user_name = target?.name ?? "Гость"

# Ранний выход из Option через '?'
fn get_user_name_len(id Int) Int? {
    let user = find_user(id)?
    Some(user.name.len())
}

# Result: строгая типизация исключений и ошибок
fn parse_port(str_val Str) Result<Int, Str> =>
    if str_val.is_empty() {
        Err("Порт не указан")
    } else {
        Ok(Int.parse(str_val))
    }

# Преобразование Option в Result через ok_or и проброс через '?'
fn load_user_id(id Int) Result<Int, Str> {
    let user = find_user(id).ok_or("Пользователь не найден")?
    Ok(user.id)
}

# ==============================================================================
# 10. СОПОСТАВЛЕНИЕ С ОБРАЗЦОМ (PATTERN MATCHING) И УПРАВЛЕНИЕ ПОТОКОМ
# ==============================================================================

# 10.1. Тернарные и блочные if-выражения
let max_age = if alice.age > bob.age { alice.age } else { bob.age }

# Скобки `(...)` для структур в условиях исключают коллизию тела блока
if alice == (User { id: 1, name: "Алиса", age: 25, password: None }) {
    console.print("Данные профиля совпадают")
}

# 10.2. Match с guards, rest '..' и деструктуризацией
let state State<User> = Success(alice)

let description = match state {
    Idle => "Система спит",
    Loading => "Загрузка...",
    Success(User { name, age, .. }) if age >= 18 => "Взрослый: ${name}",
    Success(User { name, .. }) => "Пользователь: ${name}",
    Failure { code: 404, .. } => "Ресурс не найден",
    Failure { message, .. } => "Ошибка: ${message}"
}

# 10.3. Альтернативы '|' и wildcard '_'
let http_status = 404
let status_msg = match http_status {
    200 | 201 => "Успешно",
    400 | 404 => "Ошибка клиента",
    code if code >= 500 => "Ошибка сервера: ${code}",
    _ => "Неизвестный статус"
}

# ==============================================================================
# 11. КОЛЛЕКЦИИ, СРЕЗЫ И ЦИКЛЫ
# ==============================================================================

let numbers List<Int> = [1, 2, 3, 4, 5, 6]
let empty_list List<Str> = []

# Индексация и диапазоны срезов
let first = numbers[0]
let head_slice = numbers[..3]               # Срез [0, 3) -> [1, 2, 3]
let mid_slice = numbers[2..4]               # Срез [2, 4) -> [3, 4]
let tail_slice = numbers[3..]               # Срез от 3 до конца -> [4, 5, 6]
let full_copy = numbers[..]                 # Полная копия

# Паттерн-матчинг списков
let list_view = match numbers {
    [] => "пусто",
    [one] => "один элемент: ${one}",
    [head, ..tail] => "голова: ${head}, элементов в хвосте: ${tail.len()}"
}

# Итерация for-in
for item in numbers {
    console.print(item)
}

# Диапазонные циклы: полуоткрытый '..' и замкнутый '..='
for i in 0..5 {                             # 0, 1, 2, 3, 4
    console.print(i)
}

for i in 1..=5 {                            # 1, 2, 3, 4, 5
    console.print(i)
}

# Цикл while
let mut counter_loop = 3
while counter_loop > 0 {
    counter_loop -= 1
}

# ==============================================================================
# 12. ПАЙПЛАЙНЫ (PIPES), КОМПОЗИЦИЯ И ПОТОКОВАЯ ОБРАБОТКА
# ==============================================================================

fn trim(s Str) Str => s.trim()
fn to_upper(s Str) Str => s.upper()
fn wrap_tag(s Str, tag Str) Str => "<${tag}>${s}</${tag}>"

# Композиция функций вперед через '>>'
let format_header = trim >> to_upper

# Конвейерная обработка '.>' с плейсхолдером '_'
let raw_input = "   заголовок статьи   "
let rendered = raw_input
    .> format_header
    .> wrap_tag(_, "h1")                    # Подстановка в позицию '_'

# Fluent API с геттером свойств `.fieldName` и лямбдами
let users = [alice, bob]

let names_of_adults = users
    .filter(u => u.is_adult())
    .map(.name)                             # Синтаксис-сахар для `u => u.name`

# ==============================================================================
# 13. ТОЧКА ВХОДА И ПРИМЕР СИСТЕМНОГО СЦЕНАРИЯ
# ==============================================================================

pub fn main() {
    console.print("Запуск системы v${VERSION}")

    # Создание экземпляра вызываемого класса (Callable)
    let tax_calculator = Multiplier.new(2)
    let base_price = 100
    let total_price = tax_calculator(base_price)

    console.print("Итоговая цена: ${total_price}")

    # Использование расширения типа
    console.print("готово".shouted())       # ВЫВОД: "ГОТОВО!"
}

# ==============================================================================
# ДЖЕНЕРИКИ ВЫСШИХ ПОРЯДКОВ (HIGHER-KINDED TYPES — HKT): 2-Й И 3-Й ПОРЯДОК
# ==============================================================================
# Обозначение кайндов (Kinds / мета-типов типов):
# • Обычный тип (Kind *): Int, Str, User
# • Конструктор типа 1-го порядка (Kind * -> *): List<_>, Option<_>
# • Конструктор типа 2-го порядка (Kind (* -> *) -> *): Functor<F<_>>, Monad<M<_>>
# • Конструктор 3-го порядка (Kind ((* -> *) -> * -> *) -> *): MonadTrans<T<_, _>>
# ==============================================================================

# ------------------------------------------------------------------------------
# 1. ДЖЕНЕРИКИ 2-ГО ПОРЯДКА (HIGHER-KINDED TYPES: KIND (* -> *))
# Параметризация целым контейнером/эффектом F<_>, а не только типом значения T.
# ------------------------------------------------------------------------------

# 1.1. Классический Functor (Маппинг содержимого любого контейнера F)
type Functor<F<_>> {
    # Метод принимает F<A> и отображает в F<B> с помощью функции (A) => B
    fn map<A, B>(fa F<A>, f (A) => B) F<B>
}

# Реализация Functor для List<_>
impl Functor<List> {
    fn map<A, B>(fa List<A>, f (A) => B) List<B> => match fa {
        [] => [],
        [head, ..tail] => [f(head), ..Functor<List>.map(tail, f)]
    }
}

# Реализация Functor для Option<_>
impl Functor<Option> {
    fn map<A, B>(fa Option<A>, f (A) => B) Option<B> => match fa {
        Some(val) => Some(f(val)),
        None => None
    }
}

# 1.2. Монада: абстракция последовательных цепочек вычислений (FlatMap)
type Monad<M<_>> : Functor<M> {
    # Упаковка чистого значения в эффект M
    fn pure<A>(value A) M<A>

    # Связывание / FlatMap
    fn flat_map<A, B>(ma M<A>, f (A) => M<B>) M<B>

    # Дефолтная реализация map через flat_map и pure
    fn map<A, B>(ma M<A>, f (A) => B) M<B> =>
        self.flat_map(ma, a => self.pure(f(a)))
}

# 1.3. Полиморфная функция, абстрагированная от конкретного контейнера/эффекта
# Работает одинаково и для List, и для Option, и для Future!
fn increment_all<C<_> : Functor<C>>(container C<Int>) C<Int> =>
    Functor<C>.map(container, n => n + 1)

let list_res = increment_all([1, 2, 3])             # List: [2, 3, 4]
let opt_res = increment_all(Some(41))               # Option: Some(42)


# 1.4. Естественное преобразование (Natural Transformation: F ~> G)
# Морфизм между двумя контейнерами 1-го порядка без распаковки типов значений
type Transform<F<_>, G<_>> {
    # Конвертирует структуру F в структуру G для любого произвольного T
    fn apply<T>(source F<T>) G<T>
}

# Реализация преобразования Option<T> -> List<T>
type OptionToList : Transform<Option, List> {
    fn apply<T>(source Option<T>) List<T> => match source {
        Some(val) => [val],
        None => []
    }
}

let opt_to_list = OptionToList {}
let converted = opt_to_list.apply(Some(100))        # [100]


# ==============================================================================
# РЕКУРСИВНЫЕ ДЖЕНЕРИКИ, F-BOUNDED ТИПЫ И АЛГЕБРА ТИПОВ
# ==============================================================================

# 1. F-Bounded Polymorphism (Самоссылочные дженерики / Self-referential Bounds)
# Шаблон, где тип параметризуется самим собой (как Comparable<T> в Java/Rust)
type Comparable<SelfType : Comparable<SelfType>> {
    fn compare(self, other SelfType) Int

    fn is_greater(self, other SelfType) Bool =>
        self.compare(other) > 0
}

# Реализация: Money указывает самого себя в качестве типового аргумента
type Money : Comparable<Money> {
    pub amount Int
    pub currency Str

    fn compare(self, other Money) Int =>
        self.amount - other.amount
}

# Обобщенная функция поиска максимума над любым F-Bounded типом
fn max_item<T : Comparable<T>>(a T, b T) T =>
    if a.is_greater(b) { a } else { b }


# 2. Рекурсивные структуры данных (Деревья, Графы, Связные списки)
# Рекурсивный дженерик-список (Cons-ячейка / Nil)
enum LinkedList<T> {
    Nil,
    Cons(T, Box<LinkedList<T>>)               # Рекурсивная ссылка через Box/кучу
}

# Рекурсивное бинарное дерево поиска (BST)
type TreeNode<T : Comparable<T>> {
    pub value T
    pub left TreeNode<T>?                    # Рекурсивная ссылка через опционал
    pub right TreeNode<T>?

    fn new(value T) TreeNode<T> =>
        TreeNode { value, left: None, right: None }

    # Рекурсивный метод вставки
    fn insert(self, new_val T) TreeNode<T> => match self {
        node if new_val.compare(node.value) < 0 =>
            node with { left: Some(node.left?.insert(new_val) ?? TreeNode.new(new_val)) },
        node =>
            node with { right: Some(node.right?.insert(new_val) ?? TreeNode.new(new_val)) }
    }
}


# 3. Рекурсивное построение API: Fluent Builder / Method Chaining
# Дженерик 'SelfType' возвращает конкретный дочерний класс, сохраняя цепочку вызовов
type QueryBuilder<SelfType : QueryBuilder<SelfType>> {
    table Str
    filters List<Str>

    # Возврат конкретного финального типа через F-bound
    fn where_clause(self, condition Str) SelfType =>
        (self with { filters: self.filters.append(condition) }) as SelfType
}

type UserQueryBuilder : QueryBuilder<UserQueryBuilder> {
    include_deleted Bool

    fn with_deleted(self) UserQueryBuilder =>
        self with { include_deleted: true }
}

# Цепочка сохраняет доступ к специфичным методам UserQueryBuilder
let query = UserQueryBuilder { table: "users", filters: [], include_deleted: false }
    .where_clause("age >= 18")
    .with_deleted()


# 4. Рекурсивные TypeScript-style типы (DeepReadonly, JSON AST)
# Определение произвольного валидного JSON AST через рекурсивный Sum Type
type JsonValue =
    | None
    | Bool
    | Float
    | Str
    | List<JsonValue>                        # Рекурсивная ссылка на список значений
    | Record<Str, JsonValue>                 # Рекурсивная ссылка на словарь

# Рекурсивный мета-тип DeepReadonly (по аналогии с TypeScript conditional types)
type DeepReadonly<T> = match T {
    List<infer U> => ReadonlyList<DeepReadonly<U>>,
    Record<Str, infer U> => ReadonlyRecord<Str, DeepReadonly<U>>,
    infer Primitive => Primitive
}


# 5. Type-Level вычисления на рекурсивных типах (Peano Numbers / Числа Пеано)
# Вычисление размерностей на этапе компиляции (Haskell / Rust typenum)
type Zero
type Succ<N>                                 # Конструктор следующего числа N + 1

type One = Succ<Zero>
type Two = Succ<One>
type Three = Succ<Two>

# Рекурсивный безопасный вектор с фиксированной длиной на уровне типов
type FixedVector<T, N> {
    data List<T>

    # Добавление элемента увеличивает счетчик N -> Succ<N> на этапе проверки типов
    fn push(self, item T) FixedVector<T, Succ<N>> =>
        FixedVector { data: self.data.append(item) }
}

let empty_vec FixedVector<Int, Zero> = FixedVector { data: [] }
let vec_one = empty_vec.push(10)            # Тип: FixedVector<Int, One>
let vec_two = vec_one.push(20)              # Тип: FixedVector<Int, Two>

# ------------------------------------------------------------------------------
# 2. ДЖЕНЕРИКИ 3-ГО ПОРЯДКА (3RD-ORDER KINDS: ((* -> *) -> * -> *))
# Абстракция над типами, которые сами принимают конструкторы типов.
# Применяется в трансформаторах эффектов, свободных монадах и Tagless Final.
# ------------------------------------------------------------------------------

# 2.1. Монадный трансформатор (Monad Transformer)
# Trans<_<_, _>> — это конструктор 3-го порядка.
# Он принимает:
#   1) Базовую монаду M<_> (конструктор 1-го порядка)
#   2) Тип значения A
# И производит объединенный стек вычислений Trans<M, A>
type MonadTrans<Trans<M<_>, _>> {
    # Функция "lift": поднимает эффект из внутренней монады M в трансформер Trans<M>
    fn lift<M<_> : Monad<M>, A>(base_effect M<A>) Trans<M, A>
}

# Конкретный трансформер OptionT: добавляет возможность отсутствия значения (Option)
# поверх любой произвольной базовой монады M<_> (например, Task<Option<A>> или List<Option<A>>)
type OptionT<M<_>, A> {
    pub run M<Option<A>>                             # Вложенный стек эффектов

    fn new(run M<Option<A>>) OptionT<M, A> =>
        OptionT { run }

    # FlatMap через внутреннюю композицию
    fn flat_map<B>(self, f (A) => OptionT<M, B>, monad Monad<M>) OptionT<M, B> =>
        OptionT.new(
            monad.flat_map(self.run, opt_a => match opt_a {
                Some(a) => f(a).run,
                None => monad.pure(None)
            })
        )
}

# Реализация MonadTrans 3-го порядка для OptionT
impl MonadTrans<OptionT> {
    fn lift<M<_> : Monad<M>, A>(base_effect M<A>) OptionT<M, A> => {
        let monad = Monad<M>
        # Оборачиваем чистый результат M<A> в M<Option<A>> через Some
        let lifted_run = monad.map(base_effect, a => Some(a))
        OptionT.new(lifted_run)
    }
}


# 2.2. Мета-абстракция 3-го порядка над вычислительными движками (Tagless Final Algebra)
# Конструктор Interpreter<_<_>> принимает абстрактный вычислительный контекст F<_>
type LanguageAlgebra<F<_>> {
    fn number(val Int) F<Int>
    fn add(left F<Int>, right F<Int>) F<Int>
    fn text(val Str) F<Str>
}

# Мета-компилятор 3-го порядка: умеет транслировать любую программу из
# одного интерпретатора F<_> в другой G<_>, используя естественное преобразование T<F, G>
type PipelineCompiler<F<_>, G<_>, Trans : Transform<F, G>> {
    fn compile<A>(expr F<A>, transformer Trans) G<A> =>
        transformer.apply(expr)
}