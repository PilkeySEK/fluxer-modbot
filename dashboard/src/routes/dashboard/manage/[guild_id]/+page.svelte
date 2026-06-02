<script lang="ts">
	import type { PageProps } from './$types';
	import { api, type ApiRes } from '$lib';
	import { onMount } from 'svelte';
	import { PUBLIC_FLUXER_MEDIA_PROXY_BASE } from '$env/static/public';
	import Loader from '$lib/components/Loader.svelte';
	import RedButton from '$lib/components/RedButton.svelte';
	import GreenButton from '$lib/components/GreenButton.svelte';

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

{#if guild_data === undefined}
	<div class="flex min-h-screen items-center justify-center gap-2">
		<Loader size={1.25} />
		<p>Fetching community data...</p>
	</div>
{:else if guild_data.current !== undefined}
	<!-- FIXME: When the formatter doesn't fail in declaration tags anymore, don't use the deprecated @const anymore -->
	{@const config = guild_data.current}
	{@const data = guild_data.fetched[1]}
	<div class="m-10">
		<div class="flex items-center gap-5">
			{#if data.icon}
				<img
					class="h-25 w-25"
					src={`${PUBLIC_FLUXER_MEDIA_PROXY_BASE}/icons/${data.id}/${data.icon}.webp?size=512`}
					alt="Community icon"
				/>
			{:else}
				<p>No icon</p>
			{/if}
			<h1 class="text-[5em]">{data.name}</h1>
		</div>
		<div class="flex w-fit items-center gap-4 rounded-sm bg-gray-950 p-3">
			<p>Command prefixes</p>
			<div class="flex gap-2">
				{#each config.command_prefixes as command_prefix, i (command_prefix)}
					<div class="rounded-sm bg-white p-1 text-black">
						<span class="">{command_prefix}</span>
						<button
							class="rounded-sm bg-gray-700 pr-1 pl-1 text-white"
							onclick={() => {
								config.command_prefixes.splice(i, 1);
							}}>X</button
						>
					</div>
				{/each}
				<div class="rounded-sm bg-white p-1 text-black">
					<input
						type="text"
						class="w-5 rounded-sm border border-black"
						id="add-prefix"
						bind:value={add_prefix_value}
					/>
					<button
						class="rounded-sm bg-gray-800 pr-1 pl-1 text-white"
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
						+
					</button>
				</div>
			</div>
		</div>
	</div>

	{#if JSON.stringify(guild_data.fetched[0]) !== JSON.stringify(guild_data.current)}
		{@const guild_data_not_undefined = guild_data}
		<div
			class="fixed right-10 bottom-10 flex items-center gap-3 rounded-lg border-2 border-gray-500 bg-gray-900 p-3"
		>
			<p>Changes detected</p>
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
		<p>Detected changes</p>
		<p>
			fetched: {JSON.stringify(guild_data.fetched[0])}
			<br />
			current: {JSON.stringify(guild_data.current)}
		</p>
	{:else}
		<p>Did not detect changes</p>
	{/if}
{:else}
	<p>{guild_data.error}<br />See the browser console for details.</p>
{/if}
