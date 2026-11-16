import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// LanLink 前端构建配置
// 说明：与 Tauri v2 集成时，Tauri 会通过 VITE 的环境变量注入；
// 这里保持标准 Vite 配置，浏览器态可直接 npm run dev 预览。
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  // Tauri 期望相对路径资源；浏览器预览不受影响
  base: './',
  server: {
    // 与 tauri.conf.json 的 devUrl 必须一致，strictPort 防止被占用时漂移
    // 到 5174 导致 Tauri 窗口加载不到前端。
    port: 5173,
    strictPort: true,
    host: '127.0.0.1',
  },
  build: {
    target: 'es2020',
    outDir: 'dist',
    emptyOutDir: true,
  },
})
