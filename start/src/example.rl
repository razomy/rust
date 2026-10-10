import console

# 1. Обобщённая сумма-тип (Generic Union)
type Response(T) = Success(data T) | Failure(error Str)

# 2. Обобщённая структура (Generic Record)
type Container(T) {
    value T,
    tag Str
}

type Role = Admin | User(id Int) | Guest
type Person { name Str, age Int, role Role }

# 3. Обобщённая функция упаковки значения
fn wrap(T)(val T, tag Str) Container(T) = {
    { value val, tag tag }
}

fn check_access(person Person) Response(Person) =
    match person.role {
        Admin = Success(person)
        User(id) if person.age >= 18 = Success(person)
        User(_) = Failure("Пользователь слишком молод")
        Guest = Failure("Гостям вход запрещен")
    }

out fn main() =
    let users = [
        { name "Алиса", age 25, role Admin },
        { name "Боб",  age 16, role User(101) },
        { name "Ева",  age 22, role Guest }
    ]

    # Обработка с использованием дженерик-типов и упаковки
    users
        .map((u) = check_access(u))
        .map((res) = match res {
            Success(person) = {
                let boxed = wrap(person.name, "verified_user")
                "✅ Доступ разрешен: ${boxed.value} [${boxed.tag}]"
            }
            Failure(err) = "❌ Отказ: ${err}"
        })
        .iter((log) = console.print(log))

    # Пример работы с отсутствующим значением (none)
    let config_path Str? = none

    let path = match config_path {
        Some(p) = p
        none = "/default/path.conf"
    }

    console.print("Используется путь: ${path}")