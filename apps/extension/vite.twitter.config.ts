import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
    build: {
        lib: {
            entry: resolve(__dirname, 'content/twitter/injectTwitterBadge.ts'),
            formats: ['iife'],
            name: 'InjectTwitterBadge',
            fileName: () => 'injectTwitterBadge.js'
        },
        outDir: './dist/content/twitter',
        emptyOutDir: false
    }
});
