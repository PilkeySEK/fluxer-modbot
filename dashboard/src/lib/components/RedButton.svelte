<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	let {
		children,
		...rest
	}: {
		children: Snippet;
	} & HTMLButtonAttributes = $props();
</script>

<button class="rounded-lg border border-red-600/60 bg-red-600/40 hover:bg-red-600/60 transition-colors h-8 px-2.5" {...rest}>
    {@render children()}
</button>
