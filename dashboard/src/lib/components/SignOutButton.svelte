<script lang="ts">
	import { resolve } from '$app/paths';
	import { sign_out } from '$lib';
	import Loader from './Loader.svelte';
	import RedButton from './RedButton.svelte';

	let signing_out: boolean = $state(false);
</script>

<RedButton
	onclick={async () => {
		signing_out = true;
		await sign_out();
		location.href = resolve('/');
	}}
	// Should be about the same as the text here:
	style="min-width: 5em;"
>
	{#if !signing_out}
		Sign out
	{:else}
		<Loader size={1} />
	{/if}
</RedButton>
