import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      // Le BFF FastAPI expose son API JSON sous /api ; le reste du dev
      // server (Vite) sert le SPA. En prod, un même reverse proxy (ou
      // FastAPI servant les fichiers buildés) réunira les deux origines.
      "/api": {
        target: "http://127.0.0.1:8000",
        changeOrigin: true,
      },
    },
  },
})
