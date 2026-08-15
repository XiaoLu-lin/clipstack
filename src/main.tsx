import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Settings from "./Settings";

// 用 hash 区分设置窗口（query 参数在 Windows 的 tauri.localhost 下容易丢失）
const isSettings = window.location.hash.includes("settings");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{isSettings ? <Settings /> : <App />}</React.StrictMode>,
);
