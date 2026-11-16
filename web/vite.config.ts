import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { createMock } from "./mock/server";

/** `--mode mock`：/api 由内存里的假后端应答（dev 和 preview 都挂）。 */
function mockApi(): Plugin {
  const handle = createMock();
  // 不能有返回值：Vite 会把钩子返回的函数当作「内置中间件之后再挂」来调用。
  const use = (server: { middlewares: { use: (fn: (req: any, res: any, next: () => void) => void) => void } }): void => {
    server.middlewares.use(async (req, res, next) => {
      const chunks: Buffer[] = [];
      for await (const c of req) chunks.push(c);
      const request = new Request(`http://local${req.url}`, {
        method: req.method,
        headers: req.headers,
        body: chunks.length ? Buffer.concat(chunks) : undefined,
      });
      const out = await handle(request);
      if (!out) return next();
      res.statusCode = out.status;
      out.headers.forEach((v, k) => res.setHeader(k, v));
      res.end(Buffer.from(await out.arrayBuffer()));
    });
  };
  return { name: "micjz-mock-api", configureServer: use, configurePreviewServer: use };
}

// 相对路径：本地 / 和服务器 /micjz/ 子路径都能用。
export default defineConfig(({ mode }) => ({
  base: "./",
  plugins: [svelte(), ...(mode === "mock" ? [mockApi()] : [])],
  // 平时连本机的 micjz 服务。
  server: mode === "mock" ? {} : { proxy: { "/api": "http://127.0.0.1:8730" } },
}));
