import Editor from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import { useMemo, useState } from "react";
import { StageOutput } from "./StageOutput";
import { StageKey } from "./stageParsers";

type CompileResponse = {
  success: boolean;
  summary: string;
  diagnostics: string;
  tokens: string;
  cst: string;
  ast: string;
  hir: string;
  typecheck: string;
  thir: string;
  ir: string;
};

type BuildRunResponse = {
  compile: CompileResponse;
  stdout: string;
  stderr: string;
  exit_code: number | null;
  backend_error: string | null;
};

const TABS: Array<{ key: StageKey; label: string }> = [
  { key: "diagnostics", label: "Diagnostics" },
  { key: "tokens", label: "Tokens" },
  { key: "cst", label: "CST" },
  { key: "ast", label: "AST" },
  { key: "hir", label: "HIR" },
  { key: "typecheck", label: "Typecheck" },
  { key: "thir", label: "THIR" },
  { key: "ir", label: "IR" },
  { key: "run", label: "Run" },
];

const HANOI_EXAMPLE = `extern fn printf(fmt:str, ...) -> i32;
extern fn scanf(fmt:str, ...) -> i32;

fn total_moves(n:i32) -> i32 {
    let mut result:i32 = 1;
    let mut i:i32 = 0;
    while i < n {
        result = result * 2;
        i = i + 1;
    }
    return result - 1;
}

fn hanoi(n:i32, from:str, aux:str, to:str) {
    if n <= 0 {
        return;
    }
    hanoi(n - 1, from, to, aux);
    printf("move disk %d: %s -> %s\\n", n, from, to);
    hanoi(n - 1, aux, from, to);
}

fn main() -> i32 {
    let mut n:i32 = 0;
    printf("Enter height:\\n");
    scanf("%d", &mut n);

    printf("Tower of Hanoi\\n");
    printf("disks: %d\\n", n);
    printf("total moves: %d\\n", total_moves(n));
    hanoi(n, "A", "B", "C");
    printf("done\\n");
    return 0;
}
`;

const STRUCT_EXAMPLE = `struct Point { x: i32, y: i32 }

fn main() -> i32 {
    let p = Point { x: 1, y: 2 };
    let Point { x, y } = p;
    return x + y;
}
`;

function App() {
  const [source, setSource] = useState(HANOI_EXAMPLE);
  const [stdin, setStdin] = useState("3\n");
  const [activeTab, setActiveTab] = useState<StageKey>("diagnostics");
  const [compileResult, setCompileResult] = useState<CompileResponse | null>(
    null,
  );
  const [runResult, setRunResult] = useState<BuildRunResponse | null>(null);
  const [status, setStatus] = useState("Ready");
  const [busy, setBusy] = useState(false);
  const [example, setExample] = useState("hanoi");

  const stageText = useMemo(
    () => outputForTab(activeTab, compileResult, runResult),
    [activeTab, compileResult, runResult],
  );

  async function compile() {
    setBusy(true);
    setStatus("Compiling");
    setRunResult(null);
    try {
      const response = await invoke<CompileResponse>("compile_source", {
        request: {
          source,
          file_name: "gui_input.txt",
          verbose: false,
        },
      });
      setCompileResult(response);
      setStatus(response.success ? "Compile succeeded" : "Diagnostics emitted");
      if (!response.success) {
        setActiveTab("diagnostics");
      }
    } catch (error) {
      setStatus("Command failed");
      setCompileResult(commandErrorResponse(error));
      setActiveTab("diagnostics");
    } finally {
      setBusy(false);
    }
  }

  async function buildAndRun() {
    setBusy(true);
    setStatus("Building");
    try {
      const response = await invoke<BuildRunResponse>("build_and_run", {
        request: {
          source,
          file_name: "gui_input.txt",
          stdin,
        },
      });
      setCompileResult(response.compile);
      setRunResult(response);
      setActiveTab(response.compile.success ? "run" : "diagnostics");
      setStatus(response.backend_error ? "Backend failed" : "Run finished");
    } catch (error) {
      setStatus("Command failed");
      setCompileResult(commandErrorResponse(error));
      setRunResult(null);
      setActiveTab("diagnostics");
    } finally {
      setBusy(false);
    }
  }

  function loadExample() {
    if (example === "struct") {
      setSource(STRUCT_EXAMPLE);
      setStdin("");
    } else {
      setSource(HANOI_EXAMPLE);
      setStdin("3\n");
    }
    setCompileResult(null);
    setRunResult(null);
    setStatus("Ready");
    setActiveTab("diagnostics");
  }

  function clearAll() {
    setSource("");
    setStdin("");
    setCompileResult(null);
    setRunResult(null);
    setStatus("Ready");
    setActiveTab("diagnostics");
  }

  return (
    <main className="app-shell">
      <header className="toolbar">
        <div className="brand">
          <span className="brand-mark">MR</span>
          <div>
            <h1>MyRustCompiler</h1>
            <p>{status}</p>
          </div>
        </div>

        <div className="actions">
          <select
            value={example}
            onChange={(event) => setExample(event.target.value)}
            aria-label="Example"
          >
            <option value="hanoi">Hanoi</option>
            <option value="struct">Struct Pattern</option>
          </select>
          <button type="button" onClick={loadExample} disabled={busy}>
            Load Example
          </button>
          <button type="button" onClick={compile} disabled={busy}>
            Compile
          </button>
          <button type="button" onClick={buildAndRun} disabled={busy}>
            Build & Run
          </button>
          <button type="button" className="ghost" onClick={clearAll} disabled={busy}>
            Clear
          </button>
        </div>
      </header>

      <section className="workspace">
        <div className="editor-pane">
          <div className="pane-title">Source</div>
          <Editor
            language="rust"
            theme="vs"
            value={source}
            onChange={(value) => setSource(value ?? "")}
            options={{
              fontSize: 14,
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              wordWrap: "on",
              tabSize: 4,
              automaticLayout: true,
            }}
          />
        </div>

        <div className="output-pane">
          <nav className="tabs" aria-label="Compiler output">
            {TABS.map((tab) => (
              <button
                key={tab.key}
                type="button"
                className={activeTab === tab.key ? "active" : ""}
                onClick={() => setActiveTab(tab.key)}
              >
                {tab.label}
              </button>
            ))}
          </nav>

          {activeTab === "run" && (
            <label className="stdin-box">
              <span>stdin</span>
              <textarea
                value={stdin}
                onChange={(event) => setStdin(event.target.value)}
                spellCheck={false}
              />
            </label>
          )}

          <StageOutput stage={activeTab} text={stageText} />
        </div>
      </section>
    </main>
  );
}

function outputForTab(
  tab: StageKey,
  compile: CompileResponse | null,
  run: BuildRunResponse | null,
) {
  if (tab === "run") {
    if (!run) {
      return "No run output";
    }

    const parts = [
      `exit: ${run.exit_code ?? "-"}`,
      run.backend_error ? `backend: ${run.backend_error}` : null,
      run.stdout ? `stdout\n${run.stdout}` : "stdout\n",
      run.stderr ? `stderr\n${run.stderr}` : "stderr\n",
    ].filter(Boolean);
    return parts.join("\n\n");
  }

  if (!compile) {
    return "No compile output";
  }

  if (tab === "diagnostics") {
    return [compile.summary, compile.diagnostics || "No diagnostics"]
      .filter(Boolean)
      .join("\n");
  }

  return compile[tab] || "<not available>";
}

function commandErrorResponse(error: unknown): CompileResponse {
  return {
    success: false,
    summary: "",
    diagnostics: error instanceof Error ? error.message : String(error),
    tokens: "<not available>",
    cst: "<not available>",
    ast: "<not available>",
    hir: "<not available>",
    typecheck: "<not available>",
    thir: "<not available>",
    ir: "<not available>",
  };
}

export default App;
