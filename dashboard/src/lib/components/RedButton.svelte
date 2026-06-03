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

<button
	class="h-8 rounded-lg border-2 border-red-600/60 bg-red-600/40 px-2.5 transition-colors hover:bg-red-600/60"
	{...rest}
>
	{@render children()}
</button>
