import { fileURLToPath, URL } from "node:url";

import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const proofRoot = fileURLToPath(new URL("../../src/security-proof", import.meta.url));

export default defineConfig({
  root: proofRoot,
  plugins: [vue()],
  build: {
    outDir: fileURLToPath(new URL("../../dist/security-proof", import.meta.url)),
    emptyOutDir: true,
    rollupOptions: {
      input: fileURLToPath(new URL("../../src/security-proof/index.html", import.meta.url)),
    },
  },
  server: {
    port: 1422,
    strictPort: true,
  },
});
