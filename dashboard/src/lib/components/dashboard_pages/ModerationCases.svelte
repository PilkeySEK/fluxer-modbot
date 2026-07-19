<script lang="ts">
	import { onMount } from 'svelte';
	import type { DashboardPagesProps } from './props';
	import { api, iconSize, userProfilePictureUrl, type ApiRes } from '$lib';
	import Loader from '../Loader.svelte';
	import {
		ArrowFatLeftIcon,
		ArrowFatLinesLeftIcon,
		ArrowFatLinesRightIcon,
		ArrowFatRightIcon,
		MagnifyingGlassIcon,
	} from 'phosphor-svelte';
	import { SvelteMap } from 'svelte/reactivity';

	let { params }: DashboardPagesProps = $props();

	let cases_res:
		| {
				error: undefined;
				data: ApiRes<'/guilds/{guild_id}/cases'>;
		  }
		| {
				error: string;
				data: undefined;
		  }
		| undefined = $state(undefined);

	let searchQuery: string = $state('');

	let hasNext = $state(false);
	let hasPrev = $state(false);
	let currentPage = $state(0);

	let userInfo:
		| {
				error: undefined;
				data: SvelteMap<string, ApiRes<'/users/info-bulk', 'post'>[string]>;
		  }
		| {
				error: string;
				data: undefined;
		  }
		| undefined = $state(undefined);

	onMount(() => fetchCases(0));

	async function fetchUserInfo(user_ids: string[]) {
		let res = await api.POST('/users/info-bulk', {
			body: {
				user_ids,
			},
		});

		if (res.data) {
			let map = new SvelteMap<string, ApiRes<'/users/info-bulk', 'post'>[string]>();
			for (const key in res.data) {
				map.set(key, res.data[key]!);
			}
			userInfo = {
				data: map,
				error: undefined,
			};
		} else {
			userInfo = {
				data: undefined,
				error: 'Failed to fetch user info',
			};
		}
	}

	async function fetchCases(page: number) {
		cases_res = undefined;
		let trimmed_query = searchQuery.trim();
		let fetchedCases = await api.GET('/guilds/{guild_id}/cases', {
			params: {
				path: {
					guild_id: params.guild_id,
				},
				query: {
					page,
					search: trimmed_query.length === 0 ? undefined : trimmed_query,
				},
			},
		});

		if (fetchedCases.data) {
			cases_res = {
				error: undefined,
				data: fetchedCases.data,
			};
			hasNext = fetchedCases.data.has_next;
			hasPrev = page > 0;
			currentPage = page;
			await fetchUserInfo(
				fetchedCases.data.cases.map((moderation_case) => moderation_case.target_id)
			);
		} else {
			cases_res = {
				data: undefined,
				error: 'Failed to fetch',
			};
		}
	}
</script>

<div
	class="search-field-parent border-colors-default flex items-center gap-2 rounded-md border-2 p-0.5 px-1"
>
	<input
		bind:value={searchQuery}
		class="w-full px-1"
		placeholder="Case ID, user ID or reason"
		onkeydown={(event) => {
			if (event.key === 'Enter') {
				fetchCases(0);
			}
		}}
	/>
	<button
		onclick={() => {
			fetchCases(0);
		}}
	>
		<MagnifyingGlassIcon size={iconSize(5)} />
	</button>
</div>

{#if cases_res !== undefined}
	{#if cases_res.data !== undefined}
		<p class="text-center">
			Found <b>{cases_res.data.total}</b>
			{cases_res.data.total === 1 ? 'case' : 'cases'}
			matching the current filters (<b>{cases_res.data.cases.length}</b> on this page)
		</p>
		<div class="flex flex-col gap-1">
			{#each cases_res.data.cases as moderation_case (moderation_case.case_id)}
				<div class="border-colors-default rounded-md border-2 px-1">
					<div class="flex items-center gap-1">
						<!-- svelte-ignore a11y_img_redundant_alt -->
						<img
							src={userProfilePictureUrl(
								moderation_case.target_id,
								userInfo?.data?.get(moderation_case.target_id)?.avatar,
								128
							)}
							alt="The target user's profile picture"
							class="max-h-5 max-w-5 rounded-full"
						/>
						<p>
							{#if userInfo?.data}
								{@const thisUser = userInfo.data.get(moderation_case.target_id)}
								{#if thisUser}
									{thisUser.username}#{thisUser.discriminator} ({thisUser.user_id})
								{:else}
									{moderation_case.target_id}
								{/if}
							{:else}
								{moderation_case.target_id}
							{/if}
						</p>
						{#if userInfo === undefined}
							<Loader size={1.25} />
						{/if}
					</div>
					<p>
						<b>{moderation_case.moderation_kind}</b>
						•
						{#if moderation_case.reason}
							{moderation_case.reason}
						{:else}
							<i>No reason specified.</i>
						{/if}
						•
						{#if moderation_case.expiry}
							{JSON.stringify(moderation_case.expiry.duration)}
						{:else}
							permanent
						{/if}
					</p>
				</div>
			{/each}
		</div>
		<div class="sticky right-0 bottom-2 mt-2 flex w-full items-center justify-center">
			<div
				class="grid w-full max-w-100 grid-cols-5 rounded-md border-2 border-neutral-700 bg-neutral-900/80 text-center"
			>
				<button
					disabled={!hasPrev}
					class="flex items-center justify-center"
					onclick={() => {
						fetchCases(0);
					}}
				>
					<ArrowFatLinesLeftIcon size={iconSize(6)} weight={hasPrev ? 'fill' : 'regular'} />
				</button>
				<button
					disabled={!hasPrev}
					class="flex items-center justify-center"
					onclick={() => {
						fetchCases(currentPage - 1);
					}}><ArrowFatLeftIcon size={iconSize(6)} weight={hasPrev ? 'fill' : 'regular'} /></button
				>
				<input
					class="no-number-controls rounded-md border-2 border-transparent text-center invalid:border-red-500 invalid:bg-red-900/20"
					type="number"
					min="0"
					value={currentPage + 1}
					onkeydown={(event) => {
						if (event.key === 'Enter' && !event.currentTarget.validity.patternMismatch) {
							const page = parseInt(event.currentTarget.value);
							fetchCases(page - 1);
						}
					}}
				/>
				<button
					disabled={!hasNext}
					class="flex items-center justify-center"
					onclick={() => {
						fetchCases(currentPage + 1);
					}}><ArrowFatRightIcon size={iconSize(6)} weight={hasNext ? 'fill' : 'regular'} /></button
				>
				<button
					disabled={!hasNext}
					class="flex items-center justify-center"
					onclick={() => {
						if (cases_res?.data) {
							const lastPage = Math.floor(cases_res.data.total / cases_res.data.per_page);
							fetchCases(lastPage);
						}
					}}
				>
					<ArrowFatLinesRightIcon size={iconSize(6)} weight={hasNext ? 'fill' : 'regular'} />
				</button>
			</div>
		</div>
	{:else}
		<div class="flex h-full w-full items-center justify-center">
			<p>
				Error fetching moderation cases: {cases_res.error}<br />See the browser console for details
			</p>
		</div>
	{/if}
{:else}
	<div class="flex h-full w-full items-center justify-center gap-2">
		<Loader size={1.25} />
		<p>Fetching moderation cases...</p>
	</div>
{/if}

<style>
	.search-field-parent:has(input:focus) {
		background-color: var(--color-neutral-900);
	}
</style>
