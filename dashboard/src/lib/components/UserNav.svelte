<script lang="ts">
	import { PUBLIC_FLUXER_MEDIA_PROXY_BASE, PUBLIC_FLUXER_STATIC_BASE } from '$env/static/public';
	import { api, type ApiRes } from '$lib';
	import { onMount } from 'svelte';
	import SignOutButton from '$lib/components/SignOutButton.svelte';

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
	class="border-default relative mt-2 mr-2 ml-auto flex min-h-18 w-fit min-w-0 items-center gap-3 rounded-xl border-neutral-700 bg-neutral-800/80 p-3 md:mt-10 md:mr-10"
>
	{#if me !== undefined}
		{#if me.data !== undefined}
			<div
				class="flex size-8 min-h-8 min-w-8 items-center justify-center rounded-full bg-gray-700/20 text-center md:size-10 md:min-h-10 md:min-w-10"
			>
				<img
					src={me.data.avatar
						? `${PUBLIC_FLUXER_MEDIA_PROXY_BASE}/avatars/${me.data.id}/${me.data.avatar}.webp?size=512`
						: `${PUBLIC_FLUXER_STATIC_BASE}/avatars/0.png`}
					alt={me.data.avatar ? 'Your Fluxer avatar' : 'The default Fluxer avatar'}
					class="h-full w-full rounded-full"
				/>
			</div>
			<div class="flex min-w-0 flex-col gap-1">
				<p class="truncate text-sm leading-none md:text-base">
					{me.data.username}<span class="text-gray-300">#</span>{me.data.discriminator}
				</p>
				<p class="truncate text-xs leading-none text-neutral-400">{me.data.id}</p>
			</div>
			<SignOutButton />
		{:else}
			<p>{me.error} Check the browser console for details.</p>
		{/if}
	{:else}
		<p class="text-base">Loading profile...</p>
	{/if}
</div>
