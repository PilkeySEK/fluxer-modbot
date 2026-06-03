<script lang="ts">
	import { PUBLIC_API_BASE } from '$env/static/public';
	import RedButton from './RedButton.svelte';
</script>

<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
<a href="{PUBLIC_API_BASE}/session/sign-out">
	<RedButton>
		<p class="text-sm font-medium">Sign out</p>
	</RedButton>
</a>
