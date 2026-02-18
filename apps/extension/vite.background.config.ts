import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
    build: {
        lib: {
            entry: resolve(__dirname, 'background/service-worker.ts'),
            formats: ['iife'],
            name: 'PrivIDServiceWorker',
            fileName: () => 'service-worker.js'
        },
        outDir: './dist/background',
        emptyOutDir: false
    }
});
