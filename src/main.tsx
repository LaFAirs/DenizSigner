import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { LogProvider } from "./components/LogContext";
import { Toaster } from "sonner";
import "./i18n";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <LogProvider>
      <App />
      <Toaster richColors position="bottom-center" />
    </LogProvider>
  </React.StrictMode>,
);
