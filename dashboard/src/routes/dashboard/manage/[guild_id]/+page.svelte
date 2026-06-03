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
	import { resolve } from '$app/paths';

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

<div class="flex w-full items-center justify-between">
	<a
		class="ml-2 flex h-8 items-center justify-center gap-2 rounded-lg border border-neutral-700 bg-neutral-800/80 px-2 text-center text-white transition-colors hover:bg-neutral-800 md:ml-10"
		href={resolve('/dashboard')}
	>
		<Icon src={ArrowLeft} theme="bold" class="size-4 min-h-4 min-w-4" />
		<p class="text-sm">Back to dashboard</p>
	</a>
	<UserNav />
</div>

<div class="mx-2 mt-10 flex flex-col gap-10 md:mx-10">
	{#if guild_data === undefined}
		<div class="my-auto flex items-center justify-center gap-2">
			<Loader size={1.25} />
			<p>Fetching community data...</p>
		</div>
	{:else if guild_data.current !== undefined}
		<!-- FIXME: When the formatter doesn't fail in declaration tags anymore, don't use the deprecated @const anymore -->
		{@const config = guild_data.current}
		{@const data = guild_data.fetched[1]}
		<div
			class="relative flex min-w-0 items-center gap-4 rounded-2xl border border-neutral-700 bg-neutral-800/80 p-4"
		>
			<div
				class="flex size-14 min-h-14 min-w-14 items-center justify-center rounded-full bg-gray-700/20 text-center md:size-20 md:min-h-20 md:min-w-20"
			>
				{#if data.icon}
					<img
						src="{PUBLIC_FLUXER_MEDIA_PROXY_BASE}/icons/{data.id}/{data.icon}.webp?size=240"
						alt="Community icon"
						class="h-full w-full rounded-full"
					/>
				{:else}
					<p class="text-muted-foreground text-2xl font-medium">{data.name.slice(0, 2)}</p>
				{/if}
			</div>
			<div class="flex min-w-0 flex-col gap-2">
				<p class="truncate text-lg leading-none font-medium md:text-4xl">{data.name}</p>
				<p class="truncate text-base leading-none text-neutral-400">{data.id}</p>
			</div>
		</div>
		<div class="flex flex-col gap-4">
			<p class="text-lg font-medium">Command prefixes</p>
			<div class="flex flex-wrap gap-2">
				{#each config.command_prefixes as command_prefix, i (command_prefix)}
					<div class="flex h-fit w-fit items-center">
						<div
							class="flex size-8 items-center justify-center rounded-l-lg border border-r-0 border-neutral-700 bg-neutral-800 text-center font-mono text-white"
						>
							<span>{command_prefix}</span>
						</div>
						<button
							class="group flex size-8 items-center justify-center rounded-r-lg border border-neutral-700 bg-neutral-800/80 text-center font-mono text-white transition-colors hover:bg-neutral-800"
							onclick={() => config.command_prefixes.splice(i, 1)}
						>
							<Icon
								src={X}
								theme="bold"
								class="size-4 min-h-4 min-w-4 transition-colors group-hover:text-rose-500"
							/>
						</button>
					</div>
				{/each}
				<div class="flex h-fit w-fit items-center">
					<input
						bind:value={add_prefix_value}
						class="flex size-8 items-center justify-center rounded-l-lg border border-r-0 border-neutral-700 bg-neutral-800 text-center font-mono text-sm text-white"
					/>
					<button
						class="group flex size-8 items-center justify-center rounded-r-lg border border-neutral-700 bg-neutral-800/80 text-center font-mono text-white transition-colors hover:bg-neutral-800 disabled:pointer-events-none disabled:opacity-80"
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
						<Icon src={Plus} theme="bold" class="size-4 min-h-4 min-w-4" />
					</button>
				</div>
			</div>
		</div>
		{#if JSON.stringify(guild_data.fetched[0]) !== JSON.stringify(guild_data.current)}
			{@const guild_data_not_undefined = guild_data}
			<div
				class="fixed right-10 bottom-10 flex min-w-0 items-center gap-4 rounded-xl border border-neutral-700 bg-neutral-800/80 p-4"
			>
				<Icon src={WarningCircle} theme="bold" class="size-5 min-h-5 min-w-5 text-amber-400" />
				<p>Changes detected</p>
				<div class="ml-auto flex items-center gap-2">
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
