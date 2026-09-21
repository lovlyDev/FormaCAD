import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter, Routes, Route } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import * as Tooltip from "@radix-ui/react-tooltip";
import App from "./app/App";
import "./styles.css";
import { getLocale, setLocale } from "./i18n";
import { applyTheme } from "./lib/theme";
import { restorePreferences, persist } from "./lib/persistence";
async function start() {
await restorePreferences().catch(console.error);
setLocale(localStorage.getItem("forma.locale") === "en" ? "en" : "ru");
if (!location.hash || location.hash === "#/") {
  const saved = localStorage.getItem("forma.ui.route");
  if (saved && /^#\/project\/[a-zA-Z0-9-]+$/.test(saved)) location.hash = saved;
}
window.addEventListener("hashchange", () => persist("forma.ui.route", location.hash));
document.documentElement.lang = getLocale();
applyTheme(localStorage.getItem("forma.theme") ?? "dark");
const client = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
});
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={client}>
      <Tooltip.Provider delayDuration={300}>
        <HashRouter>
          <Routes>
            <Route path="/" element={<App />} />
            <Route path="/project/:projectId" element={<App />} />
          </Routes>
        </HashRouter>
      </Tooltip.Provider>
    </QueryClientProvider>
  </React.StrictMode>,
);
}
void start();
