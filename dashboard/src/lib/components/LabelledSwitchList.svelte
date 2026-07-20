<script lang="ts">
	import Switch from './Switch.svelte';

	let {
		switches,
	}: {
		switches: Array<{
			isChecked: () => boolean;
			onChange: (checked: boolean) => void;
			label: string;
			sublabel?: string;
		}>;
	} = $props();
</script>

<div>
	<!-- eslint-disable-next-line svelte/require-each-key -->
	{#each switches as elem}
		<div class="grid w-full grid-cols-[1fr_auto] items-center gap-2">
			{#if elem.sublabel === undefined}
				<p>{elem.label}</p>
			{:else}
				<div>
					<p>{elem.label}</p>
					<p class="text-sm text-neutral-400">{elem.sublabel}</p>
				</div>
			{/if}
			<Switch
				bind:checked={
					elem.isChecked,
					(newChecked) => {
						elem.onChange(newChecked);
						return newChecked;
					}
				}
			/>
		</div>
	{/each}
</div>
