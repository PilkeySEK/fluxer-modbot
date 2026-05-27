<script lang="ts">
	import type { PageProps } from './$types';
	let { params }: PageProps = $props();
</script>

<p>Managing {params.guild_id}</p>
