<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount, type Component, type ComponentProps } from 'svelte';
	import type { PageProps } from './$types';
	import { api, iconSize, type ApiRes } from '$lib';
	import Loader from '$lib/components/Loader.svelte';
	import { PUBLIC_FLUXER_MEDIA_PROXY_BASE, PUBLIC_FLUXER_STATIC_BASE } from '$env/static/public';
	import CopyButton from '$lib/components/CopyButton.svelte';
	import { ArrowsLeftRightIcon, GearSixIcon, ScrollIcon } from 'phosphor-svelte';
	import type { ResolvedPathname } from '$app/types';
	import type { DashboardPagesProps } from '$lib/components/dashboard_pages/props';
	import GeneralSettings from '$lib/components/dashboard_pages/GeneralSettings.svelte';
	import ModerationCases from '$lib/components/dashboard_pages/ModerationCases.svelte';
	import RedButton from '$lib/components/RedButton.svelte';
	import GreenButton from '$lib/components/GreenButton.svelte';
	import { pushState } from '$app/navigation';
	import { page } from '$app/state';

	let userDataStore: ApiRes<'/users/@me'> | undefined = $state(undefined);
	let guildDataStore:
		| {
				current: ApiRes<'/guilds/{guild_id}'>;
				fetched: [ApiRes<'/guilds/{guild_id}'>, ApiRes<'/users/@me/guilds'>[number]];
		  }
		| undefined = $state(undefined);

	let { params }: PageProps = $props();

	let error: undefined | string = $state(undefined);
	let currentlySaving: boolean = $state(false);

	const possiblePageIds = ['general-settings', 'cases'] as const;
	type PageId = (typeof possiblePageIds)[number];
	// let selectedPage: PageId = $state('general-settings');
	let selectedPage: PageId = $derived.by(() => {
		const pageFromUrl = page.url.searchParams.get('page');
		if (pageFromUrl === null) {
			return 'general-settings';
		}
		if (possiblePageIds.includes(pageFromUrl as PageId)) {
			return pageFromUrl as PageId;
		} else {
			return 'general-settings';
		}
	});
	// svelte-ignore state_referenced_locally
	const pages: Map<
		PageId,
		{
			label: string;
			icon: Component<ComponentProps<typeof GearSixIcon>, object, ''>;
			path: ResolvedPathname;
			component: Component<DashboardPagesProps, object, ''>;
		}
	> = new Map([
		[
			'general-settings',
			{
				label: 'General Settings',
				icon: GearSixIcon,
				path: resolve('/dashboard/manage/[guild_id]', {
					guild_id: params.guild_id,
				}),
				component: GeneralSettings,
			},
		],
		[
			'cases',
			{
				label: 'Moderation Cases',
				icon: ScrollIcon,
				path: resolve('/dashboard/manage/[guild_id]', {
					guild_id: params.guild_id,
				}),
				component: ModerationCases,
			},
		],
	]);

	onMount(async () => {
		// Insert the ?page= parameter
		if (page.url.searchParams.get('page') === null) {
			gotoPage('general-settings');
		}
		let api_res = await api.GET('/users/@me');

		if (api_res.data !== undefined) {
			// setUserContext(api_res.data);
			userDataStore = api_res.data;
		} else {
			error = 'Failed to fetch the current user.';
			return;
		}

		let guild_config = await api.GET('/guilds/{guild_id}', {
			params: {
				path: {
					guild_id: params.guild_id,
				},
			},
		});
		let my_guilds = await api.GET('/users/@me/guilds');

		if (guild_config.data === undefined) {
			error = 'Failed to fetch community config.';
			return;
		}
		if (my_guilds.data === undefined) {
			error = 'Failed to fetch your communities.';
			return;
		}
		let this_guild = my_guilds.data.find((guild) => guild.id === params.guild_id);
		if (this_guild === undefined) {
			error = 'Could not find this community in the list of your communities.';
			return;
		}

		guildDataStore = {
			current: guild_config.data,
			fetched: [guild_config.data, this_guild],
		};
	});

	function gotoPage(id: PageId) {
		selectedPage = id;
		let url = page.url;
		url.searchParams.set('page', selectedPage);
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		pushState(url, {});
	}
</script>

<div class="flex min-h-screen w-screen">
	<div class="w-75 min-w-75">
		<!-- So that the sidebar doesn't overlap with the page content -->
	</div>
	<div class="fixed grid h-full w-75 min-w-75 grid-rows-[auto_1fr_auto] p-1">
		<div class="rounded-t-lg border-2 border-b-0 border-neutral-700 bg-neutral-800/80 p-2">
			{#if guildDataStore !== undefined}
				{@const data = guildDataStore.fetched[1]}
				<div class="flex items-center gap-2">
					<div class="h-14 min-h-14 w-14 min-w-14">
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
					<div class="flex flex-col">
						<div class="flex items-center gap-1">
							<p class="text-lg font-medium">{guildDataStore.fetched[1].name}</p>
							<a href={resolve('/dashboard')} title="Manage a different community"
								><ArrowsLeftRightIcon size={iconSize(6)} /></a
							>
						</div>
						<div class="flex gap-1">
							<p class="text-2sm text-neutral-300">{data.id}</p>
							<CopyButton size={3} text={data.id} />
						</div>
					</div>
				</div>
			{:else if error === undefined}
				<div class="flex items-center justify-center gap-2">
					<Loader size={1.25} />
					<p>Fetching community data...</p>
				</div>
			{/if}
		</div>
		<nav
			class="border-t-dashed border-b-dashed flex flex-col border-2 border-neutral-700 bg-neutral-800/80 p-1"
		>
			{#each pages as page (page[0])}
				{@const IconComponent = page[1].icon}
				<button
					class="flex items-center gap-2 rounded-lg p-1 px-3 text-lg text-gray-200 {page[0] ===
					selectedPage
						? 'bg-neutral-700/50'
						: 'hover:bg-neutral-700/50'}"
					onclick={() => {
						// selectedPage = page[0];
						gotoPage(page[0]);
					}}
				>
					<IconComponent size={iconSize(6)} weight="fill" />
					<p>{page[1].label}</p>
				</button>
			{/each}
		</nav>
		<div class="rounded-b-lg border-2 border-t-0 border-neutral-700 bg-neutral-800/80 p-1">
			{#if userDataStore !== undefined}
				<div class="flex items-center gap-2">
					<div class="h-12 w-12">
						{#if userDataStore.avatar}
							<img
								src={`${PUBLIC_FLUXER_MEDIA_PROXY_BASE}/avatars/${userDataStore.id}/${userDataStore.avatar}.webp?size=512`}
								alt="Your Fluxer avatar"
								class="h-full w-full rounded-full"
							/>
						{:else}
							<img
								src={`${PUBLIC_FLUXER_STATIC_BASE}/avatars/${BigInt(userDataStore.id) % 6n}.png`}
								alt="The default Fluxer avatar"
								class="h-full w-full rounded-full"
							/>
						{/if}
					</div>
					<div class="flex flex-col">
						<p class="text-lg font-medium">
							{userDataStore.username}#{userDataStore.discriminator}
						</p>
						<div class="flex gap-1">
							<p class="text-2sm text-neutral-300">{userDataStore.id}</p>
							<CopyButton size={3} text={userDataStore.id} />
						</div>
					</div>
				</div>
			{:else}
				<div class="flex items-center justify-center gap-2">
					<Loader size={1.25} />
					<p>Loading user data...</p>
				</div>
			{/if}
		</div>
	</div>
	<div class="w-full">
		{#if guildDataStore !== undefined && userDataStore !== undefined}
			{@const PageComponent = pages.get(selectedPage)!.component}
			{@const IconComponent = pages.get(selectedPage)!.icon}
			<div class="flex w-full flex-col p-1">
				<div class="flex gap-3 p-2">
					<IconComponent size={iconSize(10)} weight="fill" />
					<p class="text-4xl font-semibold">{pages.get(selectedPage)!.label}</p>
				</div>
				<div class="w-full p-3">
					<PageComponent
						userData={userDataStore}
						currentGuildData={guildDataStore.current}
						fetchedGuildData={guildDataStore.fetched[0]}
						fluxerGuildData={guildDataStore.fetched[1]}
						{params}
					/>
				</div>
			</div>
		{:else}
			<div class="flex h-full w-full flex-row items-center justify-center gap-2">
				{#if error === undefined}
					<Loader size={1.25} />
					<p>Loading community and user data...</p>
				{:else}
					<p>Error fetching community or user data: {error}</p>
				{/if}
			</div>
		{/if}
	</div>
	{#if guildDataStore}
		{@const data = guildDataStore}
		{#if JSON.stringify(guildDataStore.current) !== JSON.stringify(guildDataStore.fetched[0])}
			<div
				class="border-colors-default absolute right-5 bottom-5 flex items-center gap-2 rounded-lg border-2 p-2"
			>
				<p>Changes detected!</p>
				<RedButton
					onclick={() => {
						data.current = JSON.parse(JSON.stringify(data.fetched[0]));
					}}>Undo</RedButton
				>
				<GreenButton
					retain_size
					onclick={async () => {
						currentlySaving = true;

						let res = await api.PATCH('/guilds/{guild_id}', {
							params: {
								path: {
									guild_id: params.guild_id,
								},
							},
							body: data.current,
						});

						if (res.error) {
							console.error('Failed to update guild with data: ' + JSON.stringify(data.current));
						} else if (!res.data) {
							console.error('Response data is undefined');
						} else {
							data.fetched[0] = res.data;
							data.current = structuredClone(res.data);
						}
						currentlySaving = false;
					}}
					>{#if currentlySaving}<Loader size={1.25} />{:else}Save{/if}</GreenButton
				>
			</div>
		{/if}
	{/if}
</div>
