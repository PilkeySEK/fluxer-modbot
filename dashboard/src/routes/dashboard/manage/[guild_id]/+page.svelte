<script lang="ts">
	import type { PageProps } from './$types';
	import { api } from '$lib';

	const { params }: PageProps = $props();

	const guild_promise = $state(
		api.GET('/guilds/{guild_id}', {
			params: {
				path: {
					// Wtf, eslint??
					// eslint-disable-next-line svelte/no-unused-svelte-ignore
					// svelte-ignore state_referenced_locally
					guild_id: params.guild_id,
				},
			},
		})
	);
</script>

<div>
	{#await guild_promise}
		<p>Fetching community configuration...</p>
	{:then { data, response }}
		{#if data}
			<div class="m-10">
				<h2 class="text-xl">Command prefixes</h2>
				<!--<ul>
					{#each data.command_prefixes as command_prefix (command_prefix)}
						<li>{command_prefix}</li>
					{/each}
				</ul>-->
				<table class="border">
					<thead>
						<tr>
							<td> a </td>
							<td> a </td>
							<td> a </td>
							<td> a </td>
						</tr>
					</thead>
					<tbody>
						<tr>
							<td> b </td>
							<td> b </td>
							<td> b </td>
							<td> b </td>
						</tr>
					</tbody>
				</table>
			</div>
		{:else}
			<p>[{response.status}] Something went wrong. See the browser console for details.</p>
		{/if}
	{/await}
</div>
