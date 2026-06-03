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
	class="border-default rounded-sm border-gray-500 bg-gray-800 pr-2 pl-2 hover:bg-gray-700"
	{...rest}>{@render children()}</button
>
