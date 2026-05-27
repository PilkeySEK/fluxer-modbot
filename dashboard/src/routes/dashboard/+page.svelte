<script lang="ts">
	import type { PageProps } from './$types';
	import { resolve } from '$app/paths';
	import type { GuildResponse } from '$lib/server/fluxer_types';

	let { data }: PageProps = $props();

	let guilds_promise: Promise<GuildResponse[]> = $state(
		new Promise((fulfill, reject) => {
			fetch(resolve('/api/users/@me/guilds'))
				.then((res) => res.json())
				.then((json) => fulfill(json))
				.catch((err) => reject(err));
		})
	);
</script>

<h1 class="text-center text-2xl font-semibold">
	Welcome, {data.user.global_name ?? data.user.username}
</h1>
{#await guilds_promise}
	<div class="mt-10 flex items-center justify-center gap-3">
		<span class="loader"></span>
		<p class="text-[1.75em]">Fetching your communities...</p>
	</div>
{:then guilds}
	<p class="mt-3 mb-5 text-center text-2xl">Choose a community to manage:</p>
	<div class="flex flex-wrap justify-center gap-2">
		{#each guilds as guild (guild.id)}
			<a href={resolve(`/dashboard/manage/${guild.id}`)}>
				<div class="mr-1 ml-1 flex w-[10em] flex-col items-center">
					{#if guild.icon !== undefined && guild.icon !== null}
						<img
							src="https://fluxerusercontent.com/icons/{guild.id}/{guild.icon}.webp?size=240"
							alt="Community icon"
							class="h-[10em] w-[10em] rounded-xl"
						/>
					{:else}
						<div class="h-[10em] content-center items-center">No icon</div>
					{/if}
					<p class="mt-3 text-center">{guild.name}</p>
				</div>
			</a>
		{/each}
	</div>
{:catch err}
	<p>Error: {err}</p>
{/await}

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
