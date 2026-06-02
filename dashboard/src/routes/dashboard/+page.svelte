<script lang="ts">
	import { resolve } from '$app/paths';
	import { PUBLIC_FLUXER_MEDIA_PROXY_BASE, PUBLIC_FLUXER_STATIC_BASE } from '$env/static/public';
	import { api, type ApiRes } from '$lib';
	import Loader from '$lib/components/Loader.svelte';
	import { onMount } from 'svelte';
	import SignOutButton from '$lib/components/SignOutButton.svelte';

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

	let me:
		| {
				error: undefined;
				data: ApiRes<'/users/@me'>;
		  }
		| {
				error: string;
				data: undefined;
		  }
		| undefined = $state(undefined);

	onMount(async () => {
		let api_res = await api.GET('/users/@me');

		if (api_res.data !== undefined) {
			me = {
				error: undefined,
				data: api_res.data,
			};
		} else {
			me = {
				data: undefined,
				error: 'Failed to fetch the current user.',
			};
		}
	});
</script>

<div
	class="mt-5 mr-5 ml-auto flex w-fit items-center gap-3 rounded-sm border-2 border-gray-700 bg-gray-900 p-1"
>
	{#if me !== undefined}
		{#if me.data !== undefined}
			<img
				src={me.data.avatar
					? `${PUBLIC_FLUXER_MEDIA_PROXY_BASE}/avatars/${me.data.id}/${me.data.avatar}.webp?size=512`
					: `${PUBLIC_FLUXER_STATIC_BASE}/avatars/0.png`}
				alt={me.data.avatar ? 'Your Fluxer avatar' : 'The default Fluxer avatar'}
				class="h-10 w-10 rounded-full border-2 border-gray-600"
			/>
			<p class="text-2xl">
				{me.data.username}<span class="text-gray-300">#</span>{me.data.discriminator}
			</p>
			<SignOutButton />
		{:else}
			<p>{me.error} Check the browser console for details.</p>
		{/if}
	{:else}
		<!-- min-h-10 to make the height the same as when it is finished loading (to avoid elements below this one changing their position) -->
		<div class="min-h-10">
			<p class="text-2xl">Loading profile...</p>
		</div>
	{/if}
</div>

{#await guilds_promise}
	<div class="mt-10 flex items-center justify-center gap-3">
		<Loader size={2} />
		<p class="text-[1.75em]">Fetching your communities...</p>
	</div>
{:then guilds}
	<p class="mt-3 mb-5 text-center text-2xl">Choose a community to manage:</p>
	<div
		class="m-10 flex h-fit flex-wrap justify-center gap-2 rounded-md border border-gray-700 bg-gray-900 p-3"
	>
		{#each guilds as guild (guild.id)}
			<a href={resolve(`/dashboard/manage/${guild.id}`)}>
				<div class="mr-1 ml-1 flex w-[10em] flex-col items-center">
					{#if guild.icon !== undefined && guild.icon !== null}
						<img
							src="https://fluxerusercontent.com/icons/{guild.id}/{guild.icon}.webp?size=240"
							alt="Community icon"
							class="h-[10em] w-[10em] rounded-full"
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
