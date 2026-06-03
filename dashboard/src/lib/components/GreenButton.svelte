<script lang="ts">
	import { type Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	let {
		children,
		retain_size = false,
		onclick,
		...rest
	}: {
		children: Snippet;
		retain_size: boolean;
	} & HTMLButtonAttributes = $props();

	let is_first_click = $state(true);
</script>

<button
	onclick={(event) => {
		if (is_first_click) {
			is_first_click = false;
			if (retain_size) {
				event.currentTarget.style.minHeight = `${event.currentTarget.offsetHeight}px`;
				event.currentTarget.style.minWidth = `${event.currentTarget.offsetWidth}px`;
			}
		}
		if (onclick) {
			onclick(event);
		}
	}}
	class="border-default flex min-h-8 items-center justify-center rounded-lg border-green-500 bg-green-800 px-2.5 transition-colors hover:bg-green-700"
	{...rest}>{@render children()}</button
>
