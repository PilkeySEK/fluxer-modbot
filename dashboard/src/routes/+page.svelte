<script lang="ts">
	import { resolve } from '$app/paths';
	import { PUBLIC_BOT_NAME } from '$env/static/public';
	import fluxer_symbol from '$lib/assets/fluxer-symbol-white.svg';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<div class="flex h-screen flex-col items-center justify-center">
	<h1 class="text-center text-[3em]">{PUBLIC_BOT_NAME} Dashboard</h1>
	<div class="mt-5 flex flex-col items-center rounded-xl border border-white p-3">
		<h3 class="text-center text-[2em]">Sign in</h3>
		{#if data.user !== null}
			<p>You are signed in already as {data.user.username}</p>
			<button
				onclick={() => {
					location.href = resolve('/dashboard');
				}}>Go to dashboard</button
			>
		{/if}
		<p>Sign in with your Fluxer account to access the dashboard.</p>
		<button
			class="mt-3 flex items-center gap-3 rounded-xl border border-white bg-[#4641D9] p-2 hover:bg-[#2621B9]"
			onclick={() => {
				location.href = resolve('/auth/initiate');
			}}
			><img src={fluxer_symbol} alt="Fluxer symbol" class="h-[2em]" />
			<p class="text-xl">Sign in with Fluxer</p></button
		>
	</div>
</div>
