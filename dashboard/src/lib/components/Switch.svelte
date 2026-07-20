<script lang="ts">
	let { checked = $bindable() } = $props();
</script>

<input type="checkbox" class="switch" bind:checked />

<style>
	.switch {
		appearance: none;
		-webkit-appearance: none;
		position: relative;
		width: 50px;
		height: 26px;
		background-color: var(--color-neutral-700);
		border-radius: calc(infinity * 1px);
		border: white 2px solid;
		cursor: pointer;
		transition: background-color 0.1s ease;
	}

	.switch::before {
		content: '';
		position: absolute;
		top: 2px;
		left: 2px;
		width: 18px;
		height: 18px;
		background-color: var(--color-neutral-300);
		border-radius: calc(infinity * 1px);
		transition: transform 0.1s ease;
	}

	.switch:checked {
		background-color: var(--color-green-700);
	}

	.switch:checked::before {
		transform: translateX(24px);
		background-color: white;
	}
</style>
