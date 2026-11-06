import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { defineConfig, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";
import MarkdownIt from "markdown-it";
import { katex } from "@mdit/plugin-katex";

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

// Heading ids as the VitePress site makes them, so the doc's in-page links
// resolve to the same anchors in both.
function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9 _-]+/g, "")
    .trim()
    .replace(/\s+/g, "-");
}

// Imports a Markdown file as a rendered HTML string. Math is rendered to
// KaTeX markup at build time, so the browser only needs katex's stylesheet.
function markdownHtml(): Plugin {
  const md = new MarkdownIt({ html: true, linkify: true }).use(katex, {
    throwOnError: true,
  });

  md.core.ruler.push("heading_ids", (state) => {
    for (let i = 0; i < state.tokens.length; i++) {
      const token = state.tokens[i];
      if (token.type !== "heading_open") continue;
      const inline = state.tokens[i + 1];
      const text = inline.children
        ?.filter((child) => child.type === "text" || child.type === "code_inline")
        .map((child) => child.content)
        .join("");
      token.attrSet("id", slugify(text ?? ""));
    }
  });

  const renderLink = md.renderer.rules.link_open;
  md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    if (/^https?:\/\//.test(token.attrGet("href") ?? "")) {
      token.attrSet("target", "_blank");
      token.attrSet("rel", "noopener noreferrer");
    }
    return renderLink
      ? renderLink(tokens, idx, options, env, self)
      : self.renderToken(tokens, idx, options);
  };

  return {
    name: "markdown-html",
    load(id) {
      if (!id.endsWith(".md")) return null;
      const html = md.render(readFileSync(id, "utf8"));
      return `export default ${JSON.stringify(html)};`;
    },
  };
}

export default defineConfig({
  base: process.env.BASE_PATH ?? "/",
  plugins: [vue(), cfasimWasm(), markdownHtml()],
});
