<script lang="ts">
	import { resolve } from '$app/paths';
	import { PUBLIC_FLUXER_MEDIA_PROXY_BASE } from '$env/static/public';
	import { api, type ApiRes } from '$lib';
	import Loader from '$lib/components/Loader.svelte';
	import UserNav from '$lib/components/UserNav.svelte';

	let guilds_promise: Promise<ApiRes<'/users/@me/guilds'> | undefined> = $state(
		new Promise((fulfill, reject) => {
			api
				.GET('/users/@me/guilds')
				.then(({ data }) => {
					fulfill(data);
				})
				.catch(reject);
		})
	);
</script>

<UserNav />

<div class="mx-2 mt-10 flex flex-col gap-10 md:mx-10">
	{#await guilds_promise}
		<div class="flex items-center gap-4">
			<Loader size={1.5} />
			<p class="text-3xl font-bold">Fetching your communities...</p>
		</div>
	{:then guilds}
		<p class="text-3xl font-bold">Your communities</p>
		<div class="grid grid-cols-2 gap-2 md:grid-cols-3 md:gap-3 lg:grid-cols-5">
			{#each guilds as guild (guild.id)}
				<a
					href={resolve(`/dashboard/manage/${guild.id}`)}
					class="border-default relative flex min-w-0 items-center gap-3 rounded-xl border-neutral-700 bg-neutral-800/80 p-3 transition-colors hover:bg-neutral-800"
				>
					<div
						class="flex size-8 min-h-8 min-w-8 items-center justify-center rounded-full bg-gray-700/20 text-center md:size-10 md:min-h-10 md:min-w-10"
					>
						{#if guild.icon}
							<img
								src="{PUBLIC_FLUXER_MEDIA_PROXY_BASE}/icons/{guild.id}/{guild.icon}.webp?size=240"
								alt="Community icon"
								class="h-full w-full rounded-full"
							/>
						{:else}
							<p class="text-muted-foreground text-base font-medium">{guild.name.slice(0, 2)}</p>
						{/if}
					</div>
					<div class="flex min-w-0 flex-col gap-1">
						<p class="truncate text-sm leading-none font-medium md:text-base">{guild.name}</p>
						<p class="truncate text-xs leading-none text-neutral-400">{guild.id}</p>
					</div>
				</a>
			{/each}
		</div>
	{:catch err}
		<p>Error: {err}</p>
	{/await}
</div>

<!--
<style>
	.loader {
		--loader-size: 2em;
		width: var(--loader-size);
		height: var(--loader-size);
		border: 0.2em solid #fff;
		border-bottom-color: transparent;
		border-radius: 50%;
		display: inline-block;
		box-sizing: border-box;
		animation: rotation 1s linear infinite;
	}

	@keyframes rotation {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
-->
