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

<div class="mt-10 flex flex-col gap-10 md:mx-10 mx-2">
    {#await guilds_promise}
        <div class="flex items-center gap-4">
            <Loader size={1.5} />
            <p class="text-3xl font-bold">Fetching your communities...</p>
        </div>
    {:then guilds}
        <p class="text-3xl font-bold">Your communities</p>
        <div class="grid lg:grid-cols-5 md:grid-cols-3 grid-cols-2 md:gap-3 gap-2">
            {#each guilds as guild (guild.id)}
                <a href={resolve(`/dashboard/manage/${guild.id}`)} class="p-3 rounded-xl flex items-center gap-3 bg-neutral-800/80 hover:bg-neutral-800 transition-colors border border-neutral-700 min-w-0 relative">
                    <div class="md:size-10 md:min-w-10 md:min-h-10 size-8 min-w-8 min-h-8 rounded-full bg-gray-700/20 flex items-center justify-center text-center">
                        {#if guild.icon}
                            <img
                                src="{PUBLIC_FLUXER_MEDIA_PROXY_BASE}/icons/{guild.id}/{guild.icon}.webp?size=240"
                                alt="Community icon"
                                class="w-full h-full rounded-full"
                            />
                        {:else}
                            <p class="text-base font-medium text-muted-foreground">{guild.name.slice(0, 2)}</p>
                        {/if}
                    </div>
                    <div class="flex flex-col gap-1 min-w-0">
                        <p class="md:text-base text-sm font-medium truncate leading-none">{guild.name}</p>
                        <p class="text-xs text-neutral-400 truncate leading-none">{guild.id}</p>
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
