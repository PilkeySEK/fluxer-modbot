<script lang="ts">
	import type { GenericButtonProps } from '$lib';
	import GenericButton from './GenericButton.svelte';

	let { class: extraClasses = '', ...rest }: GenericButtonProps = $props();
</script>

<GenericButton
	class="border-2 border-gray-500 bg-gray-800 px-2.5 transition-colors hover:bg-gray-700 {extraClasses}"
	{...rest}
/>
