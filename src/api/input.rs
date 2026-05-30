use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedSource {
    pub source: String,
    pub input_format: String,
    pub normalization_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockKind {
    Wrapper,
    Function,
    Control,
}

#[derive(Debug, Clone)]
struct OpenBlock {
    indent: usize,
    kind: BlockKind,
}

pub fn normalize_source_input(source: &str) -> NormalizedSource {
    if looks_like_python(source) {
        let normalized = translate_python_to_nyx(source);
        return NormalizedSource {
            source: normalized,
            input_format: "python".to_string(),
            normalization_applied: true,
        };
    }

    NormalizedSource {
        source: source.to_string(),
        input_format: "nyx".to_string(),
        normalization_applied: false,
    }
}

fn looks_like_python(source: &str) -> bool {
    let mut has_python_marker = false;

    for line in source.lines() {
        let cleaned = strip_inline_comment(line);
        let trimmed = cleaned.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("def ") && trimmed.ends_with(':') {
            return true;
        }

        if trimmed.starts_with("for ") && trimmed.contains(" in range(") && trimmed.ends_with(':') {
            return true;
        }

        if (trimmed.starts_with("if ") || trimmed.starts_with("while ") || trimmed == "else:")
            && trimmed.ends_with(':')
        {
            has_python_marker = true;
        }
    }

    has_python_marker
}

fn translate_python_to_nyx(source: &str) -> String {
    let source = source.replace('\t', "    ");
    let has_function_def = source
        .lines()
        .any(|line| strip_inline_comment(line).trim().starts_with("def "));

    let mut output = Vec::new();
    let mut blocks = Vec::<OpenBlock>::new();
    let mut declared_vars = HashSet::<String>::new();

    if !has_function_def {
        output.push("fn main() {".to_string());
        blocks.push(OpenBlock {
            indent: 0,
            kind: BlockKind::Wrapper,
        });
    }

    for raw_line in source.lines() {
        let cleaned = strip_inline_comment(raw_line);
        let trimmed = cleaned.trim();
        if trimmed.is_empty() {
            continue;
        }

        let indent = raw_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum::<usize>();

        close_blocks_for_indent(indent, &mut blocks, &mut output, &mut declared_vars);

        if let Some((name, params)) = parse_def(trimmed) {
            output.push(format!("fn {}({}) {{", name, params.join(", ")));
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Function,
            });
            declared_vars.clear();
            continue;
        }

        if let Some((var, start, end)) = parse_for_range(trimmed) {
            output.push(format!("for {} in {}..{} {{", var, start, end));
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Control,
            });
            continue;
        }

        if let Some(condition) = parse_block_condition(trimmed, "while ") {
            output.push(format!("while {} {{", condition));
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Control,
            });
            continue;
        }

        if let Some(condition) = parse_block_condition(trimmed, "if ") {
            output.push(format!("if {} {{", condition));
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Control,
            });
            continue;
        }

        if trimmed == "else:" {
            output.push("else {".to_string());
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Control,
            });
            continue;
        }

        if let Some(expr) = trimmed.strip_prefix("return ") {
            output.push(format!("return {}", normalize_expr(expr)));
            continue;
        }

        if trimmed == "pass" {
            continue;
        }

        if let Some((lhs, rhs)) = parse_assignment(trimmed) {
            let rhs = normalize_expr(&rhs);
            if declared_vars.insert(lhs.clone()) {
                output.push(format!("let {} = {}", lhs, rhs));
            } else {
                output.push(format!("{} = {}", lhs, rhs));
            }
            continue;
        }

        if trimmed.starts_with("return") {
            output.push("return 0".to_string());
        }
    }

    while let Some(block) = blocks.pop() {
        output.push("}".to_string());
        if matches!(block.kind, BlockKind::Function | BlockKind::Wrapper) {
            declared_vars.clear();
        }
    }

    output.join("\n")
}

fn close_blocks_for_indent(
    current_indent: usize,
    blocks: &mut Vec<OpenBlock>,
    output: &mut Vec<String>,
    declared_vars: &mut HashSet<String>,
) {
    loop {
        let should_close = match blocks.last() {
            Some(OpenBlock {
                kind: BlockKind::Wrapper,
                ..
            }) => false,
            Some(last) => current_indent <= last.indent,
            None => false,
        };

        if !should_close {
            break;
        }

        if let Some(closed) = blocks.pop() {
            output.push("}".to_string());
            if matches!(closed.kind, BlockKind::Function) {
                declared_vars.clear();
            }
        }
    }
}

fn parse_def(line: &str) -> Option<(String, Vec<String>)> {
    let rest = line.strip_prefix("def ")?.strip_suffix(':')?.trim();
    let open = rest.find('(')?;
    let close = rest.rfind(')')?;
    if close <= open {
        return None;
    }

    let name = rest[..open].trim();
    if name.is_empty() {
        return None;
    }

    let params = rest[open + 1..close]
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(normalize_parameter)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();

    Some((name.to_string(), params))
}

fn normalize_parameter(param: &str) -> String {
    let without_type = param.split(':').next().unwrap_or(param);
    let without_default = without_type.split('=').next().unwrap_or(without_type);
    without_default.trim().trim_start_matches('*').to_string()
}

fn parse_for_range(line: &str) -> Option<(String, String, String)> {
    let rest = line.strip_prefix("for ")?.strip_suffix(':')?.trim();
    let (var_part, iter_part) = rest.split_once(" in ")?;
    let var = var_part.trim();
    if var.is_empty() {
        return None;
    }

    let range = iter_part.trim();
    if !(range.starts_with("range(") && range.ends_with(')')) {
        return None;
    }

    let inner = &range[6..range.len() - 1];
    let args: Vec<String> = inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let (start, end) = match args.len() {
        1 => ("0".to_string(), args[0].clone()),
        _ => (args[0].clone(), args[1].clone()),
    };

    Some((var.to_string(), normalize_expr(&start), normalize_expr(&end)))
}

fn parse_block_condition(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?.strip_suffix(':')?.trim();
    if rest.is_empty() {
        return None;
    }
    Some(normalize_expr(rest))
}

fn parse_assignment(line: &str) -> Option<(String, String)> {
    let idx = line.find('=')?;

    if idx > 0 {
        let prev = line[..idx].chars().last()?;
        if matches!(prev, '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%') {
            return None;
        }
    }

    if idx + 1 < line.len() {
        let next = line[idx + 1..].chars().next()?;
        if next == '=' {
            return None;
        }
    }

    let lhs = line[..idx].trim();
    let rhs = line[idx + 1..].trim();
    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }

    if !lhs.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }

    Some((lhs.to_string(), rhs.to_string()))
}

fn normalize_expr(expr: &str) -> String {
    expr.trim()
        .replace("True", "1")
        .replace("False", "0")
}

fn strip_inline_comment(line: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;

    for (idx, ch) in line.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return line[..idx].to_string(),
            _ => {}
        }
    }

    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_nyx_source_unchanged() {
        let source = "fn sum() {\nlet x = 1\nreturn x\n}";
        let normalized = normalize_source_input(source);
        assert_eq!(normalized.input_format, "nyx");
        assert!(!normalized.normalization_applied);
        assert_eq!(normalized.source, source);
    }

    #[test]
    fn translates_python_function_to_nyx() {
        let source = r#"
def hot_path():
    total = 0
    for i in range(1, 6):
        total = total + i
    return total
"#;

        let normalized = normalize_source_input(source);
        assert_eq!(normalized.input_format, "python");
        assert!(normalized.normalization_applied);
        assert!(normalized.source.contains("fn hot_path() {"));
        assert!(normalized.source.contains("for i in 1..6 {"));
        assert!(normalized.source.contains("let total = 0"));
        assert!(normalized.source.contains("total = total + i"));
    }

    #[test]
    fn wraps_top_level_python_script() {
        let source = r#"
x = 1
y = 2
if x < y:
    y = y + 1
"#;

        let normalized = normalize_source_input(source);
        assert!(normalized.source.starts_with("fn main() {"));
        assert!(normalized.source.contains("if x < y {"));
        assert!(normalized.source.ends_with('}'));
    }
}