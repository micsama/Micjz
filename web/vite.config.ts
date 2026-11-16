import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// 相对路径：本地 / 和服务器 /micjz/ 子路径都能用。
export default defineConfig({
  base: "./",
  plugins: [svelte()],
});
