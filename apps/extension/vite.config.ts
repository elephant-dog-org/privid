import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
    build: {
        rollupOptions: {
            input: {
                popup: resolve(__dirname, 'popup/popup.ts'),
                injectBadge: resolve(__dirname, 'content/injectBadge.ts')
            },
            output: {
                entryFileNames: (chunkInfo) => {
                    if (chunkInfo.name === 'injectBadge') {
                        return 'content/[name].js';
                    }
                    return 'popup/[name].js';
                },
                format: 'iife'
            }
        },
        outDir: './dist',
        emptyOutDir: true,
        target: 'es2015',
        minify: false
    }
});
