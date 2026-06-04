import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Relative base keeps the build portable: works at site root and under /brocolis/.
export default defineConfig({
  base: "./",
  plugins: [react()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
