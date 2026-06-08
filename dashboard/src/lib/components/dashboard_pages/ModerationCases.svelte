<script lang="ts">
	import { onMount } from 'svelte';
	import type { DashboardPagesProps } from './props';
	import { api, type ApiRes } from '$lib';
	import Loader from '../Loader.svelte';

	/*let {
		userData,
		currentGuildData,
		fetchedGuildData,
		fluxerGuildData,
	}: DashboardPagesProps = $props();*/

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

	let query: string = $state('');

	onMount(fetchCases);

	async function fetchCases() {
		let trimmed_query = query.trim();
		let fetchedCases = await api.GET('/guilds/{guild_id}/cases', {
			params: {
				path: {
					guild_id: params.guild_id,
				},
				query: {
					page: undefined,
					search: trimmed_query.length === 0 ? undefined : trimmed_query,
				},
			},
		});

		if (fetchedCases.data) {
			cases_res = {
				error: undefined,
				data: fetchedCases.data,
			};
		} else {
			cases_res = {
				data: undefined,
				error: 'Failed to fetch',
			};
		}
	}
</script>

<input bind:value={query} />
<button onclick={() => fetchCases()}>Fetch</button>
{#if cases_res !== undefined}
	{#if cases_res.data !== undefined}
		<div>
			{#each cases_res.data.cases as moderation_case (moderation_case.case_id)}
				<div class="border-colors-default border-2">
					<p><b>{moderation_case.moderation_kind}</b> of <b>{moderation_case.target_id}</b></p>
				</div>
			{/each}
		</div>
		{#if cases_res.data.has_next}
			has next
		{:else}
			does not have next
		{/if}
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
