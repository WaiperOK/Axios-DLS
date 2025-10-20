use serde::{Deserialize, Serialize};
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
    pub value: String,
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

    let value = parse_quoted(value_part)?;

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
    pub value: String,
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
