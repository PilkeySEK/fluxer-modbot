<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { PUBLIC_BOT_NAME } from '$env/static/public';

	let { children } = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /><title>{PUBLIC_BOT_NAME}</title></svelte:head>
{@render children()}
