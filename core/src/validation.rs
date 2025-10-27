use crate::scenario::{
    LiteralValue, LoopIterable, LoopStep, ReportStep, ScanStep, Scenario, ScriptStep, SecretSource,
    SecretStep, Step,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub location: Option<String>,
    pub message: String,
}

impl Diagnostic {
    fn error(location: Option<String>, message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            location,
            message: message.into(),
        }
    }

    fn warning(location: Option<String>, message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            location,
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self.level, DiagnosticLevel::Error)
    }
}

pub fn validate_scenario(scenario: &Scenario) -> Vec<Diagnostic> {
    let mut ctx = ValidationContext::new();
    validate_steps(&scenario.steps, &mut ctx);
    ctx.finish()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub kind: Option<String>,
    pub description: Option<String>,
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub allow_additional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchemaBundle {
    pub version: String,
    pub generated_at: String,
    pub tools: Vec<ToolSchema>,
}

pub fn builtin_tool_schemas() -> Vec<ToolSchema> {
    BUILTIN_SCHEMAS
        .iter()
        .map(|schema| ToolSchema {
            name: schema.name.to_string(),
            kind: Some(schema.kind.to_string()),
            description: Some(schema.description.to_string()),
            required: schema.required.iter().map(|s| (*s).to_string()).collect(),
            optional: schema.optional.iter().map(|s| (*s).to_string()).collect(),
            allow_additional: schema.allow_additional,
        })
        .collect()
}

pub fn builtin_tool_schema_bundle() -> ToolSchemaBundle {
    ToolSchemaBundle {
        version: SCHEMA_VERSION.to_string(),
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string()),
        tools: builtin_tool_schemas(),
    }
}

struct ValidationContext {
    stack: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

impl ValidationContext {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn push(&mut self, label: String) {
        self.stack.push(label);
    }

    fn pop(&mut self) {
        self.stack.pop();
    }

    fn location(&self) -> Option<String> {
        if self.stack.is_empty() {
            None
        } else {
            Some(self.stack.join(" > "))
        }
    }

    fn error(&mut self, message: impl Into<String>) {
        let diag = Diagnostic::error(self.location(), message);
        self.diagnostics.push(diag);
    }

    fn warning(&mut self, message: impl Into<String>) {
        let diag = Diagnostic::warning(self.location(), message);
        self.diagnostics.push(diag);
    }

    fn finish(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

fn validate_steps(steps: &[Step], ctx: &mut ValidationContext) {
    for step in steps {
        match step {
            Step::Import(_) => {}
            Step::Variable(_) => {}
            Step::Secret(secret) => {
                ctx.push(format!("secret {}", secret.name));
                validate_secret(secret, ctx);
                ctx.pop();
            }
            Step::AssetGroup(group) => {
                ctx.push(format!("asset_group {}", group.name));
                // Asset group properties are free-form.
                ctx.pop();
            }
            Step::Scan(scan) => {
                ctx.push(format!("scan {}", scan.name));
                validate_scan(scan, ctx);
                ctx.pop();
            }
            Step::Script(script) => {
                ctx.push(format!("script {}", script.name));
                validate_script(script, ctx);
                ctx.pop();
            }
            Step::Report(report) => {
                ctx.push(format!("report {}", report.name));
                validate_report(report, ctx);
                ctx.pop();
            }
            Step::Conditional(block) => {
                ctx.push(format!("if {}", block.condition));
                validate_steps(&block.then_steps, ctx);
                ctx.pop();

                if !block.else_steps.is_empty() {
                    ctx.push("else".to_string());
                    validate_steps(&block.else_steps, ctx);
                    ctx.pop();
                }
            }
            Step::Loop(loop_step) => {
                ctx.push(format!("for {}", loop_step.iterator));
                validate_loop(loop_step, ctx);
                validate_steps(&loop_step.body, ctx);
                ctx.pop();
            }
        }
    }
}

fn validate_scan(scan: &ScanStep, ctx: &mut ValidationContext) {
    let params = &scan.params;
    if let Some(schema) = lookup_schema(scan.tool.as_str()) {
        validate_with_schema(&scan.tool, params, schema, ctx);
    } else {
        if let Some(value) = params.get("target") {
            if value.trim().is_empty() {
                ctx.error("parameter 'target' cannot be empty");
            }
        } else {
            ctx.warning("parameter 'target' is not set; scans may rely on tool defaults");
        }
        enforce_known(params, &["target", "flags", "args", "cwd"], ctx, &scan.tool);
    }
}

fn validate_secret(secret: &SecretStep, ctx: &mut ValidationContext) {
    if secret.name.trim().is_empty() {
        ctx.error("secret name cannot be empty");
    }

    match &secret.source {
        SecretSource::Env { mappings } => {
            if mappings.is_empty() {
                ctx.error("env secret requires at least one mapping");
            }
            for (alias, env_key) in mappings {
                if alias.trim().is_empty() {
                    ctx.error("env secret mapping name cannot be empty");
                }
                if env_key.trim().is_empty() {
                    ctx.error(format!(
                        "env secret mapping '{}' references an empty variable name",
                        alias
                    ));
                }
            }
        }
        SecretSource::File { path } => {
            if path.trim().is_empty() {
                ctx.error("file secret path cannot be empty");
            }
        }
        SecretSource::Vault { path, .. } => {
            if path.trim().is_empty() {
                ctx.error("vault secret requires a path");
            }
            ctx.warning(
                "vault provider is not implemented yet; this step will be skipped at runtime",
            );
        }
    }
}

fn validate_script(script: &ScriptStep, ctx: &mut ValidationContext) {
    let params = &script.params;
    if let Some(schema) = lookup_schema("script") {
        validate_with_schema("script", params, schema, ctx);
    }
}

fn validate_report(report: &ReportStep, ctx: &mut ValidationContext) {
    if report.includes.is_empty() {
        ctx.warning("report does not include any artifacts");
    }
}

fn validate_loop(loop_step: &LoopStep, ctx: &mut ValidationContext) {
    if let LoopIterable::Literal(literal) = &loop_step.iterable {
        match literal {
            LiteralValue::Array(_) | LiteralValue::String(_) => {}
            other => ctx.error(format!(
                "loop iterable must be an array or string literal, found {}",
                other.display()
            )),
        }
    }
}

fn check_required(
    params: &BTreeMap<String, String>,
    required: &[&str],
    ctx: &mut ValidationContext,
    tool: &str,
) {
    for key in required {
        match params.get(*key) {
            Some(value) if value.trim().is_empty() => {
                ctx.error(format!(
                    "parameter '{}' for tool '{}' cannot be empty",
                    key, tool
                ));
            }
            Some(_) => {}
            None => ctx.error(format!(
                "missing required parameter '{}' for tool '{}'",
                key, tool
            )),
        }
    }
}

fn enforce_known(
    params: &BTreeMap<String, String>,
    allowed: &[&str],
    ctx: &mut ValidationContext,
    tool: &str,
) {
    let allowed: HashSet<&str> = allowed.iter().copied().collect();
    for key in params.keys() {
        if !allowed.contains(key.as_str()) {
            ctx.warning(format!(
                "unknown parameter '{}' for tool '{}'; it will be ignored",
                key, tool
            ));
        }
    }
}
struct ToolSchemaDef {
    name: &'static str,
    kind: &'static str,
    description: &'static str,
    required: &'static [&'static str],
    optional: &'static [&'static str],
    allow_additional: bool,
}

impl ToolSchemaDef {
    fn allows(&self, key: &str) -> bool {
        self.required
            .iter()
            .chain(self.optional.iter())
            .any(|candidate| *candidate == key)
    }
}

const BUILTIN_SCHEMAS: &[ToolSchemaDef] = &[
    ToolSchemaDef {
        name: "nmap",
        kind: "scan",
        description: "Nmap TCP/UDP scanner",
        required: &["target"],
        optional: &["flags"],
        allow_additional: false,
    },
    ToolSchemaDef {
        name: "gobuster",
        kind: "scan",
        description: "Gobuster content discovery",
        required: &["target", "args"],
        optional: &["flags", "wordlist", "mode"],
        allow_additional: false,
    },
    ToolSchemaDef {
        name: "script",
        kind: "script",
        description: "Generic script execution",
        required: &["run"],
        optional: &["args", "cwd"],
        allow_additional: false,
    },
];

const SCHEMA_VERSION: &str = "1.0.0";

fn lookup_schema(tool: &str) -> Option<&'static ToolSchemaDef> {
    BUILTIN_SCHEMAS.iter().find(|schema| schema.name == tool)
}

fn validate_with_schema(
    tool: &str,
    params: &BTreeMap<String, String>,
    schema: &ToolSchemaDef,
    ctx: &mut ValidationContext,
) {
    check_required(params, schema.required, ctx, tool);

    if let Some(value) = params.get("args") {
        if tool == "gobuster" && value.trim().is_empty() {
            ctx.error("parameter 'args' cannot be empty for tool 'gobuster'");
        }
    }
    if let Some(value) = params.get("target") {
        if value.trim().is_empty() {
            ctx.error("parameter 'target' cannot be empty");
        }
    }
    if let Some(value) = params.get("run") {
        if tool == "script" && value.trim().is_empty() {
            ctx.error("parameter 'run' cannot be empty");
        }
    }

    if !schema.allow_additional {
        for key in params.keys() {
            if !schema.allows(key) {
                ctx.warning(format!(
                    "unknown parameter '{}' for tool '{}'; it will be ignored",
                    key, tool
                ));
            }
        }
    }
}
