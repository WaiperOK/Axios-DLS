use crate::scenario::{
    LiteralValue, LoopIterable, LoopStep, ReportStep, ScanStep, Scenario, ScriptStep, Step,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

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
    match scan.tool.as_str() {
        "nmap" => validate_with_schema("nmap", params, &NMAP_SCHEMA, ctx),
        "gobuster" => {
            validate_with_schema("gobuster", params, &GOBUSTER_SCHEMA, ctx);
            if let Some(args) = params.get("args") {
                if args.trim().is_empty() {
                    ctx.error("parameter 'args' cannot be empty for tool 'gobuster'");
                }
            }
        }
        tool => {
            if let Some(value) = params.get("target") {
                if value.trim().is_empty() {
                    ctx.error("parameter 'target' cannot be empty");
                }
            } else {
                ctx.warning("parameter 'target' is not set; scans may rely on tool defaults");
            }
            enforce_known(params, &["target", "flags", "args", "cwd"], ctx, tool);
        }
    }
}

fn validate_script(script: &ScriptStep, ctx: &mut ValidationContext) {
    let params = &script.params;
    match params.get("run") {
        Some(value) if value.trim().is_empty() => {
            ctx.error("parameter 'run' cannot be empty");
        }
        Some(_) => {}
        None => {
            // Parser already enforces this, but guard against future changes.
            ctx.error("missing required parameter 'run'");
        }
    }
    enforce_known(params, &["run", "args", "cwd"], ctx, "script");
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

#[derive(Clone, Copy)]
struct ToolSchema {
    required: &'static [&'static str],
    optional: &'static [&'static str],
    allow_additional: bool,
}

impl ToolSchema {
    fn allows(&self, key: &str) -> bool {
        self.required
            .iter()
            .chain(self.optional.iter())
            .any(|k| *k == key)
    }
}

const NMAP_SCHEMA: ToolSchema = ToolSchema {
    required: &["target"],
    optional: &["flags"],
    allow_additional: false,
};

const GOBUSTER_SCHEMA: ToolSchema = ToolSchema {
    required: &["target", "args"],
    optional: &["flags", "wordlist", "mode"],
    allow_additional: false,
};

fn validate_with_schema(
    tool: &str,
    params: &BTreeMap<String, String>,
    schema: &ToolSchema,
    ctx: &mut ValidationContext,
) {
    check_required(params, schema.required, ctx, tool);
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
