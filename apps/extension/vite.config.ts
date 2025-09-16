import { defineConfig } from 'vite';
import { resolve } from 'path';

const suppressChunkSizeWarning = () => {
    return {
        name: 'suppress-chunk-size-warning',
        buildStart() {
            const originalWarn = console.warn;
            console.warn = (...args) => {
                const message = args.join(' ');
                if (
                    message.includes('Some chunks are larger than') ||
                    message.includes('chunk size limit') ||
                    message.includes('Consider:')
                ) {
                    return;
                }
                originalWarn.apply(console, args);
            };
        }
    };
};

export default defineConfig({
    define: {
        'process.env.NODE_ENV': '"production"'
    },
    build: {
        rollupOptions: {
            plugins: [suppressChunkSizeWarning()],
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
                chunkFileNames: 'chunks/[name]-[hash].js',
                format: 'iife',
                manualChunks: (id) => {
                    // Split ethers.js into smaller chunks
                    if (id.includes('ethers/lib/utils')) {
                        return 'ethers-utils';
                    }
                    if (id.includes('ethers/lib/providers')) {
                        return 'ethers-providers';
                    }
                    if (id.includes('ethers/lib/contracts')) {
                        return 'ethers-contracts';
                    }
                    if (id.includes('ethers')) {
                        return 'ethers-core';
                    }
                    // Separate ATProto API into its own chunk
                    if (id.includes('@atproto/api')) {
                        return 'atproto';
                    }
                    // Group other vendor libraries
                    if (id.includes('node_modules')) {
                        return 'vendor';
                    }
                    // Create separate chunks for large modules
                    if (id.includes('blockchain/typechain')) {
                        return 'typechain';
                    }
                }
            },
            treeshake: {
                moduleSideEffects: false,
                propertyReadSideEffects: false,
                tryCatchDeoptimization: false
            }
        },
        outDir: './dist',
        emptyOutDir: true,
        target: 'es2015',
        minify: 'terser',
        chunkSizeWarningLimit: 0,
        reportCompressedSize: false
    }
});
