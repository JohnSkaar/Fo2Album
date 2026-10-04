import { useCallback, useEffect, useRef, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { StartScreen } from "./components/StartScreen";
import { Toast } from "./components/Toast";
import { tekster } from "./tekster";

const TOAST_MS = 2600;

export function App() {
  const [toast, setToast] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const showToast = useCallback((message: string) => {
    setToast(message);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setToast(null), TOAST_MS);
  }, []);
  useEffect(() => () => clearTimeout(timer.current), []);

  // Mappevalg kommer i M1.
  const comingSoon = () => showToast(tekster.melding.kommerSnart);

  return (
    <div className="app">
      <Sidebar onAddFolder={comingSoon} />
      <main className="main">
        <StartScreen onPickSource={comingSoon} />
      </main>
      <Toast message={toast} />
    </div>
  );
}
