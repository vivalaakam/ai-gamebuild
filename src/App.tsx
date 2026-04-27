import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Editor, { type OnMount } from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import "./App.css";

type ScriptUnit = {
  id: string;
  name: string;
  source: string;
  dependencies: string[];
  bindings: string[];
};

type StructUnit = {
  id: string;
  name: string;
  source: string;
};

type InputAction = {
  id: string;
  label: string;
  key_code: string;
};

type Project = {
  id: string;
  name: string;
  scripts: ScriptUnit[];
  structs: StructUnit[];
  input_actions: InputAction[];
};

type FileSelection = {
  kind: "script" | "struct";
  id: string;
};

type VisibleTile = {
  tile_id: number;
  screen_x: number;
  screen_y: number;
};

type VisibleEntity = {
  id: number;
  tile_id: number;
  screen_x: number;
  screen_y: number;
  is_active: boolean;
  opacity: number;
};

type FrameView = {
  virtual_width: number;
  virtual_height: number;
  tile_size: number;
  visible_tiles: VisibleTile[];
  visible_entities: VisibleEntity[];
  logs: string[];
};

type ValidationResult = {
  valid: boolean;
  error: string | null;
};

type SavedHashMap = Record<string, number>;

const palette = ["#101820", "#243c2f", "#f2c14e", "#1b263b", "#3a7d44", "#e76f51", "#2a9d8f", "#e9c46a"];

function App() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const pressedKeys = useRef(new Set<string>());
  const lastFrame = useRef(performance.now());
  const selectionMarker = useRef<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [selectedFile, setSelectedFile] = useState<FileSelection>({ kind: "script", id: "init" });
  const [source, setSource] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [status, setStatus] = useState("loading runtime");
  const [validation, setValidation] = useState<ValidationResult>({ valid: true, error: null });
  const [captureActionId, setCaptureActionId] = useState<string | null>(null);
  const [savedHashes, setSavedHashes] = useState<SavedHashMap>({});

  const selectedUnit =
    selectedFile.kind === "script"
      ? project?.scripts.find((script) => script.id === selectedFile.id)
      : project?.structs.find((unit) => unit.id === selectedFile.id);

  useEffect(() => {
    let cancelled = false;
    invoke<Project>("load_project")
      .then((loaded) => {
        if (cancelled) return;
        setProject(loaded);
        setSavedHashes(buildSavedHashes(loaded));
        const firstScript = loaded.scripts[0];
        setSelectedFile({ kind: "script", id: firstScript?.id ?? "" });
        setSource(firstScript?.source ?? "");
        setStatus("runtime ready");
      })
      .catch((error) => setStatus(String(error)));
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!selectedUnit) return;
    setSource(selectedUnit.source);
  }, [selectedFile.kind, selectedFile.id, selectedUnit?.source]);

  useEffect(() => {
    const handle = window.setTimeout(async () => {
      if (!source.trim()) {
        setValidation({ valid: false, error: "file is empty" });
        return;
      }
      const result = await invoke<ValidationResult>("validate_script", { source });
      setValidation(result);
    }, 250);
    return () => window.clearTimeout(handle);
  }, [source]);

  useEffect(() => {
    if (!selectionMarker.current) return;
    window.setTimeout(() => selectMarker(selectionMarker.current), 0);
  }, [source, selectedFile.kind, selectedFile.id]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (editorRef.current?.hasTextFocus()) {
        return;
      }
      if (captureActionId) {
        event.preventDefault();
        updateInputAction(captureActionId, event.code);
        return;
      }

      pressedKeys.current.add(event.code);
      if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space", "Tab"].includes(event.code)) {
        event.preventDefault();
      }
    };
    const onKeyUp = (event: KeyboardEvent) => {
      if (editorRef.current?.hasTextFocus()) {
        return;
      }
      pressedKeys.current.delete(event.code);
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, [captureActionId]);

  useEffect(() => {
    let alive = true;
    let busy = false;
    const tick = async (now: number) => {
      if (!alive) return;
      if (!busy) {
        busy = true;
        const delta = Math.min((now - lastFrame.current) / 1000, 0.1);
        lastFrame.current = now;
        try {
          const frame = await invoke<FrameView>("run_frame", {
            pressedKeys: Array.from(pressedKeys.current),
            delta,
          });
          drawFrame(frame);
          if (frame.logs.length > 0) {
            setLogs((existing) => [...frame.logs, ...existing].slice(0, 80));
          }
        } catch (error) {
          setStatus(String(error));
        } finally {
          busy = false;
        }
      }
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
    return () => {
      alive = false;
    };
  }, []);

  const handleEditorMount: OnMount = (editorInstance) => {
    editorRef.current = editorInstance;
    selectMarker(selectionMarker.current);
  };

  function selectMarker(marker: string | null) {
    const editorInstance = editorRef.current;
    if (!editorInstance || !marker) return;
    const index = source.indexOf(marker);
    if (index < 0) return;
    const model = editorInstance.getModel();
    if (!model) return;
    const position = model.getPositionAt(index);
    selectionMarker.current = null;
    editorInstance.focus();
    editorInstance.setSelection({
      startLineNumber: position.lineNumber,
      startColumn: position.column,
      endLineNumber: position.lineNumber,
      endColumn: position.column + marker.length,
    });
  }

  function drawFrame(frame: FrameView) {
    const canvas = canvasRef.current;
    const context = canvas?.getContext("2d");
    if (!canvas || !context) return;

    context.fillStyle = "#101820";
    context.fillRect(0, 0, canvas.width, canvas.height);

    const scale = canvas.width / frame.virtual_width;
    for (const tile of frame.visible_tiles) {
      context.fillStyle = palette[tile.tile_id % palette.length];
      context.fillRect(tile.screen_x * scale, tile.screen_y * scale, frame.tile_size * scale, frame.tile_size * scale);
      context.strokeStyle = "rgba(255,255,255,0.06)";
      context.strokeRect(tile.screen_x * scale, tile.screen_y * scale, frame.tile_size * scale, frame.tile_size * scale);
    }

    for (const entity of frame.visible_entities) {
      context.save();
      context.globalAlpha = entity.opacity;
      context.fillStyle = palette[entity.tile_id % palette.length];
      context.fillRect(
        entity.screen_x * scale + 4,
        entity.screen_y * scale + 4,
        frame.tile_size * scale - 8,
        frame.tile_size * scale - 8,
      );
      if (entity.is_active) {
        context.strokeStyle = "#fff7e8";
        context.lineWidth = 3;
        context.strokeRect(
          entity.screen_x * scale + 5,
          entity.screen_y * scale + 5,
          frame.tile_size * scale - 10,
          frame.tile_size * scale - 10,
        );
      }
      context.fillStyle = "#101820";
      context.fillText(String(entity.id), entity.screen_x * scale + 10, entity.screen_y * scale + 22);
      context.restore();
    }
  }

  async function applyFile(): Promise<Project | null> {
    const result = await invoke<ValidationResult>("validate_script", { source });
    setValidation(result);
    if (!result.valid) {
      setStatus("file has syntax errors");
      return null;
    }

    const updated =
      selectedFile.kind === "script"
        ? await invoke<Project>("update_script", { scriptId: selectedFile.id, source })
        : await invoke<Project>("update_struct", { structId: selectedFile.id, source });
    setProject(updated);
    setStatus(`${selectedFile.kind} applied in memory`);
    return updated;
  }

  async function saveProject() {
    if (!project) return;
    let snapshotProject: Project | null = project;
    if (selectedUnit && isUnitDirty(selectedFile.kind, selectedUnit)) {
      snapshotProject = await applyFile();
      if (!snapshotProject) return;
    }
    const result = await invoke<{ snapshot_bytes: number }>("save_project");
    setStatus(`saved snapshot: ${result.snapshot_bytes} bytes`);
    if (snapshotProject) {
      setSavedHashes(buildSavedHashes(snapshotProject));
    }
  }

  async function createScript() {
    const updated = await invoke<Project>("create_script");
    const created = updated.scripts[updated.scripts.length - 1];
    selectionMarker.current = "name_fn";
    setProject(updated);
    setSelectedFile({ kind: "script", id: created?.id ?? "" });
    setSource(created?.source ?? "");
    setStatus(`created ${created?.name ?? "script"}`);
  }

  async function createStruct() {
    const updated = await invoke<Project>("create_struct");
    const created = updated.structs[updated.structs.length - 1];
    selectionMarker.current = "make_name";
    setProject(updated);
    setSelectedFile({ kind: "struct", id: created?.id ?? "" });
    setSource(created?.source ?? "");
    setStatus(`created ${created?.name ?? "struct"}`);
  }

  async function updateInputAction(actionId: string, keyCode: string) {
    const updated = await invoke<Project>("update_input_action", {
      actionId,
      keyCode,
    });
    pressedKeys.current.clear();
    setProject(updated);
    setCaptureActionId(null);
    setStatus(`bound ${actionId} to ${keyCode}`);
  }

  async function resetInputActions() {
    const updated = await invoke<Project>("reset_input_actions");
    pressedKeys.current.clear();
    setProject(updated);
    setCaptureActionId(null);
    setStatus("input bindings reset");
  }

  function buildSavedHashes(nextProject: Project): SavedHashMap {
    const hashes: SavedHashMap = {};
    for (const script of nextProject.scripts) {
      hashes[keyFor("script", script.id)] = hashSource(script.source);
    }
    for (const unit of nextProject.structs) {
      hashes[keyFor("struct", unit.id)] = hashSource(unit.source);
    }
    return hashes;
  }

  function keyFor(kind: FileSelection["kind"], id: string) {
    return `${kind}:${id}`;
  }

  function hashSource(value: string) {
    let hash = 5381;
    for (let i = 0; i < value.length; i += 1) {
      hash = (hash * 33) ^ value.charCodeAt(i);
    }
    return hash >>> 0;
  }

  function isUnitDirty(kind: FileSelection["kind"], unit: ScriptUnit | StructUnit) {
    const key = keyFor(kind, unit.id);
    const base = savedHashes[key];
    const effectiveSource =
      selectedFile.kind === kind && selectedFile.id === unit.id ? source : unit.source;
    if (base === undefined) return true;
    return base !== hashSource(effectiveSource);
  }

  const hasDirty =
    (project?.scripts.some((script) => isUnitDirty("script", script)) ?? false) ||
    (project?.structs.some((unit) => isUnitDirty("struct", unit)) ?? false);

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <h1>{project?.name ?? "Fantasy Console MVP"}</h1>
        </div>
        <div className="actions">
          <button onClick={applyFile} disabled={!validation.valid}>
            Apply File
          </button>
          <button onClick={saveProject} disabled={!hasDirty}>
            Save Project
          </button>
        </div>
      </header>

      <section className="workspace">
        <aside className="left-rail">
          <section className="script-tree">
            <div className="panel-heading">
              <h2>Scripts</h2>
              <button className="compact" onClick={createScript}>
                New
              </button>
            </div>
            {project?.scripts.map((script) => (
              (() => {
                const dirty = isUnitDirty("script", script);
                return (
              <button
                className={selectedFile.kind === "script" && script.id === selectedFile.id ? "selected" : ""}
                key={script.id}
                onClick={() => setSelectedFile({ kind: "script", id: script.id })}
              >
                <span>{script.name}{dirty ? " *" : ""}</span>
                <small>{script.bindings.length > 0 ? script.bindings.join(", ") : "library"}</small>
              </button>
                );
              })()
            ))}
          </section>

          <section className="struct-tree">
            <div className="panel-heading">
              <h2>Structs</h2>
              <button className="compact" onClick={createStruct}>
                New
              </button>
            </div>
            {project?.structs.map((unit) => (
              (() => {
                const dirty = isUnitDirty("struct", unit);
                return (
              <button
                className={selectedFile.kind === "struct" && unit.id === selectedFile.id ? "selected" : ""}
                key={unit.id}
                onClick={() => setSelectedFile({ kind: "struct", id: unit.id })}
              >
                <span>{unit.name}{dirty ? " *" : ""}</span>
                <small>rhai factory</small>
              </button>
                );
              })()
            ))}
          </section>

          <section className="events-tree">
            <div className="panel-heading">
              <h2>Events</h2>
              <button className="compact" onClick={resetInputActions}>
                Reset
              </button>
            </div>
            {project?.input_actions.map((action) => (
              <button
                className={captureActionId === action.id ? "selected listening" : ""}
                key={action.id}
                onClick={() => setCaptureActionId(action.id)}
              >
                <span>{action.label}</span>
                <small>{captureActionId === action.id ? "press any key" : action.key_code}</small>
              </button>
            ))}
          </section>
        </aside>

        <section className="editor">
          <div className="panel-heading">
            <h2>{selectedUnit?.name ?? "Rhai Editor"}</h2>
            <span className={validation.valid ? "validation ok" : "validation error"}>
              {validation.valid ? "valid" : "invalid"}
            </span>
          </div>
          <div className="editor-frame">
            <Editor
              height="100%"
              language="rhai"
              theme="vs-dark"
              value={source}
              onMount={handleEditorMount}
              onChange={(value) => setSource(value ?? "")}
              options={{
                automaticLayout: true,
                fontFamily: '"SFMono-Regular", "Cascadia Code", monospace',
                fontSize: 14,
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                tabSize: 4,
                wordWrap: "on",
              }}
            />
          </div>
          {!validation.valid && <p className="validation-message">{validation.error}</p>}
        </section>

        <section className="preview">
          <h2>Runtime Preview</h2>
          <canvas ref={canvasRef} width={720} height={720} />
          <p className="hint">`spawn_entity`, `set_entity_pos`, `get_active`, `next_active` are Rhai library functions.</p>
        </section>

        <section className="console">
          <div className="panel-heading">
            <h2>Console</h2>
            <button className="compact" onClick={() => setLogs([])}>
              Clear
            </button>
          </div>
          <p className="status">{status}</p>
          {logs.map((line, index) => (
            <code key={`${line}-${index}`}>{line}</code>
          ))}
        </section>
      </section>
    </main>
  );
}

export default App;
