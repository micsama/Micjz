import { mount } from "svelte";
import "./base.css";
import "./themes/glow/tokens.css";
import "./themes/cute/tokens.css";
import App from "./App.svelte";

// iOS Safari 要有 touchstart 监听，:active 按压态才生效。
document.addEventListener("touchstart", () => {}, { passive: true });

mount(App, { target: document.getElementById("app")! });
