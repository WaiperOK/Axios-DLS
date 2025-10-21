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
pub struct VariableDecl {
    pub name: String,
    pub value: LiteralValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        if trimmed.starts_with("import ") {
            let path = parse_import(trimmed)?;
            imports.push(path.clone());
            steps.push(Step::Import(ImportStep { path }));
        } else if trimmed.starts_with("asset_group ") {
            let step = parse_asset_group(trimmed, &mut lines)?;
            steps.push(Step::AssetGroup(step));
        } else if trimmed.starts_with("scan ") {
            let step = parse_scan(trimmed, &mut lines)?;
            steps.push(Step::Scan(step));
        } else if trimmed.starts_with("let ") {
            let step = parse_variable(trimmed)?;
            steps.push(Step::Variable(step));
        } else if trimmed.starts_with("script ") {
            let step = parse_script(trimmed, &mut lines)?;
            steps.push(Step::Script(step));
        } else if trimmed.starts_with("report ") {
            let step = parse_report(trimmed, &mut lines)?;
            steps.push(Step::Report(step));
        } else {
            return Err(ParseError::InvalidDirective(trimmed.to_string()));
        }
    }

    Ok(Scenario { steps, imports })
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

impl Scenario {
    pub fn summary(&self) -> ScenarioSummary {
        let import_list: BTreeSet<String> = self.imports.iter().cloned().collect();
        ScenarioSummary {
            total_steps: self.steps.len(),
            imports: import_list.into_iter().collect(),
            variables: self
                .steps
                .iter()
                .filter_map(|step| match step {
                    Step::Variable(var) => Some(VariableSummary {
                        name: var.name.clone(),
                        value: var.value.clone(),
                    }),
                    _ => None,
                })
                .collect(),
            asset_groups: self
                .steps
                .iter()
                .filter_map(|step| match step {
                    Step::AssetGroup(group) => Some(AssetGroupSummary {
                        name: group.name.clone(),
                        properties: group.properties.clone(),
                    }),
                    _ => None,
                })
                .collect(),
            scans: self
                .steps
                .iter()
                .filter_map(|step| match step {
                    Step::Scan(scan) => Some(ScanSummary {
                        name: scan.name.clone(),
                        tool: scan.tool.clone(),
                        output: scan.output.clone(),
                    }),
                    _ => None,
                })
                .collect(),
            scripts: self
                .steps
                .iter()
                .filter_map(|step| match step {
                    Step::Script(script) => {
                        script.params.get("run").cloned().map(|run| ScriptSummary {
                            name: script.name.clone(),
                            run,
                            output: script.output.clone(),
                        })
                    }
                    _ => None,
                })
                .collect(),
            reports: self
                .steps
                .iter()
                .filter_map(|step| match step {
                    Step::Report(report) => Some(ReportSummary {
                        name: report.name.clone(),
                        includes: report.includes.clone(),
                    }),
                    _ => None,
                })
                .collect(),
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
