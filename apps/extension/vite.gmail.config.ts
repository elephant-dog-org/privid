import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
    build: {
        lib: {
            entry: resolve(__dirname, 'content/gmail/injectGmailBadge.ts'),
            formats: ['iife'],
            name: 'InjectGmailBadge',
            fileName: () => 'injectGmailBadge.js'
        },
        outDir: './dist/content/gmail',
        emptyOutDir: false
    }
});
