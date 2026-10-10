# 1. Импорты
im console
im math = "std/math"
im { log, warn } = "std/logger"

# 2. Переменные, доступ к полям и деструктуризация
##
  Многострочный
  комментарий
##
vr name = "Алиса"
vr age = 25
vr city Str = "Киев"

# Объекты, деструктуризация и чтение полей
vr ab = { a 1, b 2 }
vr { a, b } = ab
vr c = ab.a

# Мутабельность (mut)
vr mut counter = 0
counter = counter + 1

# 3. Варианты объявления функций и лямбд
fn square(x Int) = x * x
fn square(x Int, b Int) = x * x
fn square(x Int, b Int) Int = x * x
fn square(x Int, b Int) x * x
fn square(x Int, b Int)
    x * x

fn add(a Int, b Int = 1) Int =
    a + b

fn log_msg(msg Str)
    console.print(msg)

# Дженерики в функциях: над-типы и ограничения задаются через <>
fn identity<T>(val T) T =
    val

fn calc<T Number>(x T, b T) = x * x
fn calc<T Number>(x T, b T) T = x * x

# Лямбды
vr double = (x) = x * 2
vr double_short = x = x * 2

# 4. Записи (Records) и Алгебраические типы (ADTs)
type User { name Str, age Int }
vr alice User = { name "Алиса", age 25 }

type Role =
    | Admin
    | User(id Int)
    | Guest

type State =
    | Idle
    | Loading
    | Success(User)
    | Error(Str)

# 5. Обобщенные структуры и объединения
# Базовый параметр типа задается в () как в пространстве типов
type Container(T) {
    value T,
    tag Str
}

# Если нужен над-тип (мета-дженерик/ограничение), он пишется в <>, а параметры типа — в ()
type Response<TT Real = Number>(T TT = Int) =
    | Data(T)
    | Empty

fn wrap<T>(val T, tag Str) Container(T) =
    { value val, tag tag }

# 6. Отсутствие Null (None и Option ?)
fn find_user(id Int) User? =
    if id == 1
        Some({ name "Алиса", age 25 })
    else
        none

vr target = find_user(1)
vr resolved_name = match target {
    Some(u) = u.name
    none = "Неизвестный"
}

# 7. Обработка ошибок (Result, Ok, Err)
fn divide(a Int, b Int) Result(Int, Str) =
    if b == 0
        Err("Деление на ноль")
    else
        Ok(a / b)

# 8. Управление потоком (if-выражение, match, guards, rest ..)
vr x = 10
vr y = 20
vr max = if x > y x else y

vr block_res = if x > 5 {
    console.print("Значение валидно")
    "Ok"
} else {
    "Fail"
}

vr current_state = Success({ name "Боб", age 30 })

vr state_desc = match current_state {
    Idle = "Ожидание..."
    Loading = "Загрузка..."
    Success({ name, age }) if age > 18 = "Взрослый: ${name}"
    Success({ name, .. }) = "Пользователь: ${name}"
    Error(err) = "Ошибка: ${err}"
}

# 9. Циклы (for in, диапазон, while)
vr names = ["Алиса", "Боб", "Ева"]

for n in names
    console.print(n)

for i in 1..5 {
    console.print(i)
}

vr mut count = 3
while count > 0 {
    console.print(count)
    count = count - 1
}

# 10. Конвейерная обработка (Пайплайны)
vr numbers = [1, 2, 3, 4, 5, 6]

vr sum_even = numbers
    .filter((n) = n % 2 == 0)
    .map(n = n * 10)
    .sum()

# 11. Комплексная логика и точка входа
fn check_access(user User, role Role) Result(Container(User), Str) =
    match role {
        Admin = Ok(wrap(user, "admin"))
        User(id) if user.age >= 18 = Ok(wrap(user, "verified_user"))
        User(_) = Err("Пользователь слишком молод")
        Guest = Err("Гостям вход запрещен")
    }

out fn main()
    vr users = [
        { name "Алиса", age 25 },
        { name "Боб",  age 16 },
        { name "Ева",  age 22 }
    ]

    users
        .map((u) = check_access(u, Admin))
        .map((res) = match res {
            Ok(box)  = "✅ Доступ: ${box.value.name} [${box.tag}]"
            Err(err) = "❌ Ошибка: ${err}"
        })
        .iter((msg) = console.print(msg))

    vr config_path Str? = none
    vr path = match config_path {
        Some(p) = p
        none = "/default/path.conf"
    }

    console.print("Используется путь: ${path}")