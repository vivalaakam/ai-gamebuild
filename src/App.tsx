import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type ScriptUnit = {
  id: string;
  name: string;
  source: string;
  dependencies: string[];
  bindings: string[];
};

type Project = {
  id: string;
  name: string;
  scripts: ScriptUnit[];
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
};

type FrameView = {
  virtual_width: number;
  virtual_height: number;
  tile_size: number;
  visible_tiles: VisibleTile[];
  visible_entities: VisibleEntity[];
  logs: string[];
};

const palette = [
  "#101820",
  "#243c2f",
  "#f2c14e",
  "#1b263b",
  "#3a7d44",
  "#e76f51",
  "#2a9d8f",
  "#e9c46a",
];

function App() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const pressedKeys = useRef(new Set<string>());
  const lastFrame = useRef(performance.now());
  const [project, setProject] = useState<Project | null>(null);
  const [selectedScriptId, setSelectedScriptId] = useState("main");
  const [source, setSource] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [status, setStatus] = useState("loading runtime");

  const selectedScript = project?.scripts.find((script) => script.id === selectedScriptId);

  useEffect(() => {
    let cancelled = false;
    invoke<Project>("load_project")
      .then((loaded) => {
        if (cancelled) return;
        setProject(loaded);
        const firstScript = loaded.scripts[0];
        setSelectedScriptId(firstScript?.id ?? "");
        setSource(firstScript?.source ?? "");
        setStatus("runtime ready");
      })
      .catch((error) => setStatus(String(error)));
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!selectedScript) return;
    setSource(selectedScript.source);
  }, [selectedScriptId, selectedScript?.source]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      pressedKeys.current.add(event.code);
      if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space"].includes(event.code)) {
        event.preventDefault();
      }
    };
    const onKeyUp = (event: KeyboardEvent) => pressedKeys.current.delete(event.code);
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, []);

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

  function drawFrame(frame: FrameView) {
    const canvas = canvasRef.current;
    const context = canvas?.getContext("2d");
    if (!canvas || !context) return;

    context.fillStyle = "#101820";
    context.fillRect(0, 0, canvas.width, canvas.height);

    const scale = canvas.width / frame.virtual_width;
    for (const tile of frame.visible_tiles) {
      context.fillStyle = palette[tile.tile_id % palette.length];
      context.fillRect(
        tile.screen_x * scale,
        tile.screen_y * scale,
        frame.tile_size * scale,
        frame.tile_size * scale,
      );
      context.strokeStyle = "rgba(255,255,255,0.06)";
      context.strokeRect(
        tile.screen_x * scale,
        tile.screen_y * scale,
        frame.tile_size * scale,
        frame.tile_size * scale,
      );
    }

    for (const entity of frame.visible_entities) {
      context.fillStyle = palette[entity.tile_id % palette.length];
      context.fillRect(
        entity.screen_x * scale + 4,
        entity.screen_y * scale + 4,
        frame.tile_size * scale - 8,
        frame.tile_size * scale - 8,
      );
      context.fillStyle = "#101820";
      context.fillText(String(entity.id), entity.screen_x * scale + 10, entity.screen_y * scale + 22);
    }
  }

  async function applyScript() {
    if (!selectedScriptId) return;
    const updated = await invoke<Project>("update_script", {
      scriptId: selectedScriptId,
      source,
    });
    setProject(updated);
    setStatus("script applied in memory");
  }

  async function saveProject() {
    const result = await invoke<{ snapshot_bytes: number }>("save_project");
    setStatus(`saved snapshot: ${result.snapshot_bytes} bytes`);
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">Rust + Tauri + Rhai + SQLite</p>
          <h1>{project?.name ?? "Fantasy Console MVP"}</h1>
        </div>
        <div className="actions">
          <button onClick={applyScript}>Apply Script</button>
          <button onClick={saveProject}>Save Snapshot</button>
        </div>
      </header>

      <section className="workspace">
        <aside className="script-tree">
          <h2>Scripts</h2>
          {project?.scripts.map((script) => (
            <button
              className={script.id === selectedScriptId ? "selected" : ""}
              key={script.id}
              onClick={() => setSelectedScriptId(script.id)}
            >
              <span>{script.name}</span>
              <small>{script.bindings.join(", ")}</small>
            </button>
          ))}
        </aside>

        <section className="editor">
          <h2>Rhai Editor</h2>
          <textarea value={source} onChange={(event) => setSource(event.currentTarget.value)} />
        </section>

        <section className="preview">
          <h2>Runtime Preview</h2>
          <canvas ref={canvasRef} width={720} height={720} />
          <p className="hint">Arrow keys update actions. Space paints a tile. Enter spawns an entity.</p>
        </section>

        <section className="console">
          <h2>Console</h2>
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
