use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scenario {
    pub steps: Vec<Step>,
    #[serde(default)]
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    Import(ImportStep),
    AssetGroup(AssetGroupStep),
    Scan(ScanStep),
    Variable(VariableDecl),
    Script(ScriptStep),
    Report(ReportStep),
    Conditional(ConditionalStep),
    Loop(LoopStep),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStep {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGroupStep {
    pub name: String,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStep {
    pub name: String,
    pub tool: String,
    pub params: BTreeMap<String, String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptStep {
    pub name: String,
    pub params: BTreeMap<String, String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportStep {
    pub name: String,
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalStep {
    pub condition: ConditionExpr,
    pub then_steps: Vec<Step>,
    #[serde(default)]
    pub else_steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStep {
    pub iterator: String,
    pub iterable: LoopIterable,
    pub body: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionExpr {
    Literal(bool),
    Variable(String),
    Not(Box<ConditionExpr>),
    Equals(ConditionOperand, ConditionOperand),
    NotEquals(ConditionOperand, ConditionOperand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperand {
    Variable(String),
    Literal(LiteralValue),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopIterable {
    Variable(String),
    Literal(LiteralValue),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDecl {
    pub name: String,
    pub value: LiteralValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<LiteralValue>),
    Object(BTreeMap<String, LiteralValue>),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("unexpected end of input while parsing {0}")]
    UnexpectedEof(&'static str),
    #[error("invalid directive: {0}")]
    InvalidDirective(String),
    #[error("invalid syntax in line: {0}")]
    InvalidSyntax(String),
    #[error("missing required value: {0}")]
    MissingValue(&'static str),
}

pub fn parse_scenario(source: &str) -> Result<Scenario, ParseError> {
    let mut lines = source.lines().enumerate().peekable();
    let mut steps = Vec::new();
    let mut imports = Vec::new();

    while let Some((_, raw_line)) = next_non_empty(&mut lines) {
        let trimmed = raw_line.trim();
        let step = parse_step_internal(trimmed, &mut lines, &mut imports)?;
        steps.push(step);
    }

    Ok(Scenario { steps, imports })
}

fn parse_step_internal<'a, I>(
    first_line: &str,
    lines: &mut PeekableLines<'a, I>,
    imports: &mut Vec<String>,
) -> Result<Step, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    if first_line.starts_with("import ") {
        let path = parse_import(first_line)?;
        imports.push(path.clone());
        Ok(Step::Import(ImportStep { path }))
    } else if first_line.starts_with("asset_group ") || first_line.starts_with("group ") {
        let step = parse_asset_group(first_line, lines)?;
        Ok(Step::AssetGroup(step))
    } else if first_line.starts_with("scan ") {
        let step = parse_scan(first_line, lines)?;
        Ok(Step::Scan(step))
    } else if first_line.starts_with("let ") {
        let step = parse_variable(first_line)?;
        Ok(Step::Variable(step))
    } else if first_line.starts_with("script ") {
        let step = parse_script(first_line, lines)?;
        Ok(Step::Script(step))
    } else if first_line.starts_with("report ") {
        let step = parse_report(first_line, lines)?;
        Ok(Step::Report(step))
    } else if first_line.starts_with("if ") {
        let step = parse_if(first_line, lines, imports)?;
        Ok(Step::Conditional(step))
    } else if first_line.starts_with("for ") {
        let step = parse_for(first_line, lines, imports)?;
        Ok(Step::Loop(step))
    } else if first_line.starts_with("else") {
        Err(ParseError::InvalidDirective(first_line.to_string()))
    } else {
        Err(ParseError::InvalidDirective(first_line.to_string()))
    }
}

fn parse_asset_group<'a, I>(
    first_line: &str,
    lines: &mut PeekableLines<'a, I>,
) -> Result<AssetGroupStep, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let (header, mut body) = split_header_body(first_line)?;
    let tokens: Vec<&str> = header.split_whitespace().collect();
    if tokens.len() != 2 || !matches!(tokens[0], "asset_group" | "group") {
        return Err(ParseError::InvalidSyntax(first_line.to_string()));
    }

    let name = tokens[1];
    let mut properties = BTreeMap::new();

    if let Some(content) = body.take() {
        if let Some((segment, rest)) = content.split_once('}') {
            parse_properties_segment(segment, &mut properties)?;
            if !rest.trim().is_empty() {
                return Err(ParseError::InvalidSyntax(rest.to_string()));
            }
            return Ok(AssetGroupStep {
                name: name.to_string(),
                properties,
            });
        } else {
            parse_properties_segment(content, &mut properties)?;
        }
    }

    loop {
        let (_, raw_line) = lines
            .next()
            .ok_or(ParseError::UnexpectedEof("asset_group block"))?;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        if let Some((segment, rest)) = trimmed.split_once('}') {
            parse_properties_segment(segment, &mut properties)?;
            if !rest.trim().is_empty() {
                return Err(ParseError::InvalidSyntax(rest.to_string()));
            }
            break;
        } else {
            parse_properties_segment(trimmed, &mut properties)?;
        }
    }

    Ok(AssetGroupStep {
        name: name.to_string(),
        properties,
    })
}

fn parse_scan<'a, I>(
    first_line: &str,
    lines: &mut PeekableLines<'a, I>,
) -> Result<ScanStep, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let (name, tool) = parse_scan_header(first_line)?;
    let mut params = BTreeMap::new();
    let mut output = None;

    loop {
        let (_, raw_line) = next_non_empty(lines).ok_or(ParseError::UnexpectedEof("scan block"))?;
        let trimmed = raw_line.trim();

        if trimmed.starts_with('}') {
            if let Some(pos) = trimmed.find("->") {
                let candidate = trimmed[pos + 2..].trim();
                if !candidate.is_empty() {
                    output = Some(candidate.to_string());
                }
            }
            break;
        }

        let mut parts = trimmed.splitn(2, ' ');
        let key = parts
            .next()
            .ok_or_else(|| ParseError::InvalidSyntax(trimmed.to_string()))?;
        let value = parts
            .next()
            .ok_or_else(|| ParseError::InvalidSyntax(trimmed.to_string()))?;
        let parsed_value = parse_quoted(value)?;
        params.insert(key.to_string(), parsed_value);
    }

    Ok(ScanStep {
        name: name.to_string(),
        tool: tool.to_string(),
        params,
        output,
    })
}

fn parse_variable(line: &str) -> Result<VariableDecl, ParseError> {
    let cleaned = line.trim_end_matches(';').trim();
    let rest = cleaned
        .strip_prefix("let")
        .ok_or_else(|| ParseError::InvalidSyntax(line.to_string()))?
        .trim();

    let (name_part, value_part) = rest
        .split_once('=')
        .ok_or_else(|| ParseError::InvalidSyntax(line.to_string()))?;

    let name = name_part.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ParseError::InvalidSyntax(name.to_string()));
    }

    let value = parse_literal(value_part)?;

    Ok(VariableDecl {
        name: name.to_string(),
        value,
    })
}

fn parse_import(line: &str) -> Result<String, ParseError> {
    let cleaned = line.trim_end_matches(';').trim();
    let rest = cleaned
        .strip_prefix("import")
        .ok_or_else(|| ParseError::InvalidSyntax(line.to_string()))?
        .trim();
    if rest.is_empty() {
        return Err(ParseError::MissingValue("import path"));
    }
    parse_quoted(rest)
}

fn parse_script<'a, I>(
    first_line: &str,
    lines: &mut PeekableLines<'a, I>,
) -> Result<ScriptStep, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let cleaned = first_line.trim_end_matches('{').trim();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.len() < 2 || tokens[0] != "script" {
        return Err(ParseError::InvalidSyntax(first_line.to_string()));
    }

    let name = tokens[1];
    let mut params = BTreeMap::new();
    let mut output = None;

    loop {
        let (_, raw_line) =
            next_non_empty(lines).ok_or(ParseError::UnexpectedEof("script block"))?;
        let trimmed = raw_line.trim();

        if trimmed.starts_with('}') {
            if let Some(pos) = trimmed.find("->") {
                let candidate = trimmed[pos + 2..].trim();
                if !candidate.is_empty() {
                    output = Some(candidate.to_string());
                }
            }
            break;
        }

        let mut parts = trimmed.splitn(2, ' ');
        let key = parts
            .next()
            .ok_or_else(|| ParseError::InvalidSyntax(trimmed.to_string()))?;
        let value = parts
            .next()
            .ok_or_else(|| ParseError::InvalidSyntax(trimmed.to_string()))?;
        let parsed_value = parse_quoted(value)?;
        params.insert(key.to_string(), parsed_value);
    }

    if !params.contains_key("run") {
        return Err(ParseError::MissingValue("script run"));
    }

    Ok(ScriptStep {
        name: name.to_string(),
        params,
        output,
    })
}

fn parse_report<'a, I>(
    first_line: &str,
    lines: &mut PeekableLines<'a, I>,
) -> Result<ReportStep, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let name = parse_report_header(first_line)?;
    let mut includes = Vec::new();

    loop {
        let (_, raw_line) =
            next_non_empty(lines).ok_or(ParseError::UnexpectedEof("report block"))?;
        let trimmed = raw_line.trim();

        if trimmed.starts_with('}') {
            break;
        }

        if trimmed.starts_with("include ") {
            includes.push(trimmed["include ".len()..].trim().to_string());
        } else {
            return Err(ParseError::InvalidSyntax(trimmed.to_string()));
        }
    }

    Ok(ReportStep {
        name: name.to_string(),
        includes,
    })
}

fn parse_if<'a, I>(
    first_line: &str,
    lines: &mut PeekableLines<'a, I>,
    imports: &mut Vec<String>,
) -> Result<ConditionalStep, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let (header, body) = split_header_body(first_line)?;
    let condition_raw = header
        .strip_prefix("if")
        .ok_or_else(|| ParseError::InvalidSyntax(first_line.to_string()))?
        .trim();
    if condition_raw.is_empty() {
        return Err(ParseError::InvalidSyntax(first_line.to_string()));
    }
    if let Some(content) = body {
        if !content.is_empty() {
            return Err(ParseError::InvalidSyntax(content.to_string()));
        }
    }

    let condition = parse_condition_expr(condition_raw)?;
    let (then_steps, trailing) = parse_block_steps(lines, imports)?;

    let mut else_steps = Vec::new();
    let mut remaining = trailing;

    if remaining.is_none() {
        if let Some((_, peek_line)) = peek_non_empty(lines) {
            let trimmed = peek_line.trim();
            if trimmed.starts_with("else") {
                let (_, consumed) =
                    next_non_empty(lines).ok_or(ParseError::UnexpectedEof("else block"))?;
                remaining = Some(consumed.trim().to_string());
            }
        }
    }

    if let Some(clause) = remaining {
        if clause.starts_with("else if ") {
            let nested_line = clause["else ".len()..].trim_start();
            let nested = parse_if(nested_line, lines, imports)?;
            else_steps.push(Step::Conditional(nested));
        } else if clause.starts_with("else") {
            let (header, remainder) = split_header_body(&clause)?;
            if header != "else" {
                return Err(ParseError::InvalidSyntax(clause));
            }
            if let Some(content) = remainder {
                if !content.is_empty() {
                    return Err(ParseError::InvalidSyntax(content.to_string()));
                }
            }
            let (steps, trailing_after_else) = parse_block_steps(lines, imports)?;
            if let Some(rest) = trailing_after_else {
                return Err(ParseError::InvalidSyntax(rest));
            }
            else_steps = steps;
        } else {
            return Err(ParseError::InvalidSyntax(clause));
        }
    }

    Ok(ConditionalStep {
        condition,
        then_steps,
        else_steps,
    })
}

fn parse_for<'a, I>(
    first_line: &str,
    lines: &mut PeekableLines<'a, I>,
    imports: &mut Vec<String>,
) -> Result<LoopStep, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let (header, body) = split_header_body(first_line)?;
    let rest = header
        .strip_prefix("for")
        .ok_or_else(|| ParseError::InvalidSyntax(first_line.to_string()))?
        .trim();

    let in_pos = rest
        .find(" in ")
        .ok_or_else(|| ParseError::InvalidSyntax(first_line.to_string()))?;
    let iterator = rest[..in_pos].trim();
    let iterable_raw = rest[in_pos + 4..].trim();

    if iterator.is_empty() || !is_identifier(iterator) {
        return Err(ParseError::InvalidSyntax(iterator.to_string()));
    }
    if iterable_raw.is_empty() {
        return Err(ParseError::InvalidSyntax(first_line.to_string()));
    }

    if let Some(content) = body {
        if !content.is_empty() {
            return Err(ParseError::InvalidSyntax(content.to_string()));
        }
    }

    let iterable = parse_loop_iterable(iterable_raw)?;
    let (body_steps, trailing) = parse_block_steps(lines, imports)?;
    if let Some(rest) = trailing {
        return Err(ParseError::InvalidSyntax(rest));
    }

    Ok(LoopStep {
        iterator: iterator.to_string(),
        iterable,
        body: body_steps,
    })
}

fn parse_block_steps<'a, I>(
    lines: &mut PeekableLines<'a, I>,
    imports: &mut Vec<String>,
) -> Result<(Vec<Step>, Option<String>), ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut steps = Vec::new();
    loop {
        let (_, raw_line) = next_non_empty(lines).ok_or(ParseError::UnexpectedEof("block"))?;
        let trimmed = raw_line.trim();
        if trimmed.starts_with('}') {
            let remainder = trimmed[1..].trim();
            if remainder.is_empty() {
                return Ok((steps, None));
            } else {
                return Ok((steps, Some(remainder.to_string())));
            }
        }
        let step = parse_step_internal(trimmed, lines, imports)?;
        steps.push(step);
    }
}

fn peek_non_empty<'a, I>(lines: &mut PeekableLines<'a, I>) -> Option<(usize, &'a str)>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    while let Some((idx, line)) = lines.peek() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            lines.next();
            continue;
        }
        return Some((*idx, *line));
    }
    None
}

fn parse_condition_expr(expr: &str) -> Result<ConditionExpr, ParseError> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(ParseError::InvalidSyntax(expr.to_string()));
    }

    if let Some(pos) = find_operator(trimmed, "==") {
        let left = &trimmed[..pos];
        let right = &trimmed[pos + 2..];
        let left_operand = parse_condition_operand(left)?;
        let right_operand = parse_condition_operand(right)?;
        return Ok(ConditionExpr::Equals(left_operand, right_operand));
    }
    if let Some(pos) = find_operator(trimmed, "!=") {
        let left = &trimmed[..pos];
        let right = &trimmed[pos + 2..];
        let left_operand = parse_condition_operand(left)?;
        let right_operand = parse_condition_operand(right)?;
        return Ok(ConditionExpr::NotEquals(left_operand, right_operand));
    }

    if trimmed.starts_with('!') {
        let inner = parse_condition_expr(trimmed[1..].trim())?;
        return Ok(ConditionExpr::Not(Box::new(inner)));
    }

    if trimmed.eq("true") {
        return Ok(ConditionExpr::Literal(true));
    }
    if trimmed.eq("false") {
        return Ok(ConditionExpr::Literal(false));
    }
    if is_identifier(trimmed) {
        return Ok(ConditionExpr::Variable(trimmed.to_string()));
    }

    if let Ok(literal) = parse_literal(trimmed) {
        if let LiteralValue::Boolean(value) = literal {
            return Ok(ConditionExpr::Literal(value));
        }
    }

    Err(ParseError::InvalidSyntax(trimmed.to_string()))
}

fn parse_condition_operand(value: &str) -> Result<ConditionOperand, ParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ParseError::InvalidSyntax(value.to_string()));
    }

    if is_identifier(trimmed) {
        Ok(ConditionOperand::Variable(trimmed.to_string()))
    } else {
        let literal = parse_literal(trimmed)?;
        Ok(ConditionOperand::Literal(literal))
    }
}

fn parse_loop_iterable(value: &str) -> Result<LoopIterable, ParseError> {
    if is_identifier(value) {
        Ok(LoopIterable::Variable(value.to_string()))
    } else {
        let literal = parse_literal(value)?;
        Ok(LoopIterable::Literal(literal))
    }
}

fn parse_scan_header(line: &str) -> Result<(&str, &str), ParseError> {
    let cleaned = line.trim_end_matches('{').trim();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.len() < 3 || tokens[0] != "scan" {
        return Err(ParseError::InvalidSyntax(line.to_string()));
    }

    if tokens.len() >= 4 && tokens[2] == "using" {
        return Ok((tokens[1], tokens[3]));
    }

    if tokens.len() >= 3 {
        return Ok((tokens[1], tokens[2]));
    }

    Err(ParseError::InvalidSyntax(line.to_string()))
}

fn parse_report_header(line: &str) -> Result<&str, ParseError> {
    let cleaned = line.trim_end_matches('{').trim();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.len() < 2 || tokens[0] != "report" {
        return Err(ParseError::InvalidSyntax(line.to_string()));
    }
    Ok(tokens[1])
}

fn split_header_body(line: &str) -> Result<(&str, Option<&str>), ParseError> {
    if let Some(pos) = line.find('{') {
        let header = line[..pos].trim();
        let body = line[pos + 1..].trim();
        if body.is_empty() {
            Ok((header, None))
        } else {
            Ok((header, Some(body)))
        }
    } else {
        Err(ParseError::InvalidSyntax(line.to_string()))
    }
}

fn parse_properties_segment(
    segment: &str,
    properties: &mut BTreeMap<String, String>,
) -> Result<(), ParseError> {
    for entry in segment.split(';') {
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let key = parts
            .next()
            .ok_or_else(|| ParseError::InvalidSyntax(trimmed.to_string()))?;
        let value = parts
            .next()
            .ok_or_else(|| ParseError::MissingValue("asset_group value"))?;
        let parsed_value = parse_quoted(value)?;
        properties.insert(key.to_string(), parsed_value);
    }
    Ok(())
}

fn parse_quoted(value: &str) -> Result<String, ParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if let Some(stripped) = trimmed.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        return Ok(stripped.to_string());
    }

    if let Some(stripped) = trimmed
        .strip_prefix('\'')
        .and_then(|v| v.strip_suffix('\''))
    {
        return Ok(stripped.to_string());
    }

    if trimmed.starts_with('"') || trimmed.ends_with('"') {
        return Err(ParseError::InvalidSyntax(value.to_string()));
    }

    Ok(trimmed.to_string())
}

type PeekableLines<'a, I> = std::iter::Peekable<I>;

fn next_non_empty<'a, I>(lines: &mut PeekableLines<'a, I>) -> Option<(usize, &'a str)>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    while let Some((idx, line)) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        if !line.trim().is_empty() {
            return Some((idx, line));
        }
    }
    None
}

pub fn parse_literal_expression(value: &str) -> Result<LiteralValue, ParseError> {
    parse_literal(value)
}

fn parse_literal(value: &str) -> Result<LiteralValue, ParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(LiteralValue::String(String::new()));
    }

    if trimmed.starts_with('[') {
        return parse_array_literal(trimmed);
    }
    if trimmed.starts_with('{') {
        return parse_object_literal(trimmed);
    }
    if trimmed.eq("true") {
        return Ok(LiteralValue::Boolean(true));
    }
    if trimmed.eq("false") {
        return Ok(LiteralValue::Boolean(false));
    }
    if let Some(num) = parse_number_literal(trimmed) {
        return Ok(LiteralValue::Number(num));
    }
    if let Some(stripped) = trimmed
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
        })
    {
        return Ok(LiteralValue::String(stripped.to_string()));
    }

    Ok(LiteralValue::String(trimmed.to_string()))
}

fn parse_array_literal(value: &str) -> Result<LiteralValue, ParseError> {
    if !value.ends_with(']') {
        return Err(ParseError::InvalidSyntax(value.to_string()));
    }
    let inner = &value[1..value.len() - 1];
    let mut items = Vec::new();
    for item in split_top_level(inner, ',')? {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        items.push(parse_literal(trimmed)?);
    }
    Ok(LiteralValue::Array(items))
}

fn parse_object_literal(value: &str) -> Result<LiteralValue, ParseError> {
    if !value.ends_with('}') {
        return Err(ParseError::InvalidSyntax(value.to_string()));
    }
    let inner = &value[1..value.len() - 1];
    let mut map = BTreeMap::new();
    for entry in split_top_level(inner, ',')? {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = split_key_value(trimmed)?;
        let key = parse_object_key(raw_key)?;
        let value = parse_literal(raw_value)?;
        map.insert(key, value);
    }
    Ok(LiteralValue::Object(map))
}

fn parse_number_literal(value: &str) -> Option<f64> {
    if value
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
    {
        if let Ok(number) = value.parse::<f64>() {
            return Some(number);
        }
    }
    None
}

fn find_operator(input: &str, operator: &str) -> Option<usize> {
    if operator.is_empty() {
        return None;
    }
    let chars: Vec<char> = input.chars().collect();
    let op_chars: Vec<char> = operator.chars().collect();
    let mut depth = 0i32;
    let mut in_quote: Option<char> = None;
    let mut idx = 0usize;
    while idx + op_chars.len() <= chars.len() {
        let current = chars[idx];
        if let Some(q) = in_quote {
            if current == '\\' {
                idx += 1;
            } else if current == q {
                in_quote = None;
            }
            idx += 1;
            continue;
        }
        match current {
            '"' | '\'' => {
                in_quote = Some(current);
                idx += 1;
                continue;
            }
            '[' | '{' | '(' => {
                depth += 1;
                idx += 1;
                continue;
            }
            ']' | '}' | ')' => {
                depth -= 1;
                idx += 1;
                continue;
            }
            _ => {}
        }
        if depth == 0 {
            let mut matches = true;
            for (offset, ch) in op_chars.iter().enumerate() {
                if chars[idx + offset] != *ch {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Some(idx);
            }
        }
        idx += 1;
    }
    None
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn split_top_level(input: &str, delimiter: char) -> Result<Vec<&str>, ParseError> {
    let mut items = Vec::new();
    let mut depth = 0i32;
    let mut in_quote: Option<char> = None;
    let mut start = 0usize;
    let chars: Vec<char> = input.chars().collect();
    let mut idx = 0usize;
    while idx < chars.len() {
        let c = chars[idx];
        if let Some(q) = in_quote {
            if c == '\\' {
                idx += 1;
            } else if c == q {
                in_quote = None;
            }
        } else {
            match c {
                '"' | '\'' => in_quote = Some(c),
                '[' | '{' => depth += 1,
                ']' | '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(ParseError::InvalidSyntax(input.to_string()));
                    }
                }
                _ if c == delimiter && depth == 0 => {
                    items.push(input[start..idx].trim());
                    start = idx + 1;
                }
                _ => {}
            }
        }
        idx += 1;
    }
    if depth != 0 || in_quote.is_some() {
        return Err(ParseError::InvalidSyntax(input.to_string()));
    }
    if start < input.len() {
        items.push(input[start..].trim());
    }
    Ok(items.into_iter().filter(|s| !s.is_empty()).collect())
}

fn split_key_value(entry: &str) -> Result<(&str, &str), ParseError> {
    let mut depth = 0i32;
    let mut in_quote: Option<char> = None;
    let chars: Vec<char> = entry.chars().collect();
    for (idx, c) in chars.iter().enumerate() {
        if let Some(q) = in_quote {
            if *c == '\\' {
                continue;
            }
            if *c == q {
                in_quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => in_quote = Some(*c),
            '[' | '{' => depth += 1,
            ']' | '}' => depth -= 1,
            ':' if depth == 0 => {
                let key = entry[..idx].trim();
                let value = entry[idx + 1..].trim();
                return Ok((key, value));
            }
            _ => {}
        }
    }
    Err(ParseError::InvalidSyntax(entry.to_string()))
}

fn parse_object_key(raw: &str) -> Result<String, ParseError> {
    if let Some(stripped) = raw.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        return Ok(stripped.to_string());
    }
    if let Some(stripped) = raw.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        return Ok(stripped.to_string());
    }
    if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ParseError::InvalidSyntax(raw.to_string()));
    }
    Ok(raw.to_string())
}

impl LiteralValue {
    pub fn to_json(&self) -> JsonValue {
        match self {
            LiteralValue::String(s) => JsonValue::String(s.clone()),
            LiteralValue::Number(n) => JsonValue::from(*n),
            LiteralValue::Boolean(b) => JsonValue::Bool(*b),
            LiteralValue::Array(items) => {
                JsonValue::Array(items.iter().map(|v| v.to_json()).collect())
            }
            LiteralValue::Object(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    obj.insert(k.clone(), v.to_json());
                }
                JsonValue::Object(obj)
            }
        }
    }

    pub fn display(&self) -> String {
        match self {
            LiteralValue::String(s) => s.clone(),
            LiteralValue::Number(n) => {
                if (n.fract() - 0.0).abs() < f64::EPSILON {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            }
            LiteralValue::Boolean(b) => b.to_string(),
            LiteralValue::Array(_) | LiteralValue::Object(_) => self.to_json().to_string(),
        }
    }
}

impl fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl fmt::Display for ConditionExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConditionExpr::Literal(value) => write!(f, "{value}"),
            ConditionExpr::Variable(name) => write!(f, "{name}"),
            ConditionExpr::Not(inner) => write!(f, "!{}", inner),
            ConditionExpr::Equals(left, right) => write!(f, "{} == {}", left, right),
            ConditionExpr::NotEquals(left, right) => write!(f, "{} != {}", left, right),
        }
    }
}

impl fmt::Display for ConditionOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConditionOperand::Variable(name) => write!(f, "{name}"),
            ConditionOperand::Literal(value) => write!(f, "{}", value.display()),
        }
    }
}

impl fmt::Display for LoopIterable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoopIterable::Variable(name) => write!(f, "{name}"),
            LoopIterable::Literal(value) => write!(f, "{}", value.display()),
        }
    }
}

impl Scenario {
    pub fn summary(&self) -> ScenarioSummary {
        let import_list: BTreeSet<String> = self.imports.iter().cloned().collect();
        let mut accumulator = SummaryAccumulator::default();
        collect_summary_steps(&self.steps, &mut accumulator);
        ScenarioSummary {
            total_steps: accumulator.total_steps,
            imports: import_list.into_iter().collect(),
            variables: accumulator.variables,
            asset_groups: accumulator.asset_groups,
            scans: accumulator.scans,
            scripts: accumulator.scripts,
            reports: accumulator.reports,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSummary {
    pub total_steps: usize,
    pub imports: Vec<String>,
    pub variables: Vec<VariableSummary>,
    pub asset_groups: Vec<AssetGroupSummary>,
    pub scans: Vec<ScanSummary>,
    pub scripts: Vec<ScriptSummary>,
    pub reports: Vec<ReportSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGroupSummary {
    pub name: String,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub name: String,
    pub tool: String,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptSummary {
    pub name: String,
    pub run: String,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub name: String,
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableSummary {
    pub name: String,
    pub value: LiteralValue,
}

#[derive(Default)]
struct SummaryAccumulator {
    total_steps: usize,
    variables: Vec<VariableSummary>,
    asset_groups: Vec<AssetGroupSummary>,
    scans: Vec<ScanSummary>,
    scripts: Vec<ScriptSummary>,
    reports: Vec<ReportSummary>,
}

fn collect_summary_steps(steps: &[Step], acc: &mut SummaryAccumulator) {
    for step in steps {
        acc.total_steps += 1;
        match step {
            Step::Import(_) => {}
            Step::Variable(var) => acc.variables.push(VariableSummary {
                name: var.name.clone(),
                value: var.value.clone(),
            }),
            Step::AssetGroup(group) => acc.asset_groups.push(AssetGroupSummary {
                name: group.name.clone(),
                properties: group.properties.clone(),
            }),
            Step::Scan(scan) => acc.scans.push(ScanSummary {
                name: scan.name.clone(),
                tool: scan.tool.clone(),
                output: scan.output.clone(),
            }),
            Step::Script(script) => {
                if let Some(run) = script.params.get("run").cloned() {
                    acc.scripts.push(ScriptSummary {
                        name: script.name.clone(),
                        run,
                        output: script.output.clone(),
                    });
                }
            }
            Step::Report(report) => acc.reports.push(ReportSummary {
                name: report.name.clone(),
                includes: report.includes.clone(),
            }),
            Step::Conditional(block) => {
                collect_summary_steps(&block.then_steps, acc);
                collect_summary_steps(&block.else_steps, acc);
            }
            Step::Loop(loop_step) => {
                collect_summary_steps(&loop_step.body, acc);
            }
        }
    }
}

impl fmt::Display for ScenarioSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Steps: {}", self.total_steps)?;
        if !self.imports.is_empty() {
            writeln!(f, "Imports:")?;
            for import in &self.imports {
                writeln!(f, "  - {}", import)?;
            }
        }
        if !self.variables.is_empty() {
            writeln!(f, "Variables:")?;
            for var in &self.variables {
                writeln!(f, "  - {} = {}", var.name, var.value)?;
            }
        }
        if !self.asset_groups.is_empty() {
            writeln!(f, "Asset groups:")?;
            for group in &self.asset_groups {
                let props: Vec<String> = group
                    .properties
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect();
                writeln!(f, "  - {} ({})", group.name, props.join(", "))?;
            }
        }
        if !self.scans.is_empty() {
            writeln!(f, "Scans:")?;
            for scan in &self.scans {
                let output = scan.output.as_deref().unwrap_or("<none>");
                writeln!(f, "  - {} via {} -> {}", scan.name, scan.tool, output)?;
            }
        }
        if !self.scripts.is_empty() {
            writeln!(f, "Scripts:")?;
            for script in &self.scripts {
                let output = script.output.as_deref().unwrap_or("<none>");
                writeln!(f, "  - {} run {} -> {}", script.name, script.run, output)?;
            }
        }
        if !self.reports.is_empty() {
            writeln!(f, "Reports:")?;
            for report in &self.reports {
                writeln!(
                    f,
                    "  - {} (includes: {})",
                    report.name,
                    report.includes.join(", ")
                )?;
            }
        }
        Ok(())
    }
}
