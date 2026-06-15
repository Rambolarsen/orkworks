import { useEffect, useState } from "react";
import LeftSidebar from "./components/LeftSidebar";
import CenterPanel from "./components/CenterPanel";
import RightSidebar from "./components/RightSidebar";

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
    };
  }
}

function App() {
  const [backendStatus, setBackendStatus] = useState<string>("connecting…");

  useEffect(() => {
    let cancelled = false;

    async function checkHealth() {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        for (let i = 0; i < 30; i++) {
          try {
            const resp = await fetch(`${baseUrl}/health`);
            if (resp.ok) {
              if (!cancelled) setBackendStatus("connected");
              return;
            }
          } catch {
            await new Promise((r) => setTimeout(r, 500));
          }
        }
        if (!cancelled) setBackendStatus("unreachable");
      } catch {
        if (!cancelled) setBackendStatus("unreachable");
      }
    }

    checkHealth();
    return () => { cancelled = true; };
  }, []);

  return (
    <div className="app-shell">
      <div className="titlebar">
        <span className="titlebar-text">OrkWorks</span>
        <span className={`status-badge ${backendStatus === "connected" ? "ok" : "warn"}`}>
          {backendStatus}
        </span>
      </div>
      <div className="app-layout">
        <aside className="panel left-sidebar">
          <LeftSidebar />
        </aside>
        <main className="panel center-panel">
          <CenterPanel backendStatus={backendStatus} />
        </main>
        <aside className="panel right-sidebar">
          <RightSidebar />
        </aside>
      </div>
    </div>
  );
}

export default App;
