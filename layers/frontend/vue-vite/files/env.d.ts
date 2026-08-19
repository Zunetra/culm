/// <reference types="vite/client" />

/* TypeScript has no idea what a .vue file is, so without this every
 * `import App from "./App.vue"` is TS2307 and the whole application is
 * untyped from the entry point down. A meta framework generates this file
 * during its prepare step; without one the layer has to ship it. */
declare module "*.vue" {
  import type { DefineComponent } from "vue";

  const component: DefineComponent<
    Record<string, never>,
    Record<string, never>,
    unknown
  >;
  export default component;
}
