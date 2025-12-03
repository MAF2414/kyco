/**
 * esbuild configuration for webview bundling
 */

const esbuild = require('esbuild');
const path = require('path');

const isWatch = process.argv.includes('--watch');
const isMinify = process.argv.includes('--minify');

const webviewConfig = {
    entryPoints: [path.join(__dirname, 'src/webview/webview.ts')],
    bundle: true,
    outfile: path.join(__dirname, 'out/webview/webview.js'),
    format: 'iife',
    platform: 'browser',
    target: ['es2020'],
    minify: isMinify,
    sourcemap: !isMinify,
    external: [],
    define: {
        'process.env.NODE_ENV': isMinify ? '"production"' : '"development"',
    },
    loader: {
        '.ts': 'ts',
    },
    tsconfig: path.join(__dirname, 'tsconfig.json'),
};

async function build() {
    try {
        if (isWatch) {
            const ctx = await esbuild.context(webviewConfig);
            await ctx.watch();
            console.log('Watching for changes...');
        } else {
            await esbuild.build(webviewConfig);
            console.log('Build complete');
        }
    } catch (error) {
        console.error('Build failed:', error);
        process.exit(1);
    }
}

build();
