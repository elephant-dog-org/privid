import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
    build: {
        rollupOptions: {
            input: {
                popup: resolve(__dirname, 'popup/popup.ts')
            },
            output: {
                entryFileNames: 'popup/[name].js'
            }
        },
        outDir: './dist',
        emptyOutDir: true
    }
});
