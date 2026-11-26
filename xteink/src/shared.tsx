import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

// The one target device (the backend uses the same profile in `xtch_core::Device`).
export const DEVICE = { label: "XTEink X4 Pro", w: 480, h: 800, screen: "UC8179" };

export const fmtSize = (n: number) =>
  n < 1024
    ? `${n} B`
    : n < 1024 * 1024
    ? `${(n / 1024).toFixed(1)} KB`
    : `${(n / 1024 / 1024).toFixed(1)} MB`;

export const baseName = (p: string) => p.split(/[\\/]/).pop() || p;

export function DeviceInfo() {
  return (
    <div className="panel">
      <div className="panel-h">TARGET DEVICE</div>
      <div className="hint">
        {DEVICE.label} · {DEVICE.w}×{DEVICE.h} · {DEVICE.screen} screen · CrossPoint 2.0
      </div>
    </div>
  );
}

/** Native window drag-and-drop, delivered only while `active` (the visible tab). */
export function useWindowDrop(active: boolean, onPaths: (paths: string[]) => void) {
  const [dragging, setDragging] = useState(false);
  const activeRef = useRef(active);
  activeRef.current = active;
  const cbRef = useRef(onPaths);
  cbRef.current = onPaths;
  useEffect(() => {
    const un = getCurrentWebview().onDragDropEvent((event) => {
      if (!activeRef.current) return;
      const t = event.payload.type;
      if (t === "enter" || t === "over") setDragging(true);
      else if (t === "leave") setDragging(false);
      else if (t === "drop") {
        setDragging(false);
        if (event.payload.paths?.length) cbRef.current(event.payload.paths);
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, []);
  return dragging;
}

/** Close a popup menu on any outside click or Escape. */
export function useDismiss(open: boolean, close: () => void) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("click", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [open, close]);
}
