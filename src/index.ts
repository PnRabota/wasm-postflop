import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./components/App.vue";
import "./index.css";
import "tippy.js/dist/tippy.css";

const savedTheme = localStorage.getItem("ui-theme-mode");
const hasSavedTheme = savedTheme === "light" || savedTheme === "dark";
const preferredTheme = window.matchMedia?.("(prefers-color-scheme: dark)")
  .matches
  ? "dark"
  : "light";
const themeMode = hasSavedTheme ? savedTheme : preferredTheme;
document.documentElement.setAttribute("data-theme", themeMode);
document.documentElement.style.colorScheme = themeMode;

createApp(App).use(createPinia()).mount("#app");
