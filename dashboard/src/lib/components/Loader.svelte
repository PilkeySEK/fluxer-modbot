<script lang="ts">
	let { size }: { size: number } = $props();
</script>

<span style:--loader-size="{size}em" class="loader"></span>

<style>
	.loader {
		width: var(--loader-size);
		height: var(--loader-size);
		border: 0.2em solid #fff;
		border-bottom-color: transparent;
		border-radius: 50%;
		display: inline-block;
		box-sizing: border-box;
		animation: rotation 1s linear infinite;
	}

	@keyframes rotation {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
