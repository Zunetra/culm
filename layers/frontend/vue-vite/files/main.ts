import { createApp } from "vue";

import App from "./App.vue";

const container = document.getElementById("app");

// Thrown rather than defaulted. A missing mount point means the HTML and this
// entry point disagree, and mounting into a silently created element would hide
// that until somebody wonders why the page is blank.
if (!container) {
  throw new Error("no #app element to mount into");
}

const app = createApp(App);

// Vue swallows a render error by default. Without this the page goes blank and
// nothing says why.
app.config.errorHandler = (error, _instance, info): void => {
  console.error("unhandled error", error, info);
};

app.mount(container);
