use axion_core::{
    parse_scenario, ExecutionOutcome, Executor, Scenario, ScenarioSummary, Step, StoredArtifact,
};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::collections::HashSet;
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
    },
    /// Parse a scenario file and perform a dry-run (plan + placeholder execution)
    Run {
        /// Path to the Axion DSL scenario file
        input: PathBuf,
        /// Output JSON instead of a human-readable summary
        #[arg(long)]
        json: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = AxionCli::parse();

    match cli.command {
        Command::Plan { input, json } => {
            let scenario = load_scenario(&input)?;
            let summary = scenario.summary();
            output_plan(summary, json)?;
        }
        Command::Run { input, json } => {
            let scenario = load_scenario(&input)?;
            let summary = scenario.summary();
            let executor = Executor::new();
            let outcome = executor.execute(&scenario);
            output_run(summary, outcome, json)?;
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

fn output_plan(summary: ScenarioSummary, json: bool) -> anyhow::Result<()> {
    if json {
        let payload = serde_json::to_string_pretty(&summary)?;
        println!("{payload}");
    } else {
        println!("{summary}");
    }
    Ok(())
}

fn output_run(
    summary: ScenarioSummary,
    outcome: ExecutionOutcome,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        let payload = json!({
            "summary": summary,
            "execution": outcome.report,
            "artifacts": outcome.artifacts,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("{summary}\n");
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
