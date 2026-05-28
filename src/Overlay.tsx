import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { copyToClipboard } from "./copyToClipboard";
import {
  requestClipboardAnswer,
  requestQuestionAnswer,
  requestScreenshotAnswer,
} from "./overlayActions";
import { fitOverlayWindowToContent } from "./overlayWindowLayout";
import { startOverlayResize } from "./overlayResize";
import { useOverlayHitTest } from "./useOverlayHitTest";
import { releaseClickThrough } from "./windowTraits";
import "./Overlay.css"; // scoped via html.overlay-window in main.tsx

type OverlayMode = "idle" | "ask" | "loading" | "answer" | "error";

export function Overlay() {
  const [mode, setMode] = useState<OverlayMode>("idle");
  const [question, setQuestion] = useState("");
  const [text, setText] = useState("");
  const [copyHint, setCopyHint] = useState<string | null>(null);
  const askInputRef = useRef<HTMLInputElement>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  useOverlayHitTest(rootRef);

  useEffect(() => {
    const root = rootRef.current;
    if (root) {
      void fitOverlayWindowToContent(root);
    }
    return () => {
      void releaseClickThrough();
    };
  }, []);

  useEffect(() => {
    const root = rootRef.current;
    if (root) {
      void fitOverlayWindowToContent(root, { growOnly: true });
    }
  }, [mode]);

  useEffect(() => {
    if (mode === "ask") {
      askInputRef.current?.focus();
    }
  }, [mode]);

  useEffect(() => {
    const unlisten = listen<{ answer: string; isError: boolean }>(
      "overlay-answer",
      (event) => {
        setText(event.payload.answer);
        setMode(event.payload.isError ? "error" : "answer");
      },
    );
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  function resetToIdle() {
    setMode("idle");
    setQuestion("");
    setText("");
    setCopyHint(null);
  }

  async function copyAnswer() {
    if (!text.trim() || mode === "error") {
      return;
    }
    try {
      await copyToClipboard(text);
      setCopyHint("Copied!");
      window.setTimeout(() => setCopyHint(null), 1500);
    } catch (err) {
      setCopyHint(String(err));
    }
  }

  async function runRequest(fetchAnswer: () => Promise<string>) {
    setMode("loading");
    setText("");
    try {
      const answer = await fetchAnswer();
      setText(answer);
      setMode("answer");
    } catch (err) {
      setText(String(err));
      setMode("error");
    }
  }

  function openAsk() {
    setQuestion("");
    setText("");
    setMode("ask");
  }

  async function submitQuestion() {
    const trimmed = question.trim();
    if (!trimmed) {
      setText("Type a question first.");
      setMode("error");
      return;
    }
    await runRequest(() => requestQuestionAnswer(trimmed));
  }

  function onAskKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      void submitQuestion();
    }
    if (e.key === "Escape") {
      resetToIdle();
    }
  }

  return (
    <div className="overlay-root" ref={rootRef}>
      <div className="overlay-bar">
        <span className="overlay-bar__grip" data-tauri-drag-region aria-hidden>
          ⋮⋮
        </span>
        <button
          type="button"
          className="overlay-bar__pill"
          onClick={() => void runRequest(requestScreenshotAnswer)}
          disabled={mode === "loading"}
          title="Screenshot and answer"
        >
          {mode === "loading" ? "…" : "answer plz"}
        </button>
        <button
          type="button"
          className="overlay-bar__icon overlay-bar__action"
          title="Type a question"
          onClick={openAsk}
          disabled={mode === "loading"}
        >
          ?
        </button>
        <button
          type="button"
          className="overlay-bar__icon overlay-bar__action"
          title="Answer from clipboard"
          onClick={() => void runRequest(requestClipboardAnswer)}
          disabled={mode === "loading"}
        >
          ⎘
        </button>
        <button
          type="button"
          className="overlay-bar__icon"
          title="Settings"
          onClick={() => void invoke("open_setup_window")}
          disabled={mode === "loading"}
        >
          ⚙
        </button>
        <button
          type="button"
          className="overlay-bar__icon overlay-bar__close"
          title="Hide overlay (use tray to quit)"
          onClick={async () => {
            await releaseClickThrough();
            await invoke("hide_overlay");
            resetToIdle();
          }}
          disabled={mode === "loading"}
        >
          ×
        </button>
      </div>

      {mode === "ask" && (
        <form
          className="overlay-ask"
          onSubmit={(e) => {
            e.preventDefault();
            void submitQuestion();
          }}
        >
          <input
            ref={askInputRef}
            type="text"
            className="overlay-ask__input"
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            onKeyDown={onAskKeyDown}
            placeholder="Ask anything…"
            disabled={mode !== "ask"}
          />
          <button type="submit" className="overlay-ask__submit" disabled={mode !== "ask"}>
            Go
          </button>
          <button
            type="button"
            className="overlay-ask__cancel"
            onClick={resetToIdle}
            aria-label="Cancel"
          >
            ×
          </button>
        </form>
      )}

      {(mode === "answer" || mode === "error") && (
        <div
          className={`overlay-answer ${mode === "error" ? "overlay-answer--error" : ""}`}
          role="status"
        >
          <p className="overlay-answer__text">{text}</p>
          {mode === "answer" && (
            <button
              type="button"
              className="overlay-answer__copy"
              title="Copy answer"
              onClick={() => void copyAnswer()}
              aria-label="Copy answer"
            >
              {copyHint ?? "⎘"}
            </button>
          )}
          <button
            type="button"
            className="overlay-answer__dismiss"
            onClick={resetToIdle}
            aria-label="Dismiss"
          >
            ×
          </button>
        </div>
      )}

      <button
        type="button"
        className="overlay-resize-handle"
        title="Resize (drag)"
        aria-label="Resize window"
        onPointerDown={(e) => {
          e.preventDefault();
          e.stopPropagation();
          startOverlayResize();
        }}
      />
    </div>
  );
}
