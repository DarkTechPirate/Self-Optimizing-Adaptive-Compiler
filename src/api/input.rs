use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLanguage {
    Nyx,
    Python,
    Cpp,
}

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedSource {
    pub source: String,
    pub input_format: String,
    pub source_language: SourceLanguage,
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
    closing_lines: Vec<String>,
}

pub fn normalize_source_input(source: &str) -> NormalizedSource {
    if looks_like_cpp(source) {
        let normalized = translate_cpp_to_nyx(source);
        return NormalizedSource {
            source: normalized,
            input_format: "cpp".to_string(),
            source_language: SourceLanguage::Cpp,
            normalization_applied: true,
        };
    }

    if looks_like_python(source) {
        let normalized = translate_python_to_nyx(source);
        return NormalizedSource {
            source: normalized,
            input_format: "python".to_string(),
            source_language: SourceLanguage::Python,
            normalization_applied: true,
        };
    }

    NormalizedSource {
        source: source.to_string(),
        input_format: "nyx".to_string(),
        source_language: SourceLanguage::Nyx,
        normalization_applied: false,
    }
}

fn looks_like_cpp(source: &str) -> bool {
    let mut has_cpp_marker = false;

    for line in source.lines() {
        let cleaned = strip_cpp_comment(line);
        let trimmed = cleaned.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("#include")
            || trimmed.starts_with("using ")
            || trimmed.starts_with("namespace ")
        {
            return true;
        }

        if trimmed.ends_with('{')
            && parse_cpp_function_signature(trimmed.trim_end_matches('{').trim()).is_some()
        {
            return true;
        }

        if trimmed.contains("std::") || trimmed.contains("::") {
            has_cpp_marker = true;
        }

        if trimmed.contains(';')
            && (starts_with_cpp_type_declaration(trimmed)
                || trimmed.contains("++")
                || trimmed.contains("--")
                || trimmed.starts_with("if (")
                || trimmed.starts_with("for (")
                || trimmed.starts_with("while ("))
        {
            has_cpp_marker = true;
        }

        if trimmed.starts_with("int main(")
            || trimmed.starts_with("long main(")
            || trimmed.starts_with("auto main(")
        {
            return true;
        }
    }

    has_cpp_marker
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

        if trimmed.starts_with("elif ") && trimmed.ends_with(':') {
            has_python_marker = true;
        }

        if trimmed.contains("+=") || trimmed.contains("-=") || trimmed.contains("*=")
            || trimmed.contains("/=") || trimmed.contains("%=")
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
            closing_lines: Vec::new(),
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
                closing_lines: Vec::new(),
            });
            declared_vars.clear();
            continue;
        }

        if let Some(for_range) = parse_for_range(trimmed) {
            if let Some(step) = for_range.step {
                if declared_vars.insert(for_range.var.clone()) {
                    output.push(format!("let {} = {}", for_range.var, for_range.start));
                } else {
                    output.push(format!("{} = {}", for_range.var, for_range.start));
                }
                output.push(format!("while {} < {} {{", for_range.var, for_range.end));
                blocks.push(OpenBlock {
                    indent,
                    kind: BlockKind::Control,
                    closing_lines: vec![format!("{} = {} + {}", for_range.var, for_range.var, step)],
                });
            } else {
                output.push(format!("for {} in {}..{} {{", for_range.var, for_range.start, for_range.end));
                blocks.push(OpenBlock {
                    indent,
                    kind: BlockKind::Control,
                    closing_lines: Vec::new(),
                });
            }
            continue;
        }

        if let Some(condition) = parse_block_condition(trimmed, "while ") {
            output.push(format!("while {} {{", condition));
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Control,
                closing_lines: Vec::new(),
            });
            continue;
        }

        if let Some(condition) = parse_block_condition(trimmed, "if ") {
            output.push(format!("if {} {{", condition));
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Control,
                closing_lines: Vec::new(),
            });
            continue;
        }

        if let Some(condition) = parse_elif(trimmed) {
            close_blocks_for_indent(indent, &mut blocks, &mut output, &mut declared_vars);
            output.push("else {".to_string());
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Control,
                closing_lines: Vec::new(),
            });
            output.push(format!("if {} {{", condition));
            blocks.push(OpenBlock {
                indent: indent + 1,
                kind: BlockKind::Control,
                closing_lines: Vec::new(),
            });
            continue;
        }

        if trimmed == "else:" {
            output.push("else {".to_string());
            blocks.push(OpenBlock {
                indent,
                kind: BlockKind::Control,
                closing_lines: Vec::new(),
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

        if let Some((lhs, op, rhs)) = parse_augmented_assignment(trimmed) {
            let rhs = normalize_expr(&rhs);
            let expr = format!("{} {} {}", lhs, op, rhs);
            if declared_vars.insert(lhs.clone()) {
                output.push(format!("let {} = {}", lhs, expr));
            } else {
                output.push(format!("{} = {}", lhs, expr));
            }
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
        for line in &block.closing_lines {
            output.push(line.clone());
        }
        output.push("}".to_string());
        if matches!(block.kind, BlockKind::Function | BlockKind::Wrapper) {
            declared_vars.clear();
        }
    }

    output.join("\n")
}

fn translate_cpp_to_nyx(source: &str) -> String {
    let source = source.replace('\t', "    ");
    let has_function_def = source
        .lines()
        .any(|line| parse_cpp_function_signature(strip_cpp_comment(line).trim()).is_some());

    let mut output = Vec::new();
    let mut blocks = Vec::<OpenBlock>::new();
    let mut declared_vars = HashSet::<String>::new();

    if !has_function_def {
        output.push("fn main() {".to_string());
        blocks.push(OpenBlock {
            indent: 0,
            kind: BlockKind::Wrapper,
            closing_lines: Vec::new(),
        });
    }

    for raw_line in source.lines() {
        let cleaned = strip_cpp_comment(raw_line);
        let mut trimmed = cleaned.trim().to_string();
        if trimmed.is_empty() || trimmed.starts_with("#include") || trimmed.starts_with("using ") {
            continue;
        }

        while trimmed.starts_with('}') {
            close_blocks_for_cpp_brace(&mut blocks, &mut output, &mut declared_vars);
            trimmed = trimmed[1..].trim_start().to_string();
        }

        if trimmed.is_empty() || trimmed == ";" || trimmed == "{" {
            continue;
        }

        if trimmed.ends_with('{') {
            let block_head = trimmed.trim_end_matches('{').trim();

            if let Some((name, params)) = parse_cpp_function_signature(block_head) {
                output.push(format!("fn {}({}) {{", name, params.join(", ")));
                blocks.push(OpenBlock {
                    indent: 0,
                    kind: BlockKind::Function,
                    closing_lines: Vec::new(),
                });
                declared_vars.clear();
                continue;
            }

            if let Some((var, start, end, step, inclusive)) = parse_cpp_for_loop(block_head) {
                if declared_vars.insert(var.clone()) {
                    output.push(format!("let {} = {}", var, start));
                } else {
                    output.push(format!("{} = {}", var, start));
                }

                let comparator = if inclusive { "<=" } else { "<" };
                output.push(format!("while {} {} {} {{", var, comparator, end));
                let mut closing_lines = Vec::new();
                closing_lines.push(format!("{} = {} + {}", var, var, step));
                blocks.push(OpenBlock {
                    indent: 0,
                    kind: BlockKind::Control,
                    closing_lines,
                });
                continue;
            }

            if let Some(condition) = parse_cpp_condition(block_head, "if ") {
                output.push(format!("if {} {{", condition));
                blocks.push(OpenBlock {
                    indent: 0,
                    kind: BlockKind::Control,
                    closing_lines: Vec::new(),
                });
                continue;
            }

            if let Some(condition) = parse_cpp_condition(block_head, "while ") {
                output.push(format!("while {} {{", condition));
                blocks.push(OpenBlock {
                    indent: 0,
                    kind: BlockKind::Control,
                    closing_lines: Vec::new(),
                });
                continue;
            }

            if block_head.starts_with("else if ") {
                close_blocks_for_cpp_brace(&mut blocks, &mut output, &mut declared_vars);
                output.push("else {".to_string());
                blocks.push(OpenBlock {
                    indent: 0,
                    kind: BlockKind::Control,
                    closing_lines: Vec::new(),
                });
                if let Some(condition) = parse_cpp_condition(block_head, "else if ") {
                    output.push(format!("if {} {{", condition));
                    blocks.push(OpenBlock {
                        indent: 0,
                        kind: BlockKind::Control,
                        closing_lines: Vec::new(),
                    });
                }
                continue;
            }

            if block_head == "else" {
                output.push("else {".to_string());
                blocks.push(OpenBlock {
                    indent: 0,
                    kind: BlockKind::Control,
                    closing_lines: Vec::new(),
                });
                continue;
            }

            continue;
        }

        let statement = trimmed.trim_end_matches(';').trim();

        if let Some(expr) = statement.strip_prefix("return ") {
            output.push(format!("return {}", normalize_expr(expr)));
            continue;
        }

        if let Some((lhs, op, rhs)) = parse_augmented_assignment(statement) {
            let rhs = normalize_expr(&rhs);
            let expr = format!("{} {} {}", lhs, op, rhs);
            if declared_vars.insert(lhs.clone()) {
                output.push(format!("let {} = {}", lhs, expr));
            } else {
                output.push(format!("{} = {}", lhs, expr));
            }
            continue;
        }

        if let Some((lhs, rhs)) = parse_cpp_declaration(statement) {
            let rhs = normalize_expr(&rhs);
            if declared_vars.insert(lhs.clone()) {
                output.push(format!("let {} = {}", lhs, rhs));
            } else {
                output.push(format!("{} = {}", lhs, rhs));
            }
            continue;
        }

        if let Some((lhs, rhs)) = parse_assignment(statement) {
            let rhs = normalize_expr(&rhs);
            if declared_vars.insert(lhs.clone()) {
                output.push(format!("let {} = {}", lhs, rhs));
            } else {
                output.push(format!("{} = {}", lhs, rhs));
            }
            continue;
        }

        if let Some(call) = parse_cpp_call(statement) {
            output.push(call);
            continue;
        }
    }

    while let Some(block) = blocks.pop() {
        for line in &block.closing_lines {
            output.push(line.clone());
        }
        output.push("}".to_string());
        if matches!(block.kind, BlockKind::Function | BlockKind::Wrapper) {
            declared_vars.clear();
        }
    }

    output.join("\n")
}

fn close_blocks_for_cpp_brace(
    blocks: &mut Vec<OpenBlock>,
    output: &mut Vec<String>,
    declared_vars: &mut HashSet<String>,
) {
    if let Some(closed) = blocks.pop() {
        for line in &closed.closing_lines {
            output.push(line.clone());
        }
        output.push("}".to_string());
        if matches!(closed.kind, BlockKind::Function) {
            declared_vars.clear();
        }
    }
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
            for line in &closed.closing_lines {
                output.push(line.clone());
            }
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

#[derive(Debug, Clone)]
struct ForRange {
    var: String,
    start: String,
    end: String,
    step: Option<String>,
}

fn parse_for_range(line: &str) -> Option<ForRange> {
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

    let (start, end, step) = match args.len() {
        1 => ("0".to_string(), args[0].clone(), None),
        2 => (args[0].clone(), args[1].clone(), None),
        _ => (args[0].clone(), args[1].clone(), Some(args[2].clone())),
    };

    Some(ForRange {
        var: var.to_string(),
        start: normalize_expr(&start),
        end: normalize_expr(&end),
        step: step.map(|s| normalize_expr(&s)),
    })
}

fn parse_block_condition(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?.strip_suffix(':')?.trim();
    if rest.is_empty() {
        return None;
    }
    Some(normalize_expr(rest))
}

fn parse_elif(line: &str) -> Option<String> {
    let rest = line.strip_prefix("elif ")?.strip_suffix(':')?.trim();
    if rest.is_empty() {
        return None;
    }
    Some(normalize_expr(rest))
}

fn parse_cpp_function_signature(line: &str) -> Option<(String, Vec<String>)> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    if close <= open {
        return None;
    }

    let head = line[..open].trim();
    let head_tokens: Vec<&str> = head.split_whitespace().collect();
    if head_tokens.len() < 2 {
        return None;
    }

    let first_token = head_tokens.first().copied().unwrap_or("");
    if matches!(first_token, "fn" | "if" | "for" | "while" | "switch" | "else") {
        return None;
    }

    let name = head_tokens.last()?.trim_matches(&['*', '&'][..]);
    if name.is_empty() {
        return None;
    }

    let params = line[open + 1..close]
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(normalize_cpp_parameter)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();

    Some((name.to_string(), params))
}

fn normalize_cpp_parameter(param: &str) -> String {
    let without_default = param.split('=').next().unwrap_or(param).trim();
    let without_array = without_default.split('[').next().unwrap_or(without_default).trim();
    let candidate = without_array
        .rsplit_once(' ')
        .map(|(_, tail)| tail)
        .unwrap_or(without_array)
        .trim();

    candidate.trim_matches(&['*', '&'][..]).to_string()
}

fn parse_cpp_condition(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?.trim();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }

    Some(normalize_expr(rest[1..rest.len() - 1].trim()))
}

fn parse_cpp_for_loop(line: &str) -> Option<(String, String, String, String, bool)> {
    let rest = line.strip_prefix("for")?.trim();
    let rest = rest.strip_prefix('(')?.strip_suffix(')')?.trim();
    let parts: Vec<&str> = rest.split(';').map(|part| part.trim()).collect();
    if parts.len() != 3 {
        return None;
    }

    let (var, start) = parse_cpp_for_init(parts[0])?;
    let (cond_var, end, inclusive) = parse_cpp_for_condition(parts[1])?;
    let step = parse_cpp_for_step(parts[2], &var)?;

    if cond_var != var {
        return None;
    }

    Some((var, start, end, step, inclusive))
}

fn parse_cpp_for_init(part: &str) -> Option<(String, String)> {
    let part = part.trim();
    let part = part.strip_prefix("int ")
        .or_else(|| part.strip_prefix("auto "))
        .or_else(|| part.strip_prefix("long "))
        .or_else(|| part.strip_prefix("size_t "))
        .unwrap_or(part);

    let (lhs, rhs) = part.split_once('=')?;
    let var = lhs.split_whitespace().last()?.trim_matches(&['*', '&'][..]);
    let start = normalize_expr(rhs);
    if var.is_empty() || start.is_empty() {
        return None;
    }

    Some((var.to_string(), start))
}

fn parse_cpp_for_condition(part: &str) -> Option<(String, String, bool)> {
    if let Some((lhs, rhs)) = part.split_once("<=") {
        let var = lhs.trim().to_string();
        let end = normalize_expr(rhs);
        return Some((var, end, true));
    }

    if let Some((lhs, rhs)) = part.split_once('<') {
        let var = lhs.trim().to_string();
        let end = normalize_expr(rhs);
        return Some((var, end, false));
    }

    if let Some((lhs, rhs)) = part.split_once(">=") {
        let var = lhs.trim().to_string();
        let end = normalize_expr(rhs);
        return Some((var, end, true));
    }

    if let Some((lhs, rhs)) = part.split_once('>') {
        let var = lhs.trim().to_string();
        let end = normalize_expr(rhs);
        return Some((var, end, false));
    }

    None
}

fn parse_cpp_for_step(part: &str, var: &str) -> Option<String> {
    let part = part.trim();
    if part == format!("{}++", var) || part == format!("++{}", var) {
        return Some("1".to_string());
    }
    if part == format!("{}--", var) || part == format!("--{}", var) {
        return Some("-1".to_string());
    }
    if let Some((lhs, rhs)) = part.split_once("+=") {
        if lhs.trim() == var {
            return Some(normalize_expr(rhs));
        }
    }
    if let Some((lhs, rhs)) = part.split_once("-=") {
        if lhs.trim() == var {
            return Some(format!("-{}", normalize_expr(rhs)));
        }
    }
    if let Some((lhs, rhs)) = part.split_once('=') {
        if lhs.trim() == var {
            let rhs = rhs.trim();
            if let Some(rest) = rhs.strip_prefix(&format!("{} + ", var)) {
                return Some(normalize_expr(rest));
            }
            if let Some(rest) = rhs.strip_prefix(&format!("{} - ", var)) {
                return Some(format!("-{}", normalize_expr(rest)));
            }
        }
    }

    None
}

fn parse_cpp_declaration(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let declaration_prefixes = [
        "auto ",
        "bool ",
        "char ",
        "double ",
        "float ",
        "int ",
        "long ",
        "short ",
        "size_t ",
        "unsigned ",
    ];

    let mut rest = None;
    for prefix in declaration_prefixes {
        if let Some(stripped) = trimmed.strip_prefix(prefix) {
            rest = Some(stripped.trim());
            break;
        }
    }

    let rest = rest?;
    let (lhs, rhs) = rest.split_once('=')?;
    let name = lhs.split_whitespace().last()?.trim_matches(&['*', '&'][..]);
    if name.is_empty() {
        return None;
    }

    Some((name.to_string(), rhs.trim().to_string()))
}

fn starts_with_cpp_type_declaration(line: &str) -> bool {
    let declaration_prefixes = [
        "auto ",
        "bool ",
        "char ",
        "double ",
        "float ",
        "int ",
        "long ",
        "short ",
        "size_t ",
        "unsigned ",
    ];

    declaration_prefixes.iter().any(|prefix| line.starts_with(prefix))
}

fn parse_cpp_call(line: &str) -> Option<String> {
    let expr = line.trim();
    if !expr.ends_with(')') || !expr.contains('(') {
        return None;
    }
    if expr.contains("<<") {
        return None;
    }

    Some(expr.to_string())
}

fn strip_cpp_comment(line: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;

    for (idx, ch) in line.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '/' if !in_single && !in_double => {
                if line[idx..].starts_with("//") {
                    return line[..idx].to_string();
                }
            }
            _ => {}
        }
    }

    line.to_string()
}

fn parse_augmented_assignment(line: &str) -> Option<(String, String, String)> {
    let ops = ["+=", "-=", "*=", "/=", "%="];
    for op in ops {
        if let Some((lhs, rhs)) = line.split_once(op) {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if lhs.is_empty() || rhs.is_empty() {
                return None;
            }
            if !lhs.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return None;
            }
            let binary_op = &op[..1];
            return Some((lhs.to_string(), binary_op.to_string(), rhs.to_string()));
        }
    }
    None
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

    #[test]
    fn translates_cpp_function_to_nyx() {
        let source = r#"
#include <iostream>

int sum(int n) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        total += i;
    }
    return total;
}
"#;

        let normalized = normalize_source_input(source);
        assert_eq!(normalized.input_format, "cpp");
        assert!(normalized.normalization_applied);
        assert!(normalized.source.contains("fn sum(n) {"));
        assert!(normalized.source.contains("let total = 0"));
        assert!(normalized.source.contains("while i < n {"));
        assert!(normalized.source.contains("i = i + 1"));
        assert!(normalized.source.contains("total = total + i"));
        assert!(normalized.source.contains("return total"));
    }

    #[test]
    fn wraps_top_level_cpp_script() {
        let source = r#"
int x = 1;
int y = 2;
if (x < y) {
    y += 1;
}
"#;

        let normalized = normalize_source_input(source);
        assert_eq!(normalized.input_format, "cpp");
        assert!(normalized.source.starts_with("fn main() {"));
        assert!(normalized.source.contains("if x < y {"));
        assert!(normalized.source.contains("let x = 1"));
        assert!(normalized.source.contains("let y = 2"));
    }

    #[test]
    fn translates_python_elif_chain() {
        let source = r#"
x = 3
if x < 0:
    x = 0
elif x < 5:
    x = x + 1
else:
    x = x - 1
"#;

        let normalized = normalize_source_input(source);
        assert!(normalized.source.contains("else {"));
        assert!(normalized.source.contains("if x < 5 {"));
    }

    #[test]
    fn translates_augmented_assignment() {
        let source = r#"
x = 1
x += 2
"#;

        let normalized = normalize_source_input(source);
        assert!(normalized.source.contains("x = x + 2"));
    }

    #[test]
    fn translates_range_with_step_to_while() {
        let source = r#"
for i in range(0, 6, 2):
    x = i
"#;

        let normalized = normalize_source_input(source);
        assert!(normalized.source.contains("let i = 0"));
        assert!(normalized.source.contains("while i < 6"));
        assert!(normalized.source.contains("i = i + 2"));
    }
}