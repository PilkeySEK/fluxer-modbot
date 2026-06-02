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

<div class="min-h-18 md:mr-10 mr-2 md:mt-10 mt-2 ml-auto w-fit p-3 rounded-xl flex items-center gap-3 bg-neutral-800/80 border border-neutral-700 min-w-0 relative">
	{#if me !== undefined}
		{#if me.data !== undefined}
            <div class="md:size-10 md:min-w-10 md:min-h-10 size-8 min-w-8 min-h-8 rounded-full bg-gray-700/20 flex items-center justify-center text-center">
                <img
                    src={me.data.avatar
                        ? `${PUBLIC_FLUXER_MEDIA_PROXY_BASE}/avatars/${me.data.id}/${me.data.avatar}.webp?size=512`
                        : `${PUBLIC_FLUXER_STATIC_BASE}/avatars/0.png`}
                    alt={me.data.avatar ? 'Your Fluxer avatar' : 'The default Fluxer avatar'}
                    class="w-full h-full rounded-full"
                />
            </div>
            <div class="flex flex-col gap-1 min-w-0">
                <p class="md:text-base text-sm truncate leading-none">{me.data.username}<span class="text-gray-300">#</span>{me.data.discriminator}</p>
                <p class="text-xs text-neutral-400 truncate leading-none">{me.data.id}</p>
            </div>
            <SignOutButton />
		{:else}
			<p>{me.error} Check the browser console for details.</p>
		{/if}
	{:else}
        <p class="text-base">Loading profile...</p>
	{/if}
</div>
