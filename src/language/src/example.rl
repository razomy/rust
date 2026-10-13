// --- 1. Определение типов ---
Default {}
DefaultStr str & { Default "" }
SourceDir DefaultStr & { Default "." }

Text DefaultStr
Name DefaultStr
Message {}

TextMessage Message & { text Text }
ToolMessage Message & { name Name }

AnyMessage TextMessage | ToolMessage
AnyMessages Array[AnyMessage]

AiContext {
  SourceDir
  AnyMessages
}

// --- 2. Системные модули ---
Shell {
  Log { Process.Log(<T>) }
  Command { Process.Execute(<T>) }
}

// --- 3. Предикаты и паттерн-матчинг ---
IsError { <T> instanceof Error }

MatchYesNo {
  match <Query> {
    IsError(T <Query>) { <Yes> }
    Default { <No> }
  }
}