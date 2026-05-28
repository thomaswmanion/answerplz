import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import "./App.css";

const windowLabel = getCurrentWindow().label;
if (windowLabel === "overlay") {
  document.documentElement.classList.add("overlay-window");
}
if (windowLabel.startsWith("monitor-preview-")) {
  document.documentElement.classList.add("display-preview-window");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
