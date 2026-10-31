Decimal {
  parse_scale4(raw String) Int []
}
Byte {
  parse_hex(s String) Byte []
}
starts_with(s String, prefix String) Bool []
byte_slice(s String, start Int, end Int) String []
Regex {
  matches(input String, pattern String) Bool []
}
Json {
  format_value(v Any) String []
  encode(v Any) String []
  parse(s String) Any []
}
Http {
  get(url Any) String []
}
Port {
  read(src Any) Any []
  write(data Any, dst Any) Any []
}
Ui {
  view(spec Any) Any []
  draw(children Any) Any []
}
Parse {
  end Symbol 'eof'
  match(pat Any) Stream []
  one_or_more(r Any) Stream []
  repeat(b Any, r Any) Stream []
  skip(n Int) Stream []
  to(delim Any) Stream []
  copy(f Symbol, t Any) Stream []
  run(input String, rules Any) Any []
}
FieldMeta {
  name String
  type String
  value Any
  doc String
}
Meta {
  type_of(e Any) String [] & Compile
  name_of(e Any) String [] & Compile
  doc_of(e Any) String [] & Compile
  fields_of(e Any) Tuple(FieldMeta) [] & Compile
  steps_of(s Any) Tuple(Stream) [] & Compile
  map_fields(f Any, m Any) Form [] & Compile
  compose(s Any) Stream [] & Compile
  error(msg String) None [] & Compile
  identity(x Any) Any []
  rewrite(node Any, rules Any) Any [] & Compile
}
Atom(T) {
  value T
  read() T []
  get() T []
  set(val T) T []
  swap(tr Any) T []
}
Signal(T) {
  value T
  get() T []
  read() T []
  set(val T) T []
  swap(tr Any) T []
}
Channel(T) {
  send(val T) None []
  recv() Option(T) []
}
Map(K, V) {
  (K) V
  get(m Map(K, V), key K) Option(V) []
}
trim(s String) String []
to_upper(s String) String []
split(s String, delim String) Tuple(String) []
join(items Any, sep String) String []
length(item Any) Int []
append(items Any, elem Any) Tuple []
contains(items Any, target Any) Bool []
map(items Any, mapper Any) Tuple []
filter(items Any, predicate Any) Tuple []
fold(items Any, init Any, folder Any) Any []
flat_map(items Any, mapper Any) Tuple []
zip(items Any, other Any) Tuple []
enumerate(items Any) Tuple []
for_each(items Any, action Any) None []
find(items Any, predicate Any) Any []
take_while(items Any, predicate Any) Tuple []
Effect(stream Any) None []
timeout(val Any, dur Any) Any []
bit_and(a Int, b Int) Int []
bit_or(a Int, b Int) Int []
bit_xor(a Int, b Int) Int []