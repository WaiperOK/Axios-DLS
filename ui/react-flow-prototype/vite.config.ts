import { defineConfig } from "vite";
import type { Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";
import { access, writeFile, rm } from "node:fs/promises";
import { spawn } from "node:child_process";
import type { IncomingMessage, ServerResponse } from "node:http";

const projectDir = fileURLToPath(new URL(".", import.meta.url));
const workspaceRoot = resolve(projectDir, "..", "..");

type AxionCommand = {
  command: string;
  args: string[];
};

type Middleware = (
  req: IncomingMessage,
  res: ServerResponse,
  next: () => void,
) => void;

type OverrideMap = Record<string, unknown>;

const executableCache: { entry?: AxionCommand } = {};

class HttpError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

function sanitizeLines(raw: string): string[] {
  return raw
    .replace(/\r/g, "")
    .split("\n")
    .filter((line) => line.length > 0);
}

function splitCommand(input: string): string[] {
  const result: string[] = [];
  let current = "";
  let quote: '"' | "'" | null = null;
  let escaped = false;

  for (const char of input) {
    if (escaped) {
      current += char;
      escaped = false;
      continue;
    }
    if (char === "\\") {
      escaped = true;
      continue;
    }
    if (quote) {
      if (char === quote) {
        quote = null;
      } else {
        current += char;
      }
      continue;
    }
    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }
    if (/\s/.test(char)) {
      if (current.length > 0) {
        result.push(current);
        current = "";
      }
      continue;
    }
    current += char;
  }

  if (escaped) {
    current += "\\";
  }
  if (quote) {
    throw new HttpError(400, "Unclosed quote in command");
  }
  if (current.length > 0) {
    result.push(current);
  }
  return result;
}

async function readJsonBody(req: IncomingMessage): Promise<unknown> {
  const chunks: Uint8Array[] = [];
  for await (const chunk of req) {
    chunks.push(typeof chunk === "string" ? Buffer.from(chunk) : chunk);
  }
  if (chunks.length === 0) {
    return {};
  }
  const raw = Buffer.concat(chunks).toString("utf8").trim();
  if (!raw) {
    return {};
  }
  try {
    return JSON.parse(raw);
  } catch (error) {
    throw new HttpError(400, "Invalid JSON payload");
  }
}

async function resolveExecutable(): Promise<AxionCommand> {
  if (executableCache.entry) {
    return executableCache.entry;
  }
  const binaryName = process.platform === "win32" ? "axion-cli.exe" : "axion-cli";
  const binaryPath = resolve(workspaceRoot, "target", "debug", binaryName);
  try {
    await access(binaryPath);
    executableCache.entry = { command: binaryPath, args: [] };
  } catch {
    executableCache.entry = {
      command: "cargo",
      args: ["run", "-p", "axion-cli", "--"],
    };
  }
  return executableCache.entry;
}

async function runAxion(args: string[]) {
  const executable = await resolveExecutable();
  const child = spawn(executable.command, [...executable.args, ...args], {
    cwd: workspaceRoot,
    shell: false,
  });

  let stdout = "";
  let stderr = "";

  child.stdout?.on("data", (chunk: Buffer) => {
    stdout += chunk.toString("utf8");
  });
  child.stderr?.on("data", (chunk: Buffer) => {
    stderr += chunk.toString("utf8");
  });

  return new Promise<{
    exitCode: number;
    stdout: string;
    stderr: string;
  }>((resolvePromise, rejectPromise) => {
    let settled = false;
    child.once("error", (error) => {
      if (!settled) {
        settled = true;
        rejectPromise(error);
      }
    });
    child.once("close", (code) => {
      if (!settled) {
        settled = true;
        const normalizedCode = typeof code === "number" ? code : -1;
        resolvePromise({ exitCode: normalizedCode, stdout, stderr });
      }
    });
  });
}

function buildOverrideArgs(source: unknown, flag: "--var" | "--secret"): string[] {
  if (!source || typeof source !== "object") {
    return [];
  }
  const entries = Object.entries(source as OverrideMap)
    .filter(([key]) => key.length > 0)
    .map(([key, value]) => [`${flag}`, `${key}=${String(value)}`]);
  return entries.flat();
}

async function writeTemporaryScenario(content: string): Promise<string> {
  if (!content.trim()) {
    throw new HttpError(400, "Scenario content is empty");
  }
  const unique = `${Date.now().toString(36)}-${Math.random()
    .toString(36)
    .slice(2)}`;
  const scenarioPath = resolve(
    workspaceRoot,
    "examples",
    `.ui-run-${unique}.ax`,
  );
  await writeFile(scenarioPath, content, "utf8");
  return scenarioPath;
}

function respondJson(res: ServerResponse, payload: unknown, status = 200) {
  res.statusCode = status;
  res.setHeader("Content-Type", "application/json");
  res.end(JSON.stringify(payload));
}

function handleError(res: ServerResponse, error: unknown) {
  if (error instanceof HttpError) {
    respondJson(res, { error: error.message }, error.status);
    return;
  }
  console.error("[axion-api] unexpected error", error);
  respondJson(res, { error: "Internal server error" }, 500);
}

function createApiHandler(): Middleware {
  return (req, res, next) => {
    if (!req.url?.startsWith("/api/")) {
      next();
      return;
    }

    (async () => {
      if (req.method !== "POST") {
        throw new HttpError(405, "Method not allowed");
      }

      const body = (await readJsonBody(req)) as Record<string, unknown>;
      if (req.url === "/api/run") {
        const scenario = body.scenario;
        if (typeof scenario !== "string") {
          throw new HttpError(400, "Field 'scenario' must be a string");
        }
        const varsArgs = buildOverrideArgs(body.vars, "--var");
        const secretArgs = buildOverrideArgs(body.secrets, "--secret");
        const scenarioPath = await writeTemporaryScenario(scenario);

        try {
          const { stdout, stderr, exitCode } = await runAxion([
            "run",
            scenarioPath,
            "--json",
            ...varsArgs,
            ...secretArgs,
          ]);
          respondJson(res, {
            stdout: sanitizeLines(stdout),
            stderr: sanitizeLines(stderr),
            exitCode,
          });
        } finally {
          await rm(scenarioPath, { force: true });
        }
        return;
      }

      if (req.url === "/api/cli") {
        const command = body.command;
        if (typeof command !== "string") {
          throw new HttpError(400, "Field 'command' must be a string");
        }
        const args = splitCommand(command);
        if (args.length === 0) {
          throw new HttpError(400, "CLI command is empty");
        }
        const { stdout, stderr, exitCode } = await runAxion(args);
        respondJson(res, {
          stdout: sanitizeLines(stdout),
          stderr: sanitizeLines(stderr),
          exitCode,
        });
        return;
      }

      throw new HttpError(404, "Not found");
    })().catch((error) => {
      handleError(res, error);
    });
  };
}

function axionApiPlugin(): Plugin {
  const handler = createApiHandler();
  return {
    name: "axion-api",
    configureServer(server) {
      server.middlewares.use(handler);
    },
    configurePreviewServer(server) {
      server.middlewares.use(handler);
    },
  };
}

export default defineConfig({
  plugins: [react(), axionApiPlugin()],
  server: {
    port: 9001,
    strictPort: true,
  },
});
