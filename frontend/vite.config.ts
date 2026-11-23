import { sveltekit } from '@sveltejs/kit/vite';
import adapter from '@sveltejs/adapter-node';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			adapter: adapter({ out: 'build' })
		})
	],
	resolve: {
		alias: {
			'#lib': path.resolve(__dirname, './src/lib'),
			'$lib': path.resolve(__dirname, './src/lib')
		}
	},
	server: {
		port: 3040,
		host: true,
		proxy: {
			'/api': {
				target: process.env.PUBLIC_API_URL || 'http://localhost:8080',
				changeOrigin: true,
				ws: true
			}
		}
	}
});
