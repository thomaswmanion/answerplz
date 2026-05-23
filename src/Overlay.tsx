import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Overlay.css";

type OverlayState = "idle" | "loading" | "answer" | "error";

export function Overlay() {
  const [state, setState] = useState<OverlayState>("idle");
  const [text, setText] = useState("");

  async function onAnswer() {
    setState("loading");
    setText("");
    try {
      const res = await invoke<{ answer: string }>("capture_and_answer");
      setText(res.answer);
      setState("answer");
    } catch (err) {
      setText(String(err));
      setState("error");
    }
  }

  function dismissAnswer() {
    setState("idle");
    setText("");
  }

  async function onQuit() {
    await invoke("quit_app");
  }

  async function onReconfigure() {
    await invoke("open_setup_window");
  }

  return (
    <div className="overlay-root">
      <div className="overlay-bar">
        <span className="overlay-bar__grip" data-tauri-drag-region aria-hidden>
          ⋮⋮
        </span>
        <button
          type="button"
          className="overlay-bar__pill"
          onClick={onAnswer}
          disabled={state === "loading"}
        >
          {state === "loading" ? "…" : "answer plz"}
        </button>
        <button
          type="button"
          className="overlay-bar__icon"
          title="Settings"
          onClick={onReconfigure}
        >
          ⚙
        </button>
        <button
          type="button"
          className="overlay-bar__icon overlay-bar__close"
          title="Quit"
          onClick={onQuit}
        >
          ×
        </button>
      </div>

      {(state === "answer" || state === "error") && (
        <div
          className={`overlay-answer ${state === "error" ? "overlay-answer--error" : ""}`}
          role="status"
        >
          <p className="overlay-answer__text">{text}</p>
          <button
            type="button"
            className="overlay-answer__dismiss"
            onClick={dismissAnswer}
          >
            ×
          </button>
        </div>
      )}
    </div>
  );
}
