<script lang="ts">
	import type { GenericButtonProps } from '$lib';
	import GenericButton from './GenericButton.svelte';

	let { class: extraClasses = '', ...rest }: GenericButtonProps = $props();
</script>

<GenericButton
	class="border-2 border-red-600/80 bg-red-600/40 px-2.5 transition-colors hover:bg-red-600/60 {extraClasses}"
	{...rest}
/>
