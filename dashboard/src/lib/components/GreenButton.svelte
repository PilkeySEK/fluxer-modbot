<script lang="ts">
	import type { GenericButtonProps } from '$lib';
	import GenericButton from './GenericButton.svelte';

	let { class: extraClasses = '', ...rest }: GenericButtonProps = $props();
</script>

<GenericButton
	class="border-2 border-green-500 bg-green-800 px-2.5 transition-colors hover:bg-green-700 {extraClasses}"
	{...rest}
/>
