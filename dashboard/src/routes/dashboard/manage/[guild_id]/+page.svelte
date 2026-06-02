<script lang="ts">
	import type { PageProps } from './$types';
	import { api, type ApiRes } from '$lib';
	import { onMount } from 'svelte';
	import { PUBLIC_FLUXER_MEDIA_PROXY_BASE } from '$env/static/public';
	import Loader from '$lib/components/Loader.svelte';
	import RedButton from '$lib/components/RedButton.svelte';
	import GreenButton from '$lib/components/GreenButton.svelte';
	import UserNav from '$lib/components/UserNav.svelte';
	import { Icon } from '@steeze-ui/svelte-icon';
	import { ArrowLeft, Plus, WarningCircle, X } from '@steeze-ui/phosphor-icons';

	const { params }: PageProps = $props();

	let guild_data:
		| {
				current: ApiRes<'/guilds/{guild_id}'>;
				fetched: [ApiRes<'/guilds/{guild_id}'>, ApiRes<'/users/@me/guilds'>[number]];
				error: undefined;
		  }
		| {
				current: undefined;
				fetched: undefined;
				error: string;
		  }
		| undefined = $state(undefined);

	let add_prefix_value = $state('');
	let currently_saving = $state(false);

	onMount(async () => {
		let guild_config = await api.GET('/guilds/{guild_id}', {
			params: {
				path: {
					guild_id: params.guild_id,
				},
			},
		});
		let my_guilds = await api.GET('/users/@me/guilds');

		if (guild_config.data === undefined) {
			guild_data = {
				current: undefined,
				fetched: undefined,
				error: 'Failed to fetch community config.',
			};
			return;
		}
		if (my_guilds.data === undefined) {
			guild_data = {
				current: undefined,
				fetched: undefined,
				error: 'Failed to fetch your communities.',
			};
			return;
		}
		let this_guild = my_guilds.data.find((guild) => guild.id === params.guild_id);
		if (this_guild === undefined) {
			guild_data = {
				current: undefined,
				fetched: undefined,
				error: 'Could not find this community in the list of your communities.',
			};
			return;
		}

		guild_data = {
			error: undefined,
			current: guild_config.data,
			fetched: [guild_config.data, this_guild],
		};
	});
</script>

<div class="flex items-center justify-between w-full">
    <a
        class="md:ml-10 ml-2 flex gap-2 items-center justify-center bg-neutral-800/80 border border-neutral-700 text-white h-8 px-2 text-center rounded-lg hover:bg-neutral-800 transition-colors"
        href="/dashboard"
    >
        <Icon src={ArrowLeft} theme="bold" class="size-4 min-w-4 min-h-4" />
        <p class="text-sm">Back to dashboard</p>
    </a>
    <UserNav />
</div>

<div class="mt-10 flex flex-col gap-10 md:mx-10 mx-2">
    {#if guild_data === undefined}
        <div class="flex my-auto items-center justify-center gap-2">
            <Loader size={1.25} />
            <p>Fetching community data...</p>
        </div>
    {:else if guild_data.current !== undefined}
        <!-- FIXME: When the formatter doesn't fail in declaration tags anymore, don't use the deprecated @const anymore -->
        {@const config = guild_data.current}
        {@const data = guild_data.fetched[1]}
        <div class="p-4 rounded-2xl flex items-center gap-4 bg-neutral-800/80 border border-neutral-700 min-w-0 relative">
            <div class="md:size-20 md:min-w-20 md:min-h-20 size-14 min-w-14 min-h-14 rounded-full bg-gray-700/20 flex items-center justify-center text-center">
                {#if data.icon}
                    <img
                        src="{PUBLIC_FLUXER_MEDIA_PROXY_BASE}/icons/{data.id}/{data.icon}.webp?size=240"
                        alt="Community icon"
                        class="w-full h-full rounded-full"
                    />
                {:else}
                    <p class="text-2xl font-medium text-muted-foreground">{data.name.slice(0, 2)}</p>
                {/if}
            </div>
            <div class="flex flex-col gap-2 min-w-0">
                <p class="md:text-4xl text-lg font-medium truncate leading-none">{data.name}</p>
                <p class="text-base text-neutral-400 truncate leading-none">{data.id}</p>
            </div>
        </div>
        <div class="flex flex-col gap-4">
            <p class="text-lg font-medium">Command prefixes</p>
            <div class="flex gap-2 flex-wrap">
				{#each config.command_prefixes as command_prefix, i (command_prefix)}
                    <div class="flex items-center w-fit h-fit">
                        <div class="flex items-center justify-center bg-neutral-800 text-white font-mono size-8 text-center rounded-l-lg border border-neutral-700 border-r-0">
                            <span>{command_prefix}</span>
                        </div>
                        <button
                            class="group flex items-center justify-center bg-neutral-800/80 border border-neutral-700 text-white font-mono size-8 text-center rounded-r-lg hover:bg-neutral-800 transition-colors"
                            onclick={() => config.command_prefixes.splice(i, 1)}
                        >
                            <Icon src={X} theme="bold" class="group-hover:text-rose-500 transition-colors size-4 min-w-4 min-h-4" />
                        </button>
                    </div>
				{/each}
                <div class="flex items-center w-fit h-fit">
                    <input
                        bind:value={add_prefix_value}
                        class="flex text-sm items-center justify-center bg-neutral-800 text-white font-mono size-8 text-center rounded-l-lg border border-neutral-700 border-r-0"
                    />
                    <button
                        class="group flex items-center justify-center bg-neutral-800/80 border border-neutral-700 text-white font-mono size-8 text-center rounded-r-lg hover:bg-neutral-800 transition-colors disabled:opacity-80 disabled:pointer-events-none"
                        disabled={!add_prefix_value.trim()}
						onclick={() => {
							const trimmed = add_prefix_value.trim();
							if (trimmed === '') {
								return;
							}
							if (config.command_prefixes.includes(trimmed)) {
								return;
							}
							add_prefix_value = '';
							config.command_prefixes.push(trimmed);
						}}
                    >
                        <Icon src={Plus} theme="bold" class="size-4 min-w-4 min-h-4" />
                    </button>
                </div>
            </div>
        </div>
        {#if JSON.stringify(guild_data.fetched[0]) !== JSON.stringify(guild_data.current)}
            {@const guild_data_not_undefined = guild_data}
            <div class="fixed right-10 bottom-10 p-4 rounded-xl flex items-center gap-4 bg-neutral-800/80 border border-neutral-700 min-w-0">
                <Icon src={WarningCircle} theme="bold" class="text-amber-400 size-5 min-w-5 min-h-5" />
                <p>Changes detected</p>
                <div class="flex items-center gap-2 ml-auto">
                    <RedButton
                        onclick={() => {
                            // I hate JavaScript
                            guild_data_not_undefined.current = JSON.parse(
                                JSON.stringify(guild_data_not_undefined.fetched[0])
                            );
                        }}>Undo</RedButton
                    >
                    <GreenButton
                        retain_size
                        onclick={() => {
                            currently_saving = !currently_saving;
                        }}
                        >{#if !currently_saving}Save{:else}<Loader size={1.25} />{/if}</GreenButton
                    >
                </div>
            </div>
            <p>Detected changes</p>
            <p>
                fetched: {JSON.stringify(guild_data.fetched[0])}
                <br />
                current: {JSON.stringify(guild_data.current)}
            </p>
        {/if}
    {:else}
        <p>{guild_data.error}<br />See the browser console for details.</p>
    {/if}
</div>
