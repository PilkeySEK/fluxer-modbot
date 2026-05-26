<script lang="ts">
	import { resolve } from '$app/paths';
	import { PUBLIC_BOT_NAME } from '$env/static/public';
	import fluxer_symbol from '$lib/assets/fluxer-symbol-white.svg';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let isLoggedIn = $derived(data.user !== null);
</script>

<div class="mr-10 ml-10 flex h-screen flex-col items-center justify-center text-center">
	<h1 class="text-[3em]">{PUBLIC_BOT_NAME} Dashboard</h1>
	<div class="mt-5 flex flex-col items-center rounded-xl border border-white p-3">
		<h3 class="text-[2em]">Sign in</h3>
		{#if isLoggedIn}
			<p>
				You are signed in as <span class="text-blue-300"
					>{data.user!.username}#{data.user!.discriminator}</span
				>
			</p>
			<button
				class="mt-3 rounded-xl border-2 border-white bg-blue-950 p-2 text-xl hover:bg-blue-900"
				onclick={() => {
					location.href = resolve('/dashboard');
				}}>Go to dashboard</button
			>
		{/if}
		{#if isLoggedIn}
			<div class="mt-5 mb-3 w-full border border-t border-dashed border-white"></div>
		{/if}
		<p>
			{#if isLoggedIn}
				Or, sign in with a different Fluxer account:
			{:else}
				Sign in with your Fluxer account to access the dashboard:
			{/if}
		</p>
		<button
			class="mt-3 flex items-center gap-3 rounded-xl border-2 border-white bg-[#4641D9] p-2 hover:bg-[#2621B9]"
			onclick={() => {
				location.href = resolve('/api/auth/initiate');
			}}
			><img src={fluxer_symbol} alt="Fluxer symbol" class="h-[2em]" />
			<p class="font-(family-name:--fluxer-font) text-2xl font-semibold">
				Sign in with Fluxer
			</p></button
		>
	</div>
</div>
