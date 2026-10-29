// =============================================================================
// RAZOMY V5.7 — BUILT-IN STANDARD LIBRARY PRELUDE (src/prelude.rl)
// Базовые контракты, алгебраические типы, точный десятичный Money и тегированные типы.
// =============================================================================

Compile {}
Bypass  {}
Public  {}
Private {}

None      {} & Bypass
Some(T)   { value T }
Option(T) Some(T) | None
Break(T)  { value T, label Symbol 'default' }

Money(raw String "0.00") {
  scaled (raw . Decimal.parse_scale4)

  from_scaled(val Int) [ Money { scaled val } ]

  +   (other Money)   [ Money.from_scaled(self.scaled + other.scaled) ]
  -   (other Money)   [ Money.from_scaled(self.scaled - other.scaled) ]
  *   (factor Float)  [ Money.from_scaled((self.scaled.to_float * factor).round_to_int) ]
  /   (divisor Float) [ Money.from_scaled((self.scaled.to_float / divisor).round_to_int) ]
  ==  (other Money)   [ self.scaled == other.scaled ]
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

Shell {
  log(msg String)  [ msg . native_print ]
  exec(cmd String) [ cmd . native_exec ]
}