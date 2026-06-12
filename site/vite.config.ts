import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Relative base keeps the build portable: works at site root and under /brocolis/.
export default defineConfig({
  base: "./",
  plugins: [react()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      // Multi-page : la landing (index) + la page de téléchargement.
      // Chemins relatifs à la racine du projet (site/), résolus par Vite.
      input: {
        main: "index.html",
        telecharger: "telecharger.html",
      },
    },
  },
});
