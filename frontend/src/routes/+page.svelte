<script lang="ts">
	let status = $state('Phase 1: Project Initialization & Database Online');
</script>

<svelte:head>
	<title>Nyaalist - Anime Torrent Explorer & Downloader</title>
</svelte:head>

<main class="flex-1 flex flex-col items-center justify-center p-8">
	<div class="max-w-2xl w-full bg-[#1E1E1E] border border-[#2C2C2E] rounded-2xl p-8 shadow-2xl space-y-6">
		<div class="flex items-center justify-between border-b border-[#2C2C2E] pb-4">
			<div class="flex items-center gap-3">
				<div class="w-3.5 h-3.5 rounded-full bg-[#32D74B] animate-pulse"></div>
				<h1 class="text-2xl font-bold tracking-tight text-white font-mono">nyaalist</h1>
			</div>
			<span class="text-xs font-mono px-3 py-1 bg-[#121212] text-[#00E5FF] border border-[#2C2C2E] rounded-full">v0.1.0-alpha</span>
		</div>

		<p class="text-sm text-[#98989D] leading-relaxed">
			Self-hosted anime torrent explorer, downloader, and transcode pipeline with automated AniList metadata synchronization.
		</p>

		<div class="grid grid-cols-3 gap-4 font-mono text-center">
			<div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4">
				<div class="text-xs text-[#98989D] uppercase">Architecture</div>
				<div class="text-sm font-semibold text-[#00E5FF] mt-1">Rust VSA</div>
			</div>
			<div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4">
				<div class="text-xs text-[#98989D] uppercase">Database</div>
				<div class="text-sm font-semibold text-[#00E5FF] mt-1">PostgreSQL 19</div>
			</div>
			<div class="bg-[#121212] border border-[#2C2C2E] rounded-xl p-4">
				<div class="text-xs text-[#98989D] uppercase">Frontend</div>
				<div class="text-sm font-semibold text-[#32D74B] mt-1">SvelteKit 3</div>
			</div>
		</div>

		<div class="bg-[#121212] border-l-4 border-[#00E5FF] p-4 rounded-r-xl">
			<div class="text-xs font-mono text-[#00E5FF] uppercase font-semibold">System Status</div>
			<div class="text-sm text-gray-300 font-mono mt-0.5">{status}</div>
		</div>
	</div>
</main>
