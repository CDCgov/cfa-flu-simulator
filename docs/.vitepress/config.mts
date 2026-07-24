import { defineConfig } from "vitepress";

export default defineConfig({
  title: "CFA Flu Simulator",
  description: "Documentation for cfa-flu-simulator",
  base: process.env.DOCS_BASE_PATH ?? "/",
  markdown: {
    math: true,
  },
  themeConfig: {
    outline: "deep",
    sidebar: [
      { text: "Introduction", link: "/" },
      { text: "Model description", link: "/model" },
    ],
    socialLinks: [
      {
        icon: "github",
        link: "https://github.com/CDCgov/cfa-flu-simulator",
      },
    ],
  },
});
