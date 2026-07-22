import { defineConfig } from "vite";
import nori from "@nori/vite-plugin";

export default defineConfig({
  plugins: [nori()],
  build: {
    outDir: "dist",
    target: "esnext"
  }
});
