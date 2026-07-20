<script lang="ts">
	import { resolve } from '$app/paths';
	import { api, iconSize, userProfilePictureUrl, type ApiRes } from '$lib';
	import CopyButton from '$lib/components/CopyButton.svelte';
	import GreenButton from '$lib/components/GreenButton.svelte';
	import LabelledSwitchList from '$lib/components/LabelledSwitchList.svelte';
	import Loader from '$lib/components/Loader.svelte';
	import NormalButton from '$lib/components/NormalButton.svelte';
	import RedButton from '$lib/components/RedButton.svelte';
	import { ArrowLeftIcon } from 'phosphor-svelte';
	import { onMount } from 'svelte';

	let userData:
		| {
				data: ApiRes<'/users/@me'>;
				originalSettings: ApiRes<'/users/@me'>['settings'];
				error: undefined;
		  }
		| {
				data: undefined;
				error: string;
		  }
		| undefined = $state(undefined);

	let currentlySaving = $state(false);

	onMount(async () => {
		let apiRes = await api.GET('/users/@me');
		if (apiRes.data !== undefined) {
			userData = {
				data: apiRes.data,
				error: undefined,
				originalSettings: JSON.parse(JSON.stringify(apiRes.data.settings)),
			};
		} else {
			userData = { data: undefined, error: 'Failed to fetch the current user.' };
		}
	});
</script>

{#if userData === undefined}
	<div class="flex h-screen w-screen items-center justify-center">
		<div class="flex items-center justify-center gap-2">
			<Loader size={1.25} />
			<p>Loading user data...</p>
		</div>
	</div>
{:else if userData.data !== undefined}
	{@const data = userData.data}
	<div class="flex flex-col items-center gap-5 p-3">
		<div class="grid w-full grid-cols-3 items-center border-b border-dashed pb-3">
			<a href={resolve('/dashboard')}
				><NormalButton
					><div class="flex items-center gap-2">
						<ArrowLeftIcon size={iconSize(4)} weight="bold" />
						<p>Dashboard</p>
					</div></NormalButton
				></a
			>
			<div class="flex items-center justify-center gap-3">
				<!-- svelte-ignore a11y_img_redundant_alt -->
				<div class="h-12 w-12">
					<img
						src={userProfilePictureUrl(userData.data.id, userData.data.avatar, 512)}
						class="h-full w-full rounded-full border-2 border-neutral-400"
						alt="Your Fluxer profile picture"
					/>
				</div>
				<div class="flex flex-col">
					<p class="text-lg font-medium">
						{userData.data.username}#{userData.data.discriminator}
					</p>
					<div class="flex gap-1">
						<p class="text-2sm text-neutral-300">{userData.data.id}</p>
						<CopyButton size={3} text={userData.data.id} />
					</div>
				</div>
			</div>
		</div>

		<div class="w-full px-[20%]">
			<LabelledSwitchList
				switches={[
					{
						isChecked: () => data.settings.show_duration_zeroes,
						onChange: (checked) => {
							data.settings.show_duration_zeroes = checked;
						},
						label: 'Show parts of durations that are 0',
						sublabel: 'For example, "3h 10m" will show as "3h 10m 0s" instead',
					},
				]}
			/>
		</div>
	</div>

	{#if JSON.stringify(data.settings) !== JSON.stringify(userData.originalSettings)}
		{@const originalSettings = userData.originalSettings}
		<div
			class="border-colors-default absolute right-5 bottom-5 flex items-center gap-2 rounded-lg border-2 p-2"
		>
			<p>Changes detected!</p>
			<RedButton
				onclick={() => {
					data.settings = JSON.parse(JSON.stringify(originalSettings));
				}}>Undo</RedButton
			>
			<GreenButton
				retain_size
				onclick={async () => {
					currentlySaving = true;

					let res = await api.POST('/users/@me/settings', {
						body: data.settings,
					});

					if (res.error) {
						console.error('Failed to set user settings: ' + JSON.stringify(data.settings));
					} else {
						if (userData?.data === undefined) {
							console.error('userData.data is undefined, cannot set new settings');
							return;
						}
						userData.originalSettings = JSON.parse(JSON.stringify(data.settings));
					}

					currentlySaving = false;
				}}
				>{#if currentlySaving}<Loader size={1.25} />{:else}Save{/if}</GreenButton
			>
		</div>
	{/if}
{:else}
	<div class="flex h-screen w-screen items-center justify-center">
		<p>Error: {userData.error}</p>
	</div>
{/if}
