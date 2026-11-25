import { sveltekit } from '@sveltejs/kit/vite';
import adapter from '@sveltejs/adapter-node';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig, loadEnv } from 'vite';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig(({ mode }) => {
	// Load environment variables from .env / .env.local
	const env = loadEnv(mode, process.cwd(), '');
	const backendTarget = env.PUBLIC_API_URL || 'http://127.0.0.1:5041';
	const allowedHosts = env.ALLOWED_HOSTS
		? env.ALLOWED_HOSTS === 'true'
			? true
			: env.ALLOWED_HOSTS.split(',').map((h) => h.trim()).filter(Boolean)
		: undefined;

	return {
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
			...(allowedHosts !== undefined ? { allowedHosts } : {}),
			proxy: {
				'/api': {
					target: backendTarget,
					changeOrigin: true,
					ws: true
				}
			}
		}
	};
});
