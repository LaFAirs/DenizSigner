import { createContext, useContext, useEffect, useState, ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { sanitizeForLog } from "../lib/errors";

export interface LogRecord {
  level: number; // 1 trace 2 debug 3 info 4 warn 5 error
  message: string;
  target?: string;
  timestamp: string;
}

const Ctx = createContext<{ logs: LogRecord[]; clear: () => void }>({ logs: [], clear: () => {} });

export function LogProvider({ children }: { children: ReactNode }) {
  const [logs, setLogs] = useState<LogRecord[]>([]);
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void listen<LogRecord>("log-record", (e) => {
      const rec = { ...e.payload, message: sanitizeForLog(e.payload.message) };
      setLogs((old) => [...old.slice(-1999), rec]);
    }).then((f) => {
      unlisten = f;
    });
    return () => unlisten?.();
  }, []);
  return <Ctx.Provider value={{ logs, clear: () => setLogs([]) }}>{children}</Ctx.Provider>;
}

export function useLogs() {
  return useContext(Ctx);
}
