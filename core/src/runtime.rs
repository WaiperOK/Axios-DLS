use crate::artifact::{
    ArtifactKind, Asset, AssetGroupArtifact, Finding, ReportArtifact, ScanArtifacts,
    ScriptArtifact, StoredArtifact, TableArtifact,
};
use crate::scenario::{
    AssetGroupStep, ConditionExpr, ConditionOperand, ConditionalStep, LiteralValue, LoopIterable,
    LoopStep, ReportStep, ScanStep, Scenario, ScriptStep, SecretSource, SecretStep, Step,
    VariableDecl,
};
use comfy_table::{presets::ASCII_FULL, Table};
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use time::OffsetDateTime;

#[derive(Debug, Default)]
pub struct Executor {
    artifacts_dir: PathBuf,
}

#[derive(Debug, Default)]
struct SecretStore {
    values: HashMap<String, String>,
    usage: HashSet<String>,
}

impl SecretStore {
    fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        self.values.entry(key).or_insert(value);
    }

    fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        self.values.insert(key, value);
    }

    fn resolve(&mut self, key: &str) -> Option<String> {
        if let Some(value) = self.values.get(key) {
            self.usage.insert(key.to_string());
            Some(value.clone())
        } else {
            None
        }
    }

    fn peek(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|value| value.as_str())
    }

    fn mask(&self, input: &str) -> String {
        let mut masked = input.to_string();
        for value in self.values.values() {
            if !value.is_empty() {
                masked = masked.replace(value, "***");
            }
        }
        masked
    }
}

impl Executor {
    pub fn new() -> Self {
        let artifacts_dir = PathBuf::from("artifacts");
        if let Err(err) = fs::create_dir_all(&artifacts_dir) {
            eprintln!("[warn] failed to create artifacts directory: {err}");
        }
        Self { artifacts_dir }
    }

    pub fn execute(&self, scenario: &Scenario) -> ExecutionOutcome {
        let empty_vars = HashMap::new();
        let empty_secrets = HashMap::new();
        self.execute_with_vars(scenario, &empty_vars, &empty_secrets)
    }

    pub fn execute_with_vars(
        &self,
        scenario: &Scenario,
        overrides: &HashMap<String, LiteralValue>,
        secret_overrides: &HashMap<String, String>,
    ) -> ExecutionOutcome {
        let mut store: HashMap<String, StoredArtifact> = HashMap::new();
        let mut variables: HashMap<String, LiteralValue> = overrides.clone();
        let mut report_steps = Vec::new();
        let mut secrets = SecretStore::default();

        for (key, value) in secret_overrides {
            secrets.set(key.clone(), value.clone());
        }

        self.execute_steps(
            &scenario.steps,
            overrides,
            &mut variables,
            &mut store,
            &mut report_steps,
            &mut secrets,
        );

        let artifacts = store.into_values().collect();

        ExecutionOutcome {
            report: ExecutionReport {
                steps: report_steps,
            },
            artifacts,
        }
    }

    fn execute_steps(
        &self,
        steps: &[Step],
        overrides: &HashMap<String, LiteralValue>,
        variables: &mut HashMap<String, LiteralValue>,
        store: &mut HashMap<String, StoredArtifact>,
        report: &mut Vec<StepExecution>,
        secrets: &mut SecretStore,
    ) {
        for step in steps {
            match step {
                Step::Import(_) => continue,
                Step::Variable(var) => {
                    let outcome = self.process_variable(var, overrides, variables, secrets);
                    self.record_outcome(report, store, secrets, outcome);
                }
                Step::AssetGroup(group) => {
                    let outcome = self.process_asset_group(group, variables, secrets);
                    self.record_outcome(report, store, secrets, outcome);
                }
                Step::Scan(scan) => {
                    let outcome = self.process_scan(scan, variables, secrets);
                    self.record_outcome(report, store, secrets, outcome);
                }
                Step::Script(script) => {
                    let outcome = self.process_script(script, variables, secrets);
                    self.record_outcome(report, store, secrets, outcome);
                }
                Step::Report(report_step) => {
                    let outcome = self.process_report(report_step, store, variables, secrets);
                    self.record_outcome(report, store, secrets, outcome);
                }
                Step::Secret(secret_step) => {
                    let outcome = self.process_secret(secret_step, secrets);
                    self.record_outcome(report, store, secrets, outcome);
                }
                Step::Conditional(block) => {
                    self.process_conditional(block, overrides, variables, store, report, secrets);
                }
                Step::Loop(loop_step) => {
                    self.process_loop(loop_step, overrides, variables, store, report, secrets);
                }
            }
        }
    }

    fn record_outcome(
        &self,
        report: &mut Vec<StepExecution>,
        store: &mut HashMap<String, StoredArtifact>,
        secrets: &SecretStore,
        mut outcome: StepOutcome,
    ) {
        if let Some(artifact) = outcome.artifact {
            store.insert(artifact.name.clone(), artifact);
        }
        if let Some(message) = outcome.execution.message.take() {
            outcome.execution.message = Some(secrets.mask(&message));
        }
        report.push(outcome.execution);
    }

    fn process_conditional(
        &self,
        block: &ConditionalStep,
        overrides: &HashMap<String, LiteralValue>,
        variables: &mut HashMap<String, LiteralValue>,
        store: &mut HashMap<String, StoredArtifact>,
        report: &mut Vec<StepExecution>,
        secrets: &mut SecretStore,
    ) {
        let condition_name = format!("if {}", block.condition);
        match evaluate_condition(&block.condition, variables, secrets) {
            Ok(result) => {
                let outcome = StepOutcome::from_execution(StepExecution::completed(
                    condition_name.clone(),
                    StepKind::Conditional,
                    Some(format!("condition evaluated to {result}")),
                ));
                self.record_outcome(report, store, secrets, outcome);

                let branch = if result {
                    &block.then_steps
                } else {
                    &block.else_steps
                };
                if !branch.is_empty() {
                    self.execute_steps(branch, overrides, variables, store, report, secrets);
                }
            }
            Err(err) => {
                let outcome = StepOutcome::from_execution(StepExecution::failed(
                    condition_name,
                    StepKind::Conditional,
                    Some(err),
                ));
                self.record_outcome(report, store, secrets, outcome);
            }
        }
    }

    fn process_loop(
        &self,
        loop_step: &LoopStep,
        overrides: &HashMap<String, LiteralValue>,
        variables: &mut HashMap<String, LiteralValue>,
        store: &mut HashMap<String, StoredArtifact>,
        report: &mut Vec<StepExecution>,
        secrets: &mut SecretStore,
    ) {
        let loop_name = format!("for {} in {}", loop_step.iterator, loop_step.iterable);
        match resolve_iterable(&loop_step.iterable, variables, secrets) {
            Ok(items) => {
                let previous = variables.get(&loop_step.iterator).cloned();
                let mut iterations = 0usize;
                for item in items {
                    variables.insert(loop_step.iterator.clone(), item);
                    iterations += 1;
                    self.execute_steps(
                        &loop_step.body,
                        overrides,
                        variables,
                        store,
                        report,
                        secrets,
                    );
                }
                match previous {
                    Some(value) => {
                        variables.insert(loop_step.iterator.clone(), value);
                    }
                    None => {
                        variables.remove(&loop_step.iterator);
                    }
                }
                let outcome = StepOutcome::from_execution(StepExecution::completed(
                    loop_name,
                    StepKind::Loop,
                    Some(format!("executed {iterations} iteration(s)")),
                ));
                self.record_outcome(report, store, secrets, outcome);
            }
            Err(err) => {
                let outcome = StepOutcome::from_execution(StepExecution::failed(
                    loop_name,
                    StepKind::Loop,
                    Some(err),
                ));
                self.record_outcome(report, store, secrets, outcome);
            }
        }
    }

    fn process_generic_scan(
        &self,
        scan: &ScanStep,
        params: BTreeMap<String, String>,
    ) -> StepOutcome {
        let mut cmd = Command::new(&scan.tool);
        let mut invocation = vec![scan.tool.clone()];

        if let Some(flags) = params.get("flags") {
            match shell_words::split(flags) {
                Ok(parts) => {
                    for part in parts {
                        cmd.arg(&part);
                        invocation.push(part);
                    }
                }
                Err(err) => {
                    return StepOutcome::from_execution(StepExecution::failed(
                        scan.name.clone(),
                        StepKind::Scan,
                        Some(format!("failed to parse flags: {err}")),
                    ));
                }
            }
        }

        if let Some(args) = params.get("args") {
            match shell_words::split(args) {
                Ok(parts) => {
                    for part in parts {
                        cmd.arg(&part);
                        invocation.push(part);
                    }
                }
                Err(err) => {
                    return StepOutcome::from_execution(StepExecution::failed(
                        scan.name.clone(),
                        StepKind::Scan,
                        Some(format!("failed to parse args: {err}")),
                    ));
                }
            }
        }

        if let Some(target) = params.get("target") {
            if !target.is_empty() {
                cmd.arg(target);
                invocation.push(target.clone());
            }
        }

        if let Some(cwd) = params.get("cwd") {
            if !cwd.is_empty() {
                cmd.current_dir(cwd);
            }
        }

        let started_at = OffsetDateTime::now_utc();
        let timer = Instant::now();

        match cmd.output() {
            Ok(output) => {
                let timestamp = started_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| "unknown".to_string());
                let duration_ms = timer.elapsed().as_millis();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();

                let label = scan
                    .output
                    .clone()
                    .unwrap_or_else(|| format!("scan_{}", scan.name));
                let artifact_data = json!({
                    "tool": scan.tool.clone(),
                    "params": params,
                    "invocation": invocation,
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code,
                    "started_at": timestamp,
                    "duration_ms": duration_ms,
                });

                let path = self
                    .write_artifact(&label, &artifact_data)
                    .map(|p| p.to_string_lossy().to_string());

                let message = format!(
                    "{} executed. exit: {:?}. artifact: {}",
                    scan.tool,
                    exit_code,
                    path.clone().unwrap_or_else(|| "<memory>".to_string())
                );

                let execution = if output.status.success() {
                    StepExecution::completed(scan.name.clone(), StepKind::Scan, Some(message))
                } else {
                    StepExecution::failed(scan.name.clone(), StepKind::Scan, Some(message))
                };

                StepOutcome::with_artifact(
                    execution,
                    StoredArtifact {
                        name: label,
                        kind: ArtifactKind::Scan,
                        path,
                        data: artifact_data,
                    },
                )
            }
            Err(err) => StepOutcome::from_execution(StepExecution::failed(
                scan.name.clone(),
                StepKind::Scan,
                Some(format!("failed to execute tool '{}': {err}", scan.tool)),
            )),
        }
    }

    fn process_script(
        &self,
        script: &ScriptStep,
        variables: &HashMap<String, LiteralValue>,
        secrets: &mut SecretStore,
    ) -> StepOutcome {
        let params = match resolve_map(&script.params, variables, secrets) {
            Ok(map) => map,
            Err(err) => {
                return StepOutcome::from_execution(StepExecution::failed(
                    script.name.clone(),
                    StepKind::Script,
                    Some(format!("failed to resolve variables: {err}")),
                ))
            }
        };

        let run_value = match params.get("run") {
            Some(value) if !value.trim().is_empty() => value.clone(),
            _ => {
                return StepOutcome::from_execution(StepExecution::failed(
                    script.name.clone(),
                    StepKind::Script,
                    Some("missing required parameter: run".to_string()),
                ))
            }
        };

        let mut program_and_initial_args = match shell_words::split(&run_value) {
            Ok(parts) => parts,
            Err(err) => {
                return StepOutcome::from_execution(StepExecution::failed(
                    script.name.clone(),
                    StepKind::Script,
                    Some(format!("failed to parse 'run' command: {err}")),
                ))
            }
        };

        if program_and_initial_args.is_empty() {
            return StepOutcome::from_execution(StepExecution::failed(
                script.name.clone(),
                StepKind::Script,
                Some("run command produced no executable".to_string()),
            ));
        }

        let program = program_and_initial_args.remove(0);
        let mut invocation = vec![program.clone()];
        let mut cmd = Command::new(&program);
        for arg in program_and_initial_args {
            cmd.arg(&arg);
            invocation.push(arg);
        }

        if let Some(args_value) = params.get("args") {
            match shell_words::split(args_value) {
                Ok(extra) => {
                    for arg in extra {
                        cmd.arg(&arg);
                        invocation.push(arg);
                    }
                }
                Err(err) => {
                    return StepOutcome::from_execution(StepExecution::failed(
                        script.name.clone(),
                        StepKind::Script,
                        Some(format!("failed to parse 'args' value: {err}")),
                    ))
                }
            }
        }

        if let Some(cwd) = params.get("cwd") {
            cmd.current_dir(cwd);
        }

        let started_at = OffsetDateTime::now_utc();
        let timer = Instant::now();

        match cmd.output() {
            Ok(output) => {
                let duration_ms = timer.elapsed().as_millis();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                let timestamp = started_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| "unknown".to_string());

                let artifact_label = script
                    .output
                    .clone()
                    .unwrap_or_else(|| format!("script_{}", script.name));
                let artifact_data = json!(ScriptArtifact {
                    name: script.name.clone(),
                    command: invocation.clone(),
                    stdout,
                    stderr,
                    exit_code,
                    started_at: timestamp,
                    duration_ms,
                });
                let path = self
                    .write_artifact(&artifact_label, &artifact_data)
                    .map(|p| p.to_string_lossy().to_string());

                let message = format!(
                    "script '{}' executed with code {:?}. artifact: {}",
                    artifact_label,
                    exit_code,
                    path.clone().unwrap_or_else(|| "<memory>".to_string())
                );

                let execution = if output.status.success() {
                    StepExecution::completed(script.name.clone(), StepKind::Script, Some(message))
                } else {
                    StepExecution::failed(script.name.clone(), StepKind::Script, Some(message))
                };

                StepOutcome::with_artifact(
                    execution,
                    StoredArtifact {
                        name: artifact_label,
                        kind: ArtifactKind::Script,
                        path,
                        data: artifact_data,
                    },
                )
            }
            Err(err) => StepOutcome::from_execution(StepExecution::failed(
                script.name.clone(),
                StepKind::Script,
                Some(format!("failed to execute script '{}': {err}", script.name)),
            )),
        }
    }

    fn process_variable(
        &self,
        variable: &VariableDecl,
        overrides: &HashMap<String, LiteralValue>,
        variables: &mut HashMap<String, LiteralValue>,
        secrets: &mut SecretStore,
    ) -> StepOutcome {
        let (resolved, note) = if let Some(raw) = overrides.get(&variable.name) {
            match resolve_literal_value(raw, variables, secrets) {
                Ok(value) => (value, Some("(override)")),
                Err(err) => {
                    return StepOutcome::from_execution(StepExecution::failed(
                        variable.name.clone(),
                        StepKind::Variable,
                        Some(format!("failed to resolve override: {err}")),
                    ))
                }
            }
        } else {
            match resolve_literal_value(&variable.value, variables, secrets) {
                Ok(value) => (value, None),
                Err(err) => {
                    return StepOutcome::from_execution(StepExecution::failed(
                        variable.name.clone(),
                        StepKind::Variable,
                        Some(format!("failed to resolve variables: {err}")),
                    ))
                }
            }
        };

        variables.insert(variable.name.clone(), resolved.clone());

        let message = match note {
            Some(tag) => format!("{} = {} {}", variable.name, resolved.display(), tag),
            None => format!("{} = {}", variable.name, resolved.display()),
        };

        StepOutcome::from_execution(StepExecution::completed(
            variable.name.clone(),
            StepKind::Variable,
            Some(message),
        ))
    }

    fn process_secret(&self, secret: &SecretStep, secrets: &mut SecretStore) -> StepOutcome {
        match &secret.source {
            SecretSource::Env { mappings } => {
                let mut aggregated: BTreeMap<String, String> = BTreeMap::new();

                for (alias, env_key) in mappings {
                    let scoped = format!("{}.{}", secret.name, alias);
                    if let Some(existing) = secrets.peek(&scoped) {
                        aggregated.insert(alias.clone(), existing.to_string());
                        continue;
                    }

                    match env::var(env_key) {
                        Ok(value) => {
                            secrets.insert(scoped, value.clone());
                            aggregated.insert(alias.clone(), value);
                        }
                        Err(err) => {
                            return StepOutcome::from_execution(StepExecution::failed(
                                secret.name.clone(),
                                StepKind::Secret,
                                Some(format!(
                                    "failed to read environment variable {}: {err}",
                                    env_key
                                )),
                            ))
                        }
                    }
                }

                if aggregated.len() == 1 {
                    if let Some((_, value)) = aggregated.iter().next() {
                        secrets.insert(secret.name.clone(), value.clone());
                    }
                }

                let count = aggregated.len();
                let message = if count == 1 {
                    format!("secret '{}' loaded from env (1 value)", secret.name)
                } else {
                    format!(
                        "secret '{}' loaded from env ({} values)",
                        secret.name, count
                    )
                };

                StepOutcome::from_execution(StepExecution::completed(
                    secret.name.clone(),
                    StepKind::Secret,
                    Some(message),
                ))
            }
            SecretSource::File { path } => match fs::read_to_string(path) {
                Ok(value) => {
                    secrets.insert(secret.name.clone(), value);
                    StepOutcome::from_execution(StepExecution::completed(
                        secret.name.clone(),
                        StepKind::Secret,
                        Some(format!("secret '{}' loaded from file", secret.name)),
                    ))
                }
                Err(err) => StepOutcome::from_execution(StepExecution::failed(
                    secret.name.clone(),
                    StepKind::Secret,
                    Some(format!("failed to read file {}: {err}", path)),
                )),
            },
            SecretSource::Vault { .. } => {
                StepOutcome::from_execution(StepExecution::not_implemented(
                    secret.name.clone(),
                    StepKind::Secret,
                    Some("vault provider is not implemented yet".to_string()),
                ))
            }
        }
    }

    fn process_asset_group(
        &self,
        group: &AssetGroupStep,
        variables: &HashMap<String, LiteralValue>,
        secrets: &mut SecretStore,
    ) -> StepOutcome {
        let resolved = match resolve_map(&group.properties, variables, secrets) {
            Ok(map) => map,
            Err(err) => {
                return StepOutcome::from_execution(StepExecution::failed(
                    group.name.clone(),
                    StepKind::AssetGroup,
                    Some(format!("failed to resolve variables: {err}")),
                ))
            }
        };

        let artifact_name = format!("asset_group:{}", group.name);
        let data = json!(AssetGroupArtifact {
            name: group.name.clone(),
            properties: resolved.clone(),
        });

        StepOutcome::with_artifact(
            StepExecution::skipped(
                group.name.clone(),
                StepKind::AssetGroup,
                Some("stored asset group definition".to_string()),
            ),
            StoredArtifact {
                name: artifact_name,
                kind: ArtifactKind::AssetGroup,
                path: None,
                data,
            },
        )
    }

    fn process_scan(
        &self,
        scan: &ScanStep,
        variables: &HashMap<String, LiteralValue>,
        secrets: &mut SecretStore,
    ) -> StepOutcome {
        let params = match resolve_map(&scan.params, variables, secrets) {
            Ok(map) => map,
            Err(err) => {
                return StepOutcome::from_execution(StepExecution::failed(
                    scan.name.clone(),
                    StepKind::Scan,
                    Some(format!("failed to resolve variables: {err}")),
                ))
            }
        };

        if scan.tool != "nmap" {
            return self.process_generic_scan(scan, params);
        }

        let target = match params.get("target") {
            Some(value) => value.clone(),
            None => {
                return StepOutcome::from_execution(StepExecution::failed(
                    scan.name.clone(),
                    StepKind::Scan,
                    Some("missing required parameter: target".to_string()),
                ))
            }
        };

        let mut cmd = Command::new(&scan.tool);

        if let Some(flags) = params.get("flags") {
            match shell_words::split(flags) {
                Ok(parts) => {
                    for part in parts {
                        cmd.arg(part);
                    }
                }
                Err(err) => {
                    return StepOutcome::from_execution(StepExecution::failed(
                        scan.name.clone(),
                        StepKind::Scan,
                        Some(format!("failed to parse flags: {err}")),
                    ));
                }
            }
        }

        cmd.arg("-oX");
        cmd.arg("-");
        cmd.arg(&target);

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    match parse_nmap_xml(&output.stdout, &target) {
                        Ok(parsed) => {
                            let label = scan
                                .output
                                .clone()
                                .unwrap_or_else(|| format!("findings_{}", scan.name));
                            let data = json!(parsed);
                            let path = self
                                .write_artifact(&label, &data)
                                .map(|p| p.to_string_lossy().to_string());

                            let message = format!(
                                "{} completed for target {}.\nartifact: {}",
                                scan.tool,
                                target,
                                path.clone().unwrap_or_else(|| "<memory>".to_string())
                            );

                            StepOutcome::with_artifact(
                                StepExecution::completed(
                                    scan.name.clone(),
                                    StepKind::Scan,
                                    Some(message),
                                ),
                                StoredArtifact {
                                    name: label,
                                    kind: ArtifactKind::Scan,
                                    path,
                                    data,
                                },
                            )
                        }
                        Err(err) => StepOutcome::from_execution(StepExecution::failed(
                            scan.name.clone(),
                            StepKind::Scan,
                            Some(format!("failed to parse nmap output: {err}")),
                        )),
                    }
                } else {
                    let error_msg = format!(
                        "{} exited with code {:?}\nstdout:\n{}\nstderr:\n{}",
                        scan.tool,
                        output.status.code(),
                        truncate_output(&output.stdout),
                        truncate_output(&output.stderr)
                    );
                    StepOutcome::from_execution(StepExecution::failed(
                        scan.name.clone(),
                        StepKind::Scan,
                        Some(error_msg),
                    ))
                }
            }
            Err(err) => StepOutcome::from_execution(StepExecution::failed(
                scan.name.clone(),
                StepKind::Scan,
                Some(format!("failed to spawn '{}': {}", scan.tool, err)),
            )),
        }
    }

    fn process_report(
        &self,
        report: &ReportStep,
        store: &HashMap<String, StoredArtifact>,
        variables: &HashMap<String, LiteralValue>,
        secrets: &mut SecretStore,
    ) -> StepOutcome {
        let include_names = match resolve_list(&report.includes, variables, secrets) {
            Ok(list) => list,
            Err(err) => {
                return StepOutcome::from_execution(StepExecution::failed(
                    report.name.clone(),
                    StepKind::Report,
                    Some(format!("failed to resolve variables: {err}")),
                ))
            }
        };

        let mut includes = BTreeMap::new();
        let mut tables = BTreeMap::new();

        for include in &include_names {
            match store.get(include) {
                Some(artifact) => {
                    includes.insert(include.clone(), artifact.data.clone());
                    if artifact.kind == ArtifactKind::Scan {
                        if let Some(table) = build_table_from_scan(&artifact.data) {
                            tables.insert(include.clone(), table);
                        }
                    }
                }
                None => {
                    return StepOutcome::from_execution(StepExecution::failed(
                        report.name.clone(),
                        StepKind::Report,
                        Some(format!("missing artifact '{}'", include)),
                    ));
                }
            }
        }

        let generated_at = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        let tables_for_display = tables.clone();
        let report_data = json!(ReportArtifact {
            target: report.name.clone(),
            generated_at,
            includes,
            tables,
        });

        let report_label = format!("report:{}", report.name);
        let path = self
            .write_artifact(&sanitize_label(&report_label), &report_data)
            .map(|p| p.to_string_lossy().to_string());

        if report.name == "stdout" {
            if let Ok(pretty) = serde_json::to_string_pretty(&report_data) {
                println!("{pretty}");
            }
            for (alias, table) in &tables_for_display {
                if table.rows.is_empty() {
                    continue;
                }
                println!("\n[table] {alias}");
                let rendered = render_table(table);
                println!("{rendered}");
            }
        }

        let message = format!(
            "report generated for target '{}'. artifact: {}",
            report.name,
            path.clone().unwrap_or_else(|| "<memory>".to_string())
        );

        StepOutcome::with_artifact(
            StepExecution::completed(report.name.clone(), StepKind::Report, Some(message)),
            StoredArtifact {
                name: report_label,
                kind: ArtifactKind::Report,
                path,
                data: report_data,
            },
        )
    }

    fn write_artifact(&self, label: &str, data: &Value) -> Option<PathBuf> {
        let safe_label = sanitize_label(label);
        let path = self.artifacts_dir.join(format!("{safe_label}.json"));

        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!(
                    "[warn] failed to create artifact directory {:?}: {err}",
                    parent
                );
                return None;
            }
        }

        match serde_json::to_vec_pretty(data) {
            Ok(bytes) => match fs::File::create(&path) {
                Ok(mut file) => {
                    if let Err(err) = file.write_all(&bytes) {
                        eprintln!("[warn] failed to write artifact {:?}: {err}", path);
                        None
                    } else {
                        Some(path)
                    }
                }
                Err(err) => {
                    eprintln!("[warn] failed to create artifact {:?}: {err}", path);
                    None
                }
            },
            Err(err) => {
                eprintln!("[warn] failed to serialize artifact '{}': {err}", label);
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
struct StepOutcome {
    execution: StepExecution,
    artifact: Option<StoredArtifact>,
}

impl StepOutcome {
    fn from_execution(execution: StepExecution) -> Self {
        Self {
            execution,
            artifact: None,
        }
    }

    fn with_artifact(execution: StepExecution, artifact: StoredArtifact) -> Self {
        Self {
            execution,
            artifact: Some(artifact),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub steps: Vec<StepExecution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub report: ExecutionReport,
    pub artifacts: Vec<StoredArtifact>,
}

impl ExecutionReport {
    pub fn has_failures(&self) -> bool {
        self.steps
            .iter()
            .any(|step| step.status == ExecutionStatus::Failed)
    }
}

impl fmt::Display for ExecutionReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.steps.is_empty() {
            writeln!(f, "No steps to execute.")?;
            return Ok(());
        }

        writeln!(f, "Execution results:")?;
        for step in &self.steps {
            let status = match step.status {
                ExecutionStatus::Completed => "completed",
                ExecutionStatus::Skipped => "skipped",
                ExecutionStatus::Failed => "failed",
                ExecutionStatus::NotImplemented => "not implemented",
            };
            writeln!(f, "  - [{}] {} ({:?})", status, step.name, step.kind)?;
            if let Some(message) = &step.message {
                for line in message.lines() {
                    writeln!(f, "      {}", line)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecution {
    pub name: String,
    pub kind: StepKind,
    pub status: ExecutionStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepKind {
    AssetGroup,
    Scan,
    Variable,
    Secret,
    Script,
    Report,
    Conditional,
    Loop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Completed,
    Skipped,
    Failed,
    NotImplemented,
}

impl StepExecution {
    pub fn completed(name: String, kind: StepKind, message: Option<String>) -> Self {
        Self {
            name,
            kind,
            status: ExecutionStatus::Completed,
            message,
        }
    }

    pub fn failed(name: String, kind: StepKind, message: Option<String>) -> Self {
        Self {
            name,
            kind,
            status: ExecutionStatus::Failed,
            message,
        }
    }

    pub fn skipped(name: String, kind: StepKind, message: Option<String>) -> Self {
        Self {
            name,
            kind,
            status: ExecutionStatus::Skipped,
            message,
        }
    }

    pub fn not_implemented(name: String, kind: StepKind, message: Option<String>) -> Self {
        Self {
            name,
            kind,
            status: ExecutionStatus::NotImplemented,
            message,
        }
    }
}

fn truncate_output(bytes: &[u8]) -> String {
    const MAX: usize = 512;
    let text = String::from_utf8_lossy(bytes);
    if text.len() > MAX {
        format!("{}…", &text[..MAX])
    } else {
        text.to_string()
    }
}

fn resolve_literal_value(
    value: &LiteralValue,
    variables: &HashMap<String, LiteralValue>,
    secrets: &mut SecretStore,
) -> Result<LiteralValue, String> {
    match value {
        LiteralValue::String(s) => {
            let substituted = substitute_variables(s, variables, secrets)?;
            Ok(LiteralValue::String(substituted))
        }
        LiteralValue::Number(n) => Ok(LiteralValue::Number(*n)),
        LiteralValue::Boolean(b) => Ok(LiteralValue::Boolean(*b)),
        LiteralValue::Array(items) => {
            let mut resolved = Vec::with_capacity(items.len());
            for item in items {
                resolved.push(resolve_literal_value(item, variables, secrets)?);
            }
            Ok(LiteralValue::Array(resolved))
        }
        LiteralValue::Object(map) => {
            let mut resolved = BTreeMap::new();
            for (k, v) in map {
                resolved.insert(k.clone(), resolve_literal_value(v, variables, secrets)?);
            }
            Ok(LiteralValue::Object(resolved))
        }
    }
}

fn literal_to_string(value: &LiteralValue) -> String {
    value.display()
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn resolve_map(
    source: &BTreeMap<String, String>,
    variables: &HashMap<String, LiteralValue>,
    secrets: &mut SecretStore,
) -> Result<BTreeMap<String, String>, String> {
    let mut resolved = BTreeMap::new();
    for (key, value) in source {
        let substituted = substitute_variables(value, variables, secrets)?;
        resolved.insert(key.clone(), substituted);
    }
    Ok(resolved)
}

fn resolve_list(
    items: &[String],
    variables: &HashMap<String, LiteralValue>,
    secrets: &mut SecretStore,
) -> Result<Vec<String>, String> {
    let mut resolved = Vec::with_capacity(items.len());
    for item in items {
        let substituted = substitute_variables(item, variables, secrets)?;
        resolved.push(substituted);
    }
    Ok(resolved)
}

fn substitute_variables(
    value: &str,
    variables: &HashMap<String, LiteralValue>,
    secrets: &mut SecretStore,
) -> Result<String, String> {
    let mut result = String::with_capacity(value.len());
    let mut cursor = 0;

    while let Some(start_offset) = value[cursor..].find("${") {
        let start_idx = cursor + start_offset;
        result.push_str(&value[cursor..start_idx]);

        let remainder = &value[start_idx + 2..];
        let end_offset = remainder
            .find('}')
            .ok_or_else(|| "unterminated variable placeholder".to_string())?;
        let end_idx = start_idx + 2 + end_offset;
        let token = remainder[..end_offset].trim();

        if token.is_empty() {
            return Err("empty variable placeholder".to_string());
        }

        if let Some(secret_name) = token.strip_prefix("secret:") {
            let key = secret_name.trim();
            if key.is_empty() {
                return Err("empty secret placeholder".to_string());
            }
            let resolved = secrets
                .resolve(key)
                .ok_or_else(|| format!("undefined secret '{key}'"))?;
            result.push_str(&resolved);
        } else {
            let replacement = variables
                .get(token)
                .ok_or_else(|| format!("undefined variable '{token}'"))?;
            result.push_str(&literal_to_string(replacement));
        }
        cursor = end_idx + 1;
    }

    result.push_str(&value[cursor..]);
    Ok(result)
}

fn resolve_iterable(
    iterable: &LoopIterable,
    variables: &HashMap<String, LiteralValue>,
    secrets: &mut SecretStore,
) -> Result<Vec<LiteralValue>, String> {
    match iterable {
        LoopIterable::Variable(name) => match variables.get(name) {
            Some(LiteralValue::Array(items)) => Ok(items.clone()),
            Some(LiteralValue::String(value)) => Ok(vec![LiteralValue::String(value.clone())]),
            Some(other) => Err(format!(
                "variable '{}' is not iterable (found {})",
                name,
                other.display()
            )),
            None => Err(format!("undefined variable '{}'", name)),
        },
        LoopIterable::Literal(literal) => {
            let resolved = resolve_literal_value(literal, variables, secrets)?;
            match resolved {
                LiteralValue::Array(items) => Ok(items),
                LiteralValue::String(value) => Ok(vec![LiteralValue::String(value)]),
                other => Err(format!(
                    "loop iterable must be array or string, found {}",
                    other.display()
                )),
            }
        }
    }
}

fn evaluate_condition(
    expr: &ConditionExpr,
    variables: &HashMap<String, LiteralValue>,
    secrets: &mut SecretStore,
) -> Result<bool, String> {
    match expr {
        ConditionExpr::Literal(value) => Ok(*value),
        ConditionExpr::Variable(name) => match variables.get(name) {
            Some(LiteralValue::Boolean(value)) => Ok(*value),
            Some(other) => Err(format!(
                "variable '{}' is not boolean (found {})",
                name,
                other.display()
            )),
            None => Err(format!("undefined variable '{}'", name)),
        },
        ConditionExpr::Not(inner) => Ok(!evaluate_condition(inner, variables, secrets)?),
        ConditionExpr::Equals(left, right) => {
            let lhs = evaluate_operand(left, variables, secrets)?;
            let rhs = evaluate_operand(right, variables, secrets)?;
            Ok(lhs == rhs)
        }
        ConditionExpr::NotEquals(left, right) => {
            let lhs = evaluate_operand(left, variables, secrets)?;
            let rhs = evaluate_operand(right, variables, secrets)?;
            Ok(lhs != rhs)
        }
    }
}

fn evaluate_operand(
    operand: &ConditionOperand,
    variables: &HashMap<String, LiteralValue>,
    secrets: &mut SecretStore,
) -> Result<LiteralValue, String> {
    match operand {
        ConditionOperand::Variable(name) => variables
            .get(name)
            .cloned()
            .ok_or_else(|| format!("undefined variable '{}'", name)),
        ConditionOperand::Literal(value) => resolve_literal_value(value, variables, secrets),
    }
}

fn build_table_from_scan(data: &Value) -> Option<TableArtifact> {
    let findings = data.get("findings")?.as_array()?;
    if findings.is_empty() {
        return None;
    }

    let columns = vec![
        "asset_id".to_string(),
        "port".to_string(),
        "protocol".to_string(),
        "service".to_string(),
        "state".to_string(),
        "severity".to_string(),
        "description".to_string(),
    ];

    let mut rows = Vec::new();
    for finding in findings {
        let mut row = BTreeMap::new();
        row.insert(
            "asset_id".to_string(),
            finding.get("asset_id").cloned().unwrap_or(Value::Null),
        );
        row.insert(
            "port".to_string(),
            finding.get("port").cloned().unwrap_or(Value::Null),
        );
        row.insert(
            "protocol".to_string(),
            finding.get("protocol").cloned().unwrap_or(Value::Null),
        );
        row.insert(
            "service".to_string(),
            finding.get("service").cloned().unwrap_or(Value::Null),
        );
        row.insert(
            "state".to_string(),
            finding.get("state").cloned().unwrap_or(Value::Null),
        );
        row.insert(
            "severity".to_string(),
            finding.get("severity").cloned().unwrap_or(Value::Null),
        );
        row.insert(
            "description".to_string(),
            finding.get("description").cloned().unwrap_or(Value::Null),
        );
        rows.push(row);
    }

    Some(TableArtifact { columns, rows })
}

fn render_table(table: &TableArtifact) -> String {
    let mut display = Table::new();
    display.load_preset(ASCII_FULL);
    display.set_header(table.columns.clone());

    for row in &table.rows {
        let cells: Vec<String> = table
            .columns
            .iter()
            .map(|column| value_to_string(row.get(column)))
            .collect();
        display.add_row(cells);
    }

    display.to_string()
}

fn value_to_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::Null) | None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn parse_nmap_xml(xml: &[u8], target: &str) -> Result<ScanArtifacts, String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut hosts = Vec::new();
    let mut current_host: Option<HostBuilder> = None;
    let mut current_port: Option<PortBuilder> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => match element.name() {
                QName(b"host") => {
                    current_host = Some(HostBuilder::default());
                }
                QName(b"address") => {
                    if let Some(host) = current_host.as_mut() {
                        let mut address_value = None;
                        let mut addr_type = None;
                        for attr in element.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"addr" => {
                                    address_value =
                                        Some(String::from_utf8_lossy(&attr.value).to_string())
                                }
                                b"addrtype" => {
                                    addr_type =
                                        Some(String::from_utf8_lossy(&attr.value).to_string())
                                }
                                _ => {}
                            }
                        }
                        if let Some(addr) = address_value {
                            host.addresses
                                .push((addr, addr_type.unwrap_or_else(|| "unknown".to_string())));
                        }
                    }
                }
                QName(b"hostname") => {
                    if let Some(host) = current_host.as_mut() {
                        for attr in element.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                host.hostnames
                                    .push(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                }
                QName(b"port") => {
                    let mut builder = PortBuilder::default();
                    for attr in element.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"protocol" => {
                                builder.protocol =
                                    Some(String::from_utf8_lossy(&attr.value).to_string())
                            }
                            b"portid" => {
                                if let Ok(port) =
                                    String::from_utf8_lossy(&attr.value).parse::<u16>()
                                {
                                    builder.port = Some(port);
                                }
                            }
                            _ => {}
                        }
                    }
                    current_port = Some(builder);
                }
                QName(b"state") => {
                    if let Some(port) = current_port.as_mut() {
                        for attr in element.attributes().flatten() {
                            if attr.key.as_ref() == b"state" {
                                port.state = Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                }
                QName(b"service") => {
                    if let Some(port) = current_port.as_mut() {
                        for attr in element.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                port.service =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Empty(element)) => match element.name() {
                QName(b"address") => {
                    if let Some(host) = current_host.as_mut() {
                        let mut address_value = None;
                        let mut addr_type = None;
                        for attr in element.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"addr" => {
                                    address_value =
                                        Some(String::from_utf8_lossy(&attr.value).to_string())
                                }
                                b"addrtype" => {
                                    addr_type =
                                        Some(String::from_utf8_lossy(&attr.value).to_string())
                                }
                                _ => {}
                            }
                        }
                        if let Some(addr) = address_value {
                            host.addresses
                                .push((addr, addr_type.unwrap_or_else(|| "unknown".to_string())));
                        }
                    }
                }
                QName(b"hostname") => {
                    if let Some(host) = current_host.as_mut() {
                        for attr in element.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                host.hostnames
                                    .push(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::End(element)) => match element.name() {
                QName(b"port") => {
                    if let (Some(host), Some(port)) = (current_host.as_mut(), current_port.take()) {
                        host.ports.push(port);
                    }
                }
                QName(b"host") => {
                    if let Some(host) = current_host.take() {
                        hosts.push(host);
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(err) => return Err(format!("XML parsing error: {err}")),
            _ => {}
        }
        buf.clear();
    }

    let mut assets = Vec::new();
    let mut findings = Vec::new();

    for host in hosts {
        let addresses: Vec<String> = host
            .addresses
            .iter()
            .map(|(addr, _)| addr.clone())
            .collect();
        if addresses.is_empty() {
            continue;
        }
        let primary_addr = addresses.first().cloned().unwrap_or_default();
        let asset_id = format!("asset://host/{}", primary_addr);

        let asset = Asset {
            id: asset_id.clone(),
            addresses: addresses.clone(),
            hostnames: host.hostnames.clone(),
            labels: BTreeMap::new(),
        };

        for port in host.ports {
            if port.state.as_deref() != Some("open") {
                continue;
            }

            let port_number = port.port.unwrap_or(0);
            let protocol = port.protocol.clone().unwrap_or_else(|| "tcp".to_string());
            let service = port.service.clone();
            let finding_id = format!("finding://{}/{}-{}", primary_addr, protocol, port_number);

            let mut evidence = BTreeMap::new();
            evidence.insert("port".to_string(), Value::from(port_number));
            if let Some(state) = &port.state {
                evidence.insert("state".to_string(), Value::String(state.clone()));
            }
            if let Some(svc) = &service {
                evidence.insert("service".to_string(), Value::String(svc.clone()));
            }

            findings.push(Finding {
                id: finding_id,
                asset_id: asset_id.clone(),
                port: port_number,
                protocol: protocol.clone(),
                state: port.state.unwrap_or_else(|| "unknown".to_string()),
                service: service.clone(),
                title: format!("{}:{} {} open", primary_addr, port_number, protocol),
                description: format!(
                    "Port {} {} is open on asset {} with service {:?}",
                    port_number, protocol, primary_addr, service
                ),
                severity: "informational".to_string(),
                evidence,
            });
        }

        assets.push(asset);
    }

    Ok(ScanArtifacts {
        tool: "nmap".to_string(),
        target: target.to_string(),
        assets,
        findings,
        raw_xml: String::from_utf8_lossy(xml).to_string(),
    })
}

#[derive(Default)]
struct HostBuilder {
    addresses: Vec<(String, String)>,
    hostnames: Vec<String>,
    ports: Vec<PortBuilder>,
}

#[derive(Default)]
struct PortBuilder {
    port: Option<u16>,
    protocol: Option<String>,
    state: Option<String>,
    service: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn resolves_secret_placeholders_and_masks_messages() {
        let env_key = "AXION_TEST_SECRET_VALUE";
        std::env::set_var(env_key, "super-secret-token");

        let source = format!(
            r#"
secret api from env {{
  token = "{env_key}"
}}

let header = "Bearer ${{secret:api.token}}"
"#
        );

        let scenario = crate::scenario::parse_scenario(&source).expect("failed to parse scenario");
        let executor = Executor::new();
        let overrides = HashMap::new();
        let secret_overrides = HashMap::new();
        let outcome = executor.execute_with_vars(&scenario, &overrides, &secret_overrides);

        let secret_step = outcome
            .report
            .steps
            .iter()
            .find(|step| step.name == "api")
            .expect("secret step present");
        assert_eq!(secret_step.status, ExecutionStatus::Completed);

        let variable_step = outcome
            .report
            .steps
            .iter()
            .find(|step| step.name == "header")
            .expect("variable step present");
        assert_eq!(variable_step.status, ExecutionStatus::Completed);
        let message = variable_step.message.as_deref().unwrap_or_default();
        assert!(message.contains("Bearer ***"));
        assert!(!message.contains("super-secret-token"));

        std::env::remove_var(env_key);
    }

    #[test]
    fn secret_override_allows_missing_env() {
        let source = r#"
secret token from env {
  value = "AXION_TEST_MISSING_ENV"
}

let captured = "${secret:token.value}"
"#;

        std::env::remove_var("AXION_TEST_MISSING_ENV");

        let scenario = crate::scenario::parse_scenario(source).expect("failed to parse scenario");
        let executor = Executor::new();
        let overrides = HashMap::new();
        let mut secret_overrides = HashMap::new();
        secret_overrides.insert("token.value".to_string(), "override-secret".to_string());

        let outcome = executor.execute_with_vars(&scenario, &overrides, &secret_overrides);

        let secret_step = outcome
            .report
            .steps
            .iter()
            .find(|step| step.name == "token")
            .expect("secret step present");
        assert_eq!(secret_step.status, ExecutionStatus::Completed);

        let variable_step = outcome
            .report
            .steps
            .iter()
            .find(|step| step.name == "captured")
            .expect("variable step present");
        assert_eq!(variable_step.status, ExecutionStatus::Completed);
        let message = variable_step.message.as_deref().unwrap_or_default();
        assert!(message.contains("***"));
        assert!(!message.contains("override-secret"));
    }
}
