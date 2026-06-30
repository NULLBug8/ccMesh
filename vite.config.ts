import path from "path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) return;
          if (id.includes("@codemirror") || id.includes("@uiw/react-codemirror")) return "editor-vendor";
          if (id.includes("@dnd-kit")) return "dnd-vendor";
          if (id.includes("@tanstack/react-query")) return "query-vendor";
          if (id.includes("radix-ui") || id.includes("lucide-react")) return "ui-vendor";
          if (id.includes("/react-dom/") || id.includes("/react/") || id.includes("/scheduler/")) return "react-vendor";
        },
      },
    },
    chunkSizeWarningLimit: 700,
  },
  server: {
    port: 5173,
    strictPort: true,
  },
});