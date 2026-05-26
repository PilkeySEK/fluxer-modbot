<script lang="ts">
	import { onMount } from 'svelte';
	import type { PageProps } from './$types';
	import { resolve } from '$app/paths';

	let { data }: PageProps = $props();

	let res_data = $state(null);
	let loading = $state(true);
	let error: string | null = $state(null);

	// TODO: Don't do this, it's just a test that the API actually works
	onMount(async () => {
		try {
			const response = await fetch(resolve('/api/users/@me'));
			if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
			res_data = await response.json();
		} catch (e) {
			error = (e as Error).message;
		} finally {
			loading = false;
		}
	});
</script>

<h1 class="text-center text-2xl font-semibold">
	Welcome, {data.user.global_name || data.user.username}
</h1>
{#if loading}
	<p>Loading...</p>
{:else if error}
	<p>Error: {error}</p>
{:else}
	<pre>{JSON.stringify(res_data, null, 2)}</pre>
{/if}
