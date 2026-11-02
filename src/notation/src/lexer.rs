//! Phase 1 Topological Pre-Lexer for Razomy Notation (`.rn`) v12.0.
//!
//! Implements Law 2.3 (Two-Phase Topological Activation), Law 3.1 (Indent Topology),
//! Law 5.1 (Raw Blocks `"lang` and Ranked Fences `rN"..."N`), and Law 5.2 (Homoiconic `#` Comments).

use serde::{Deserialize, Serialize};

/// Active Triadic Context Mode (Axiom I).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextMode {
    /// `[` Stream Context — Ordered Prose & Markup (Default Root Context)
    Stream,
    /// `{` Form Context — Unique Key-Value Map, Schema & Style
    Form,
    /// `(` Tuple Context — Positional Array & 2D Relational Grid
    Tuple,
    /// `"` Raw Context — Unescaped Verbatim Canvas
    Raw,
}

/// Fused indentation transition token emitted by the Phase 1 Pre-Lexer (Law 2.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndentActivation {
    /// Trailing `[` fused with `Indent + 1`
    IndentStream,
    /// Trailing `{` fused with `Indent + 1`
    IndentForm,
    /// Trailing `(` fused with `Indent + 1`
    IndentTuple,
    /// Trailing `"lang` or `"` fused with `Indent + 1` (carries optional language tag)
    IndentRaw(String),
    /// Bare `Indent + 1` inheriting parent's active mode (Law 2.1)
    IndentInherit,
    /// Same or less indent (leaf line)
    Leaf,
}

/// Diagnostic warning or error produced during lexing/parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub end_col: usize,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

/// A pre-tokenized logical line or block from Phase 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexedLine {
    /// 0-indexed physical line number in source
    pub line_idx: usize,
    /// Number of leading spaces
    pub indent: usize,
    /// Trimmed line content (with activated trailing bracket/raw marker stripped if fused)
    pub head_text: String,
    /// Original raw trimmed text before stripping trailing bracket
    pub original_trimmed: String,
    /// Fused topological activation with the next line
    pub activation: IndentActivation,
    /// If this line opens a Raw Block (`"lang` or `rN"..."N`), holds the captured verbatim content
    pub raw_payload: Option<RawPayload>,
    /// Whether this line is a comment (`# ...`)
    pub is_comment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPayload {
    pub lang: String,
    pub content: String,
    pub rank: Option<usize>,
}

/// Output of the Phase 1 Topological Pre-Lexer.
#[derive(Debug, Clone, Default)]
pub struct PreLexResult {
    pub lines: Vec<LexedLine>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Runs the O(N) Two-Phase Topological Pre-Lexer on `.rn` source text.
pub fn pre_lex(source: &str) -> PreLexResult {
    let raw_lines: Vec<&str> = source.lines().collect();
    let mut lines = Vec::new();
    let mut diagnostics = Vec::new();

    let total = raw_lines.len();
    let mut idx = 0;

    while idx < total {
        let line_str = raw_lines[idx];
        let indent = count_leading_spaces(line_str);
        let trimmed = line_str.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }

        // Law 5.2: Collision-Free Comments `# ` vs Literal `#token`
        // `#` followed by space (`# comment`) or bare `#` at end of line (multiline comment fence)
        if trimmed == "#" {
            // Multiline comment block until matching `#` at same indent
            let start_line = idx;
            let mut comment_buf = String::new();
            idx += 1;
            let mut closed = false;
            while idx < total {
                let next_str = raw_lines[idx];
                let next_indent = count_leading_spaces(next_str);
                let next_trim = next_str.trim();
                if next_indent == indent && next_trim == "#" {
                    closed = true;
                    idx += 1;
                    break;
                }
                if !comment_buf.is_empty() {
                    comment_buf.push('\n');
                }
                comment_buf.push_str(next_str);
                idx += 1;
            }
            if !closed {
                diagnostics.push(Diagnostic {
                    line: start_line,
                    col: indent,
                    end_col: indent + 1,
                    severity: DiagnosticSeverity::Warning,
                    message: "Unclosed multiline `#` comment block".to_string(),
                    code: String::from("RN001"),
                });
            }
            lines.push(LexedLine {
                line_idx: start_line,
                indent,
                head_text: format!("# {}", comment_buf),
                original_trimmed: "#".to_string(),
                activation: IndentActivation::Leaf,
                raw_payload: None,
                is_comment: true,
            });
            continue;
        }

        if trimmed.starts_with("# ") || trimmed.starts_with("#\t") {
            lines.push(LexedLine {
                line_idx: idx,
                indent,
                head_text: trimmed.to_string(),
                original_trimmed: trimmed.to_string(),
                activation: IndentActivation::Leaf,
                raw_payload: None,
                is_comment: true,
            });
            idx += 1;
            continue;
        }

        // Axiom IV: Column-0 Multiline Raw Strings (`(tag)?(N)?"` at end of line, closed by `"(N)?` at Column 0)
        if let Some((head_prefix, lang_tag, rank_opt)) = detect_col0_raw_open(trimmed) {
            let start_line = idx;
            let close_marker = match rank_opt {
                Some(r) => format!("\"{}", r),
                None => "\"".to_string(),
            };
            let mut content_lines = Vec::new();
            idx += 1;
            let mut closed = false;
            while idx < total {
                let cur = raw_lines[idx];
                // Closing `"(N)?` placed strictly alone on its own line at Column 0
                if cur.trim_end() == close_marker {
                    closed = true;
                    idx += 1;
                    break;
                }
                content_lines.push(cur.to_string());
                idx += 1;
            }

            if !closed {
                diagnostics.push(Diagnostic {
                    line: start_line,
                    col: indent,
                    end_col: line_str.len(),
                    severity: DiagnosticSeverity::Error,
                    message: format!(
                        "Unclosed Column-0 multiline raw string; expected `{}` at Column 0",
                        close_marker
                    ),
                    code: "RN002".to_string(),
                });
            }

            lines.push(LexedLine {
                line_idx: start_line,
                indent,
                head_text: head_prefix,
                original_trimmed: trimmed.to_string(),
                activation: IndentActivation::Leaf,
                raw_payload: Some(RawPayload {
                    lang: lang_tag,
                    content: content_lines.join("\n"),
                    rank: rank_opt,
                }),
                is_comment: false,
            });
            continue;
        }

        // Look ahead to the next non-empty, non-comment line to check topological activation (Law 2.3)
        let next_indent = peek_next_non_empty_indent(&raw_lines, idx + 1);
        let is_indented_child = match next_indent {
            Some(ni) => ni > indent,
            None => false,
        };

        // Check if line ends with an unclosed raw quote `"lang` or `"`
        if let Some((head_without_raw, lang)) = detect_trailing_raw_open(trimmed) {
            if is_indented_child {
                let start_line = idx;
                idx += 1;
                let mut raw_block_lines: Vec<(usize, &str)> = Vec::new();
                while idx < total {
                    let sub = raw_lines[idx];
                    if sub.trim().is_empty() {
                        raw_block_lines.push((usize::MAX, ""));
                        idx += 1;
                        continue;
                    }
                    let sub_indent = count_leading_spaces(sub);
                    if sub_indent <= indent {
                        break;
                    }
                    raw_block_lines.push((sub_indent, sub));
                    idx += 1;
                }
                // Strip trailing empty lines
                while raw_block_lines.last().map(|(_, s)| s.is_empty()).unwrap_or(false) {
                    raw_block_lines.pop();
                }
                let min_indent = raw_block_lines
                    .iter()
                    .filter(|(i, _)| *i != usize::MAX)
                    .map(|(i, _)| *i)
                    .min()
                    .unwrap_or(indent + 1);

                let stripped: Vec<String> = raw_block_lines
                    .into_iter()
                    .map(|(i, s)| {
                        if i == usize::MAX || s.len() < min_indent {
                            String::new()
                        } else {
                            s[min_indent..].to_string()
                        }
                    })
                    .collect();

                lines.push(LexedLine {
                    line_idx: start_line,
                    indent,
                    head_text: head_without_raw.trim().to_string(),
                    original_trimmed: trimmed.to_string(),
                    activation: IndentActivation::IndentRaw(lang.clone()),
                    raw_payload: Some(RawPayload {
                        lang,
                        content: stripped.join("\n"),
                        rank: None,
                    }),
                    is_comment: false,
                });
                continue;
            }
        }

        // Law 2.3: Check trailing `[`, `{`, `(` activation with following `Indent + 1`
        let trailing_bracket = detect_trailing_open_bracket(trimmed);

        let (head_text, activation) = match (trailing_bracket, is_indented_child) {
            (Some((stripped_head, '[')), true) => (
                stripped_head.trim_end().to_string(),
                IndentActivation::IndentStream,
            ),
            (Some((stripped_head, '{')), true) => (
                stripped_head.trim_end().to_string(),
                IndentActivation::IndentForm,
            ),
            (Some((stripped_head, '(')), true) => (
                stripped_head.trim_end().to_string(),
                IndentActivation::IndentTuple,
            ),
            (Some((_stripped_head, '{')), false) => {
                // Law 2.3 Safety Diagnostic in `{`: dangling trailing `{` not followed by Indent + 1
                diagnostics.push(Diagnostic {
                    line: idx,
                    col: line_str.len().saturating_sub(1),
                    end_col: line_str.len(),
                    severity: DiagnosticSeverity::Warning,
                    message: "Dangling trailing `{` not followed by indented block; use `{}` for an empty Form or quote `\"{\"` for a literal brace (Law 2.3)".to_string(),
                    code: "RN003".to_string(),
                });
                (trimmed.to_string(), IndentActivation::Leaf)
            }
            (None, true) | (Some(_), true) => {
                (trimmed.to_string(), IndentActivation::IndentInherit)
            }
            (_, false) => (trimmed.to_string(), IndentActivation::Leaf),
        };

        lines.push(LexedLine {
            line_idx: idx,
            indent,
            head_text,
            original_trimmed: trimmed.to_string(),
            activation,
            raw_payload: None,
            is_comment: false,
        });

        idx += 1;
    }

    PreLexResult { lines, diagnostics }
}

fn count_leading_spaces(s: &str) -> usize {
    let mut count = 0;
    for ch in s.chars() {
        if ch == ' ' {
            count += 1;
        } else if ch == '\t' {
            count += 1;
        } else {
            break;
        }
    }
    count
}

fn peek_next_non_empty_indent(raw_lines: &[&str], start_idx: usize) -> Option<usize> {
    let mut i = start_idx;
    while i < raw_lines.len() {
        let t = raw_lines[i].trim();
        if t.is_empty() || t.starts_with("# ") || t.starts_with("#\t") {
            i += 1;
            continue;
        }
        if t == "#" {
            // Skip multiline comment block
            let comment_indent = count_leading_spaces(raw_lines[i]);
            i += 1;
            while i < raw_lines.len() {
                if count_leading_spaces(raw_lines[i]) == comment_indent && raw_lines[i].trim() == "#"
                {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        return Some(count_leading_spaces(raw_lines[i]));
    }
    None
}

/// Detects Axiom IV Column-0 Multiline Raw String opener `(tag)?(N)?"` at the end of a line.
/// Examples:
/// - `code ts"` -> head=`"code"`, lang=`"ts"`, rank=`None`
/// - `welcome_msg 1"` -> head=`"welcome_msg"`, lang=`""`, rank=`Some(1)`
/// - `run sh1"` -> head=`"run"`, lang=`"sh"`, rank=`Some(1)`
/// - `input rn2"` -> head=`"input"`, lang=`"rn"`, rank=`Some(2)`
/// - `msg "` -> head=`"msg"`, lang=`""`, rank=`None`
/// Does NOT match closed single-line quotes (`""`, `"hello"`, `Date"12.12.23"`).
fn detect_col0_raw_open(trimmed: &str) -> Option<(String, String, Option<usize>)> {
    if !trimmed.ends_with('"') {
        return None;
    }

    // Count unescaped quotes outside backticks; if even, the trailing `"` closes an inline string!
    let chars: Vec<(usize, char)> = trimmed.char_indices().collect();
    let mut in_quotes = false;
    let mut in_backticks = false;
    let mut last_open_quote_byte = None;

    let mut i = 0;
    while i < chars.len() {
        let (byte_pos, ch) = chars[i];
        if ch == '\\' && in_quotes {
            i += 2;
            continue;
        }
        if ch == '`' && !in_quotes {
            in_backticks = !in_backticks;
        } else if ch == '"' && !in_backticks {
            if !in_quotes {
                in_quotes = true;
                last_open_quote_byte = Some(byte_pos);
            } else {
                in_quotes = false;
                last_open_quote_byte = None;
            }
        }
        i += 1;
    }

    let q_byte = last_open_quote_byte?;
    if q_byte != trimmed.len() - 1 {
        return None;
    }

    // Extract the token immediately preceding the trailing `"`
    let before_quote = &trimmed[..q_byte];
    let token_start = before_quote
        .rfind(|c: char| c.is_whitespace())
        .map(|pos| pos + 1)
        .unwrap_or(0);

    let marker_token = &before_quote[token_start..];
    let head_prefix = before_quote[..token_start].trim().to_string();

    if marker_token.is_empty() {
        return Some((head_prefix, String::new(), None));
    }

    // Split `marker_token` into `(tag)?` and `(N)?`
    let digit_start = marker_token
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(marker_token.len());
    let tag_part = &marker_token[..digit_start];
    let rank_part = &marker_token[digit_start..];

    if !tag_part
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == '_' || c == '-')
    {
        return None;
    }
    if !rank_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let rank_opt = if rank_part.is_empty() {
        None
    } else {
        rank_part.parse::<usize>().ok()
    };

    Some((head_prefix, tag_part.to_string(), rank_opt))
}

/// Detects a trailing unclosed raw quote `"lang` or `"` at the end of a line.
/// Must NOT match closed quotes `""` or `"hello"`.
fn detect_trailing_raw_open(trimmed: &str) -> Option<(String, String)> {
    let chars: Vec<(usize, char)> = trimmed.char_indices().collect();
    let mut in_quotes = false;
    let mut in_backticks = false;
    let mut last_open_quote_byte = None;

    let mut i = 0;
    while i < chars.len() {
        let (byte_pos, ch) = chars[i];
        if ch == '\\' && in_quotes {
            i += 2;
            continue;
        }
        if ch == '`' && !in_quotes {
            in_backticks = !in_backticks;
        } else if ch == '"' && !in_backticks {
            if !in_quotes {
                in_quotes = true;
                last_open_quote_byte = Some(byte_pos);
            } else {
                in_quotes = false;
                last_open_quote_byte = None;
            }
        }
        i += 1;
    }

    // If an unclosed `"` remains open at the end of the line, check if the suffix is a valid lang tag (or empty)
    if in_quotes {
        if let Some(q_pos) = last_open_quote_byte {
            let lang = &trimmed[q_pos + 1..];
            if lang
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                let head = trimmed[..q_pos].to_string();
                return Some((head, lang.to_string()));
            }
        }
    }
    None
}

/// Detects an unquoted, unshielded trailing `[`, `{`, or `(` at the end of a line.
fn detect_trailing_open_bracket(trimmed: &str) -> Option<(&str, char)> {
    let last_char = trimmed.chars().last()?;
    if last_char != '[' && last_char != '{' && last_char != '(' {
        return None;
    }

    // Ensure the trailing bracket is not inside quotes or backticks
    let mut in_quotes = false;
    let mut in_backticks = false;
    let chars: Vec<(usize, char)> = trimmed.char_indices().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        let (_, ch) = chars[i];
        if ch == '\\' && in_quotes {
            i += 2;
            continue;
        }
        if ch == '"' && !in_backticks {
            in_quotes = !in_quotes;
        } else if ch == '`' && !in_quotes {
            in_backticks = !in_backticks;
        }
        i += 1;
    }

    if in_quotes || in_backticks {
        return None;
    }

    let last_byte = chars.last()?.0;
    Some((&trimmed[..last_byte], last_char))
}