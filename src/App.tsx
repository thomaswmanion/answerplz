import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { DisplayPreview } from "./DisplayPreview";
import { Overlay } from "./Overlay";
import { Setup } from "./Setup";
import "./Overlay.css";

function App() {
  const [windowLabel, setWindowLabel] = useState<string | null>(null);

  useEffect(() => {
    setWindowLabel(getCurrentWindow().label);
  }, []);

  if (!windowLabel) {
    return null;
  }

  if (windowLabel === "overlay") {
    return <Overlay />;
  }

  if (windowLabel.startsWith("monitor-preview-")) {
    return <DisplayPreview windowLabel={windowLabel} />;
  }

  return <Setup />;
}

export default App;
