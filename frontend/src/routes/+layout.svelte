<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="min-h-screen bg-[#121212] text-[#FFFFFF] flex flex-col">
	{@render children()}
</div>
