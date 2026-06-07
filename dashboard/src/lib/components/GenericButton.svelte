<script lang="ts">
	import type { GenericButtonProps } from '$lib';

	let {
		children,
		retain_size = false,
		onclick,
		class: extraClasses = '',
		...rest
	}: GenericButtonProps = $props();

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
	class="flex min-h-8 items-center justify-center rounded-lg {extraClasses}"
	{...rest}>{@render children()}</button
>
