import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
    build: {
        lib: {
            entry: resolve(__dirname, 'content/injectBadge.ts'),
            formats: ['iife'],
            name: 'InjectBadge',
            fileName: () => 'injectBadge.js'
        },
        outDir: './dist/content',
        emptyOutDir: false
    }
});
