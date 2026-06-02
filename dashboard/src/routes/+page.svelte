<script lang="ts">
	import { resolve } from '$app/paths';
	import { PUBLIC_BOT_NAME } from '$env/static/public';
	import { api, type ApiRes } from '$lib';
	import fluxer_symbol from '$lib/assets/fluxer-symbol-white.svg';
	import Loader from '$lib/components/Loader.svelte';
	import NormalButton from '$lib/components/NormalButton.svelte';
	import SignOutButton from '$lib/components/SignOutButton.svelte';
	import { onMount } from 'svelte';

	let maybe_me: ApiRes<'/users/@maybe-me'> | undefined | null = $state(undefined);

	onMount(async () => {
		const api_res = await api.GET('/users/@maybe-me');

		if (api_res.data === undefined) {
			maybe_me = null;
			return;
		}

		if (api_res.data === null) {
			maybe_me = null;
		} else {
			maybe_me = api_res.data;
		}
	});
</script>

<div class="mr-10 ml-10 flex h-screen flex-col items-center justify-center text-center">
	<h1 class="text-[3em]">{PUBLIC_BOT_NAME} Dashboard</h1>
	<div class="mt-5 flex min-w-200 flex-col items-center rounded-xl border border-white p-3">
		{#if maybe_me === undefined}
			<div class="flex items-center justify-center gap-2">
				<Loader size={1.25} />
				<p>Checking whether you are logged in already...</p>
			</div>
		{:else if maybe_me !== null}
			<p>
				You are currently logged in as <span class="text-blue-200"
					>{maybe_me.username}#{maybe_me.discriminator}</span
				>
			</p>
			<div class="mt-1 flex items-center gap-2">
				<a href={resolve('/dashboard')}><NormalButton>Go to dashboard</NormalButton></a>
				<SignOutButton />
			</div>
		{:else}
			<h3 class="text-[2em]">Sign in</h3>
			<p>Sign in with your Fluxer account to access the dashboard:</p>
			<button
				class="mt-3 flex items-center gap-3 rounded-xl border-2 border-white bg-[#4641D9] p-2 hover:bg-[#2621B9]"
				onclick={() => {
					location.href = '/api/oauth/initiate';
				}}
				><img src={fluxer_symbol} alt="Fluxer symbol" class="h-[2em]" />
				<p class="font-(family-name:--fluxer-font) text-2xl font-semibold">
					Sign in with Fluxer
				</p></button
			>
		{/if}
	</div>
</div>
