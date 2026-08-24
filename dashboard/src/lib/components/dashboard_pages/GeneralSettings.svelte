<script lang="ts">
	import { ArrowBendLeftUpIcon, PlusIcon, XIcon } from 'phosphor-svelte';
	import type { DashboardPagesProps } from './props';
	import { iconSize } from '$lib';
	import { PUBLIC_MAX_COMMAND_PREFIX_LEN, PUBLIC_MAX_COMMAND_PREFIXES } from '$env/static/public';

	let { currentGuildData }: DashboardPagesProps = $props();

	let command_prefix_input: string = $state('');
</script>

<div class="flex flex-col gap-3">
	<div class="border-colors-default rounded-lg border-2 px-2 py-1">
		<h2 class="text-2xl font-semibold">Command prefixes</h2>
		<div class="mt-1 flex flex-wrap gap-2 font-mono">
			{#each currentGuildData.command_prefixes as command_prefix, i (command_prefix)}
				<div class="flex items-center">
					<div
						class="flex min-h-8 min-w-8 items-center justify-center rounded-l-md border-2 border-neutral-600 bg-neutral-800/80 px-2"
					>
						<p>{command_prefix}</p>
					</div>
					<button
						class="flex h-full items-center rounded-r-md border-2 border-l-0 border-neutral-600 bg-neutral-900/80 px-1 transition-colors hover:text-rose-400"
						onclick={() => {
							currentGuildData.command_prefixes.splice(i, 1);
						}}
					>
						<XIcon size={iconSize(4)} weight="bold" />
					</button>
				</div>
			{/each}
			<div class="flex items-center">
				<input
					class="flex field-sizing-content min-h-8 min-w-8 items-center justify-center rounded-l-md border-2 border-neutral-600 bg-neutral-800 px-2"
					maxlength={parseInt(PUBLIC_MAX_COMMAND_PREFIX_LEN)}
					bind:value={command_prefix_input}
				/>
				<button
					class="flex h-full items-center rounded-r-md border-2 border-l-0 border-neutral-600 bg-neutral-900/80 px-1 transition-colors hover:text-green-500 disabled:pointer-events-none"
					disabled={(() => {
						if (currentGuildData.command_prefixes.length >= parseInt(PUBLIC_MAX_COMMAND_PREFIXES)) {
							return true;
						}
						if (currentGuildData.command_prefixes.includes(command_prefix_input)) {
							return true;
						}
						if (command_prefix_input.trim().length === 0) {
							return true;
						}

						return false;
					})()}
					onclick={() => {
						const trimmed = command_prefix_input.trim();
						currentGuildData.command_prefixes.push(trimmed);
						command_prefix_input = '';
					}}
				>
					<PlusIcon size={iconSize(4)} />
				</button>
			</div>
		</div>
	</div>

	<div class="border-colors-default rounded-lg border-2 px-2 py-1">
		<div class="grid grid-cols-[auto_1fr] gap-x-2">
			<div class="flex items-center justify-center">
				<input type="checkbox" bind:checked={currentGuildData.moderation_hierarchy_enabled} />
			</div>
			<p>Respect moderation hierarchy for punishments</p>
			<ArrowBendLeftUpIcon size={iconSize(4)} weight="bold" color="gray" />
			<p class="text-[gray]">
				When this is enabled, moderators with a lower role can't punish moderators with a higher
				role.
			</p>
		</div>
	</div>
</div>
