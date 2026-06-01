// React 18's createRoot API — the modern way to mount a React app.
// Older React used ReactDOM.render() which is now deprecated.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./App.css";

// Find the <div id="root"> element in index.html and mount React into it.
// The "!" tells TypeScript we are certain this element exists —
// if it somehow does not, React will throw a clear error at startup.
createRoot(document.getElementById("root")!).render(
  // StrictMode runs each component twice in development only.
  // This surfaces bugs caused by side effects in render functions.
  // It has zero effect in production builds.
  <StrictMode>
    <App />
  </StrictMode>
);