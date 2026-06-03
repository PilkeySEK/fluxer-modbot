<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	let {
		class: extra_classes = '',
		children,
		...rest
	}: {
		children: Snippet;
	} & HTMLButtonAttributes = $props();
</script>

<button
	class="border-default min-h-8 rounded-lg border-gray-500 bg-gray-800 px-2.5 transition-colors hover:bg-gray-700 {extra_classes}"
	{...rest}>{@render children()}</button
>
