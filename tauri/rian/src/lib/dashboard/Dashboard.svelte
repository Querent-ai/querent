<script lang="ts">
	import ChartWidget from '$lib/widgets/ChartWidget.svelte';
	import { Card } from 'flowbite-svelte';
	// import type { PageData } from '../../routes/(sidebar)/$types';
	import Stats from './Stats.svelte';

	import { getChartOptions } from '../../routes/(sidebar)/dashboard/chart_options';
	import TotalSources from './TotalSources.svelte';
	// import TotalEvents from './TotalEvents.svelte';

	// export let data: PageData;

	let vectorOptions = getChartOptions(false, 'vector');
</script>

<!-- <main class="relative h-full w-full overflow-y-auto bg-blue-500 bg-image"> -->
<div class="mt-px space-y-4">
	<div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
		<div class="lg:col-span-2">
			<Card class="min-h-[550px] min-w-[900px] rounded-lg shadow-lg">
				<div class="space-y-4">
					<ChartWidget
						chartOptions={vectorOptions}
						title="Total Events"
						subtitle="Events over Time"
					/>
					<!-- <ChartWidget
						chartOptions={graphOptions}
						title="Total Graph Events"
						subtitle="Events over Time"
					/> -->
				</div>
			</Card>
			<Card class="min-h-[550px] min-w-[900px] rounded-lg shadow-lg">
				<TotalSources />
			</Card>
		</div>
		<div class="lg:col-span-1">
			<Card class="min-h-[550px] rounded-lg shadow-lg">
				<Stats />
			</Card>
		</div>
	</div>
</div>

<style>
	.bg-image {
		background-image: url('/images/querent-background.png');
		background-size: cover;
		background-position: calc(50% - 20px) center;
		background-repeat: no-repeat;
		background-color: #3b82f6;
	}
</style>
