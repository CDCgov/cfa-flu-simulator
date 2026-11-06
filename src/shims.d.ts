declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<object, object, unknown>;
  export default component;
}

declare module "*.toml?raw" {
  const content: string;
  export default content;
}

declare module "*.md" {
  const html: string;
  export default html;
}
