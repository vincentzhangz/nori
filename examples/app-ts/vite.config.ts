import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import nori from "@nori/vite-plugin";

export default defineConfig({
  plugins: [nori(), tailwindcss()],
  build: {
    outDir: "dist",
    target: "esnext"
  }
});
