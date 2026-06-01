import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],

  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Tell Vite's file watcher to completely ignore the Rust
      // build output folder. On Windows, compiled .dll and .exe
      // files inside target/ get locked by the OS during compilation.
      // If Vite tries to watch them it crashes with EBUSY.
      // Tauri handles watching src-tauri/src itself — Vite only
      // needs to watch the frontend src/ folder.
      ignored: ["**/src-tauri/target/**"],
    },
  },

  envPrefix: [
    "VITE_",
    "TAURI_ENV_*",
  ],

  build: {
    target: "chrome105",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});