import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { defineConfig, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";

function cfasimWasm(modelDir = "model"): Plugin {
  return {
    name: "cfasim-wasm",
    configResolved(config) {
      const name = basename(config.root);
      const outDir = resolve(config.root, "public", "wasm", name);
      execSync(`wasm-pack build ${modelDir} --target web --out-dir ${outDir}`, {
        cwd: config.root,
        stdio: "inherit",
      });
    },
  };
}

function uiParamsAsset(): Plugin {
  const filename = "ui-params.toml";
  let sourcePath = "";
  return {
    name: "ui-params-asset",
    configResolved(config) {
      sourcePath = resolve(config.root, "src/config", filename);
    },
    configureServer(server) {
      server.middlewares.use(`/${filename}`, (_req, res) => {
        res.setHeader("Content-Type", "application/toml");
        res.setHeader("Cache-Control", "no-store");
        res.end(readFileSync(sourcePath));
      });
    },
    handleHotUpdate({ file, server }) {
      if (file === sourcePath) {
        server.ws.send({ type: "full-reload" });
        return [];
      }
    },
    generateBundle() {
      this.emitFile({
        type: "asset",
        fileName: filename,
        source: readFileSync(sourcePath),
      });
    },
  };
}

export default defineConfig({
  base: process.env.BASE_PATH ?? "/",
  plugins: [vue(), uiParamsAsset(), cfasimWasm()],
});
