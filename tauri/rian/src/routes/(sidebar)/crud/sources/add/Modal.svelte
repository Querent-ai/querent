<script>
	export let show = false;
	export let message = '';
	export function close() {
		show = false;
	}
</script>

{#if show}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
		<div class="max-w-sm rounded-lg bg-white p-4 shadow-lg">
			<h2 class="text-lg font-semibold">Notice</h2>
			<p class="my-4">{message}</p>
			<button class="rounded bg-blue-500 px-4 py-2 text-white hover:bg-blue-700" on:click={close}
				>OK</button
			>
		</div>
	</div>
{/if}
