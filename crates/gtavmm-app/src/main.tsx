import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./i18n";
import { applyStartupTheme } from "./lib/theme";

// Resolve the theme before React mounts so the first paint is already correct
// and there is no flash of the wrong palette.
applyStartupTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
