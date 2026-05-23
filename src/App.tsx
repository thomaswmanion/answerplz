import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
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

  return <Setup />;
}

export default App;
