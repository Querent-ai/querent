<script lang="ts">
	import { Card, Chart } from 'flowbite-svelte';
	import options from '$lib/graphs/traffic';
	import SmallPanel from './SmallPanel.svelte';
	import { BookOpenOutline } from 'flowbite-svelte-icons';
	import More from '$lib/widgets/More.svelte';

	export let dark: boolean = false;
</script>

<Card size="xl" class="h-fit">
	<div
		class="mb-4 items-center justify-between border-b border-gray-200 pb-4 dark:border-gray-700 sm:flex"
	>
		<div class="mb-4 w-full sm:mb-0">
			<h3 class="text-base font-normal text-gray-500 dark:text-gray-400">
				Total Events generated per document
			</h3>
			<span class="text-2xl font-bold leading-none text-gray-900 dark:text-white sm:text-3xl"
				>Events</span
			>
		</div>
		<More title="Full report" href="#top" />
	</div>
	<Chart options={options(dark)}></Chart>
	<div class="mb-4 flex items-center justify-between pt-4 sm:pt-6 lg:justify-evenly">
		<SmallPanel title="Document1" subtitle="34">
			<BookOpenOutline slot="icon" size="xl" class="mb-1" />
		</SmallPanel>
		<SmallPanel title="Document2" subtitle="10">
			<BookOpenOutline slot="icon" size="xl" class="mb-1" />
		</SmallPanel>
		<SmallPanel title="Document3" subtitle="16">
			<BookOpenOutline slot="icon" size="xl" class="mb-1" />
		</SmallPanel>
	</div>
</Card>
