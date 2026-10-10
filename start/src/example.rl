# 1. Импорты
use console
use math = std.math
use { log, warn } = std.logger

# 2. Переменные, доступ к полям и деструктуризация
##
  Многострочный
  комментарий
##
var name = "Алиса"
var age = 25
var city Str = "Киев"

# Объекты, деструктуризация и чтение полей
var ab = { a 1, b 2 }
var { a, b } = ab
var c = ab.a

# Мутабельность (mut)
var mut counter = 0
counter = counter + 1

# 3. Варианты объявления функций и лямбд
fn square1(x Int) -> x * x
fn square2(x Int, b Int) -> x * x
fn square3(x Int, b Int) Int -> x * x

# Если функция не однострочник через '->', тело обязательно оформляется блоком '{ }'
fn square4(x Int, b Int) Int {
    x * x
}

fn add(a Int, b Int = 1) Int -> a + b

fn log_msg(msg Str) {
    console.print(msg)
}

# Дженерики в функциях задаются через <>
fn identity<T>(val T) T -> val

fn calc<T Number>(x T, b T) -> x * b
fn calc2<T Number>(x T, b T) T -> x * b

# Лямбды
var double = (x) -> x * 2
var double_short = x -> x * 2

type UserId = Int
type MaybeUser = User?
type UserResult = Result<User, Str>
type PublicUser = Omit<User, "password">
type ReadonlyUser = Readonly<User>
type UserKeys = keyof User
type NameOrAge = "name" | "age"

# 4. Записи (Records) и Алгебраические типы (ADTs)
type User {
    name Str
    age Int
}

enum Role {
    Admin
    Member { id Int }
    Guest
}

enum State {
    Idle
    Loading
    Success(User)
    Error(Str)
}

var alice User = User { name "Алиса", age 25 }

# 5. Обобщенные структуры и объединения
type Container<T> {
    value T
    tag Str
}

enum Response<T Number = Int> {
    Data(T)
    Empty
}

fn wrap<T>(val T, tag Str) Container<T> ->
    Container { value val, tag }

# 6. Отсутствие Null (None и Option ?)
fn find_user(id Int) User? ->
    if id == 1 {
        Some(User { name "Алиса", age 25 })
    } else {
        None
    }

var target = find_user(1)

var resolved_name = match target {
    Some(u) u.name,
    None "Неизвестный"
}

# 7. Обработка ошибок (Result, Ok, Err)
fn divide(a Int, b Int) Result<Int, Str> ->
    if b == 0 {
        Err("Деление на ноль")
    } else {
        Ok(a / b)
    }

# 8. Управление потоком (if-выражение, match, guards, rest ..)
var x = 10
var y = 20
var max = if x > y { x } else { y }

var block_res = if x > 5 {
    console.print("Значение валидно")
    "Ok"
} else {
    "Fail"
}

var current_state = Success(User { name "Боб", age 30 })

var state_desc = match current_state {
    Idle "Ожидание...",
    Loading "Загрузка...",
    Success(User { name, age }) if age > 18 { "Взрослый: ${name}" },
    Success(User { name, .. }) "Пользователь: ${name}",
    Error(err) "Ошибка: ${err}"
}

# 9. Циклы (for in, диапазон, while) — строго требуют блок { }
var names = ["Алиса", "Боб", "Ева"]

for n in names {
    console.print(n)
}

for i in 1..5 {
    console.print(i)
}

var mut count = 3

while count > 0 {
    console.print(count)
    count = count - 1
}

# 10. Конвейерная обработка (Пайплайны)
var numbers = [1, 2, 3, 4, 5, 6]

var sum_even = numbers
    .filter((n) -> n % 2 == 0)
    .map(n -> n * 10)
    .sum()

# 11. Комплексная логика и точка входа
fn check_access(user User, role Role) Result<Container<User>, Str> ->
    match role {
        Admin Ok(wrap(user, "admin")),
        Member { .. } if user.age >= 18 { Ok(wrap(user, "verified_user")) },
        Member { .. } Err("Пользователь слишком молод"),
        Guest Err("Гостям вход запрещен")
    }

pub fn main() {
    var users = [
        User { name "Алиса", age 25 },
        User { name "Боб", age 16 },
        User { name "Ева", age 22 }
    ]

    users
        .map((u) -> check_access(u, Admin))
        .map((res) -> match res {
            Ok(container) "✅ Доступ: ${container.value.name} [${container.tag}]",
            Err(err) "❌ Ошибка: ${err}"
        })
        .iter((msg) -> console.print(msg))

    var config_path Str? = None

    var path = match config_path {
        Some(p) p,
        None "/default/path.conf"
    }

    console.print("Используется путь: ${path}")
}