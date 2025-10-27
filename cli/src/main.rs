use anyhow::anyhow;
use axion_core::{
    builtin_tool_schema_bundle, parse_scenario, validate_scenario, Diagnostic, DiagnosticLevel,
    ExecutionOutcome, Executor, LiteralValue, Scenario, ScenarioSummary, Step, StoredArtifact,
    ToolSchema,
};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(author, version, about = "Axion DSL command line interface (MVP)")]
struct AxionCli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse a scenario file and print the execution summary
    Plan {
        /// Path to the Axion DSL scenario file
        input: PathBuf,
        /// Output JSON instead of a human-readable summary
        #[arg(long)]
        json: bool,
        /// Override a variable (format: key=value). Repeat for multiple overrides.
        #[arg(long = "var", value_parser = parse_key_val, value_name = "KEY=VALUE", action = ArgAction::Append)]
        vars: Vec<(String, String)>,
        /// Override a secret (format: key=value). Repeat for multiple overrides.
        #[arg(long = "secret", value_parser = parse_key_val, value_name = "KEY=VALUE", action = ArgAction::Append)]
        secrets: Vec<(String, String)>,
    },
    /// Parse a scenario file and perform a dry-run (plan + placeholder execution)
    Run {
        /// Path to the Axion DSL scenario file
        input: PathBuf,
        /// Output JSON instead of a human-readable summary
        #[arg(long)]
        json: bool,
        /// Override a variable (format: key=value). Repeat for multiple overrides.
        #[arg(long = "var", value_parser = parse_key_val, value_name = "KEY=VALUE", action = ArgAction::Append)]
        vars: Vec<(String, String)>,
        /// Override a secret (format: key=value). Repeat for multiple overrides.
        #[arg(long = "secret", value_parser = parse_key_val, value_name = "KEY=VALUE", action = ArgAction::Append)]
        secrets: Vec<(String, String)>,
    },
    /// Export builtin tool schemas
    Schema {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
        /// Output format
        #[arg(long, default_value_t = SchemaFormat::Json)]
        format: SchemaFormat,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = AxionCli::parse();

    match cli.command {
        Command::Plan {
            input,
            json,
            vars,
            secrets,
        } => {
            let scenario = load_scenario(&input)?;
            let overrides = parse_overrides(vars)?;
            let secret_overrides = parse_secret_overrides(secrets)?;
            let diagnostics = validate_scenario(&scenario);
            let summary = scenario.summary();
            let has_errors =
                output_plan(summary, &diagnostics, json, &overrides, &secret_overrides)?;
            if has_errors {
                anyhow::bail!("validation failed");
            }
        }
        Command::Run {
            input,
            json,
            vars,
            secrets,
        } => {
            let scenario = load_scenario(&input)?;
            let overrides = parse_overrides(vars)?;
            let secret_overrides = parse_secret_overrides(secrets)?;
            let summary = scenario.summary();
            let executor = Executor::new();
            let outcome = executor.execute_with_vars(&scenario, &overrides, &secret_overrides);
            output_run(summary, outcome, json, &overrides, &secret_overrides)?;
        }
        Command::Schema { tool, format } => {
            output_schema(tool, format)?;
        }
    }

    Ok(())
}

fn load_scenario(path: &PathBuf) -> anyhow::Result<Scenario> {
    let mut visited = HashSet::new();
    load_scenario_recursive(path, &mut visited)
}

fn load_scenario_recursive(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
) -> anyhow::Result<Scenario> {
    let canonical = fs::canonicalize(path)?;
    if !visited.insert(canonical.clone()) {
        return Ok(Scenario::default());
    }

    let content = fs::read_to_string(&canonical)?;
    let parsed = parse_scenario(&content)?;
    let base_dir = canonical.parent().unwrap_or_else(|| Path::new(""));

    let mut steps = Vec::new();
    let mut imports = Vec::new();

    for step in parsed.steps {
        match step {
            Step::Import(import_step) => {
                let import_path = base_dir.join(&import_step.path);
                let import_display = import_path.to_string_lossy().to_string();
                let imported = load_scenario_recursive(&import_path, visited)?;
                imports.push(import_display);
                steps.extend(imported.steps);
                imports.extend(imported.imports);
            }
            other => steps.push(other),
        }
    }

    Ok(Scenario { steps, imports })
}

fn output_plan(
    summary: ScenarioSummary,
    diagnostics: &[Diagnostic],
    json: bool,
    overrides: &HashMap<String, LiteralValue>,
    secret_overrides: &HashMap<String, String>,
) -> anyhow::Result<bool> {
    let has_errors = diagnostics.iter().any(Diagnostic::is_error);

    if json {
        let masked_secrets: HashMap<String, String> = secret_overrides
            .keys()
            .cloned()
            .map(|key| (key, "***".to_string()))
            .collect();
        let payload = json!({
            "summary": summary,
            "diagnostics": diagnostics,
            "overrides": overrides,
            "secrets": masked_secrets,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        if !diagnostics.is_empty() {
            print_diagnostics(diagnostics);
        }

        println!("{summary}");
        if !overrides.is_empty() {
            println!("\nOverrides (--var):");
            for (key, value) in overrides {
                println!("  - {} = {}", key, value);
            }
        }
        if !secret_overrides.is_empty() {
            println!("\nSecrets (--secret):");
            let mut keys: Vec<&String> = secret_overrides.keys().collect();
            keys.sort();
            for key in keys {
                println!("  - {} = ***", key);
            }
        }
    }

    Ok(has_errors)
}

fn output_run(
    summary: ScenarioSummary,
    outcome: ExecutionOutcome,
    json: bool,
    overrides: &HashMap<String, LiteralValue>,
    secret_overrides: &HashMap<String, String>,
) -> anyhow::Result<()> {
    if json {
        let masked_secrets: HashMap<String, String> = secret_overrides
            .keys()
            .cloned()
            .map(|key| (key, "***".to_string()))
            .collect();
        let payload = json!({
            "summary": summary,
            "execution": outcome.report,
            "artifacts": outcome.artifacts,
            "overrides": overrides,
            "secrets": masked_secrets,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("{summary}\n");
        if !overrides.is_empty() {
            println!("Overrides (--var):");
            for (key, value) in overrides {
                println!("  - {} = {}", key, value);
            }
            println!();
        }
        if !secret_overrides.is_empty() {
            println!("Secrets (--secret):");
            let mut keys: Vec<&String> = secret_overrides.keys().collect();
            keys.sort();
            for key in keys {
                println!("  - {} = ***", key);
            }
            println!();
        }
        println!("{}", outcome.report);
        if outcome.report.has_failures() {
            println!("\n[warn] some steps failed");
        }
        if !outcome.artifacts.is_empty() {
            println!("\nArtifacts:");
            for StoredArtifact {
                name, kind, path, ..
            } in &outcome.artifacts
            {
                match path {
                    Some(p) => println!("  - {} ({:?}) -> {}", name, kind, p),
                    None => println!("  - {} ({:?})", name, kind),
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SchemaFormat {
    Json,
    Yaml,
}

impl std::fmt::Display for SchemaFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SchemaFormat::Json => "json",
            SchemaFormat::Yaml => "yaml",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SchemaResponse {
    version: String,
    generated_at: String,
    tools: Vec<ToolSchema>,
}

fn output_schema(tool: Option<String>, format: SchemaFormat) -> anyhow::Result<()> {
    let bundle = builtin_tool_schema_bundle();
    let mut tools = bundle.tools;

    if let Some(filter) = tool {
        tools.retain(|schema| schema.name == filter);
        if tools.is_empty() {
            anyhow::bail!("unknown tool '{filter}'");
        }
    }

    let response = SchemaResponse {
        version: bundle.version,
        generated_at: bundle.generated_at,
        tools,
    };

    match format {
        SchemaFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        SchemaFormat::Yaml => {
            let yaml = serde_yaml::to_string(&response)?;
            print!("{yaml}");
        }
    }

    Ok(())
}

fn print_diagnostics(diagnostics: &[Diagnostic]) {
    if diagnostics.is_empty() {
        return;
    }

    println!("Diagnostics:");
    for diagnostic in diagnostics {
        let level = match diagnostic.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warn",
        };
        match &diagnostic.location {
            Some(location) => println!("  - [{level}] {location}: {}", diagnostic.message),
            None => println!("  - [{level}] {}", diagnostic.message),
        }
    }
    println!();
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 || parts[0].trim().is_empty() {
        return Err(format!("expected KEY=VALUE, got '{s}'"));
    }
    Ok((parts[0].trim().to_string(), parts[1].to_string()))
}

fn parse_overrides(vars: Vec<(String, String)>) -> anyhow::Result<HashMap<String, LiteralValue>> {
    let mut map = HashMap::new();
    for (key, raw) in vars {
        let literal = axion_core::parse_literal_expression(&raw)
            .map_err(|err| anyhow!("invalid override {key}: {err}"))?;
        map.insert(key, literal);
    }
    Ok(map)
}

fn parse_secret_overrides(
    secrets: Vec<(String, String)>,
) -> anyhow::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for (key, value) in secrets {
        map.insert(key, value);
    }
    Ok(map)
}
