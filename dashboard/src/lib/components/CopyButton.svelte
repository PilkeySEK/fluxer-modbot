<script lang="ts">
	import { iconSize } from '$lib';
	import { CheckIcon, CopyIcon } from 'phosphor-svelte';

	let { size, text }: { size: number; text: string } = $props();

	let button_state: 'copy' | 'check' = $state('copy');
</script>

<button
	onclick={() => {
		button_state = 'check';
		setTimeout(() => {
			button_state = 'copy';
		}, 500);
		navigator.clipboard.writeText(text);
	}}
>
	{#if button_state === 'check'}
		<!--<Icon src={Check} class="_size-{size} _min-h-{size} _min-w-{size} text-green-400" />-->
		<CheckIcon color="green" size={iconSize(size)} />
	{:else}
		<!--<Icon src={Copy} size={size.toString()} class="_size-{size} _min-h-{size} _min-w-{size}" />-->
		<CopyIcon size={iconSize(size)} />
	{/if}
</button>
