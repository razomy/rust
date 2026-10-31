#
RAZOMY V6.4 — BUILT-IN STANDARD LIBRARY PRELUDE (src/prelude.rl)
Базовые контракты, алгебраические типы, Range, точный десятичный Money и тегированные типы.
#

Compile {}
Bypass  {}
Public  {}
Private {}

None       {} & Bypass
Some       { value _T }
Option(_T) Some(_T) | None
Break      { value _T, label Symbol 'default' } & Bypass
Return     { value _T } & Bypass
Loop(body Stream) [ body . native_loop ]

Range {
  start _T
  end   _T
  contains(val _T) [ (val >= start) && (val <= end) ]
}

Money(raw String "0.00") {
  scaled (raw . Decimal.parse_scale4)

  from_scaled(val Int) [ Money { scaled val } ]

  +   (other Money)   [ Money.from_scaled(scaled + other.scaled) ]
  -   (other Money)   [ Money.from_scaled(scaled - other.scaled) ]
  *   (factor Float)  [ Money.from_scaled((scaled.to_float * factor).round_to_int) ]
  /   (divisor Float) [ Money.from_scaled((scaled.to_float / divisor).round_to_int) ]
  ==  (other Money)   [ scaled == other.scaled ]
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