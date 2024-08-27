<script lang="ts">
	import { Card } from 'flowbite-svelte';
	import Stats from './Stats.svelte';
	import { onMount } from 'svelte';
	import Chart from 'chart.js/auto';
	import { commands } from '../../service/bindings';

	// import { getChartOptions } from '../../routes/(sidebar)/dashboard/chart_options';

	import TopPairs from './TopPairs.svelte';
	import { pipelineState } from '../../stores/appState';
	// let vectorOptions = getChartOptions(false, 'vector');

	$: selectedPipeline = $pipelineState?.id ? $pipelineState?.id : 'no_active_pipeline';

	let chartInstance: Chart<'line', any[], unknown>;
	let dataPoints: any[] = [];

	onMount(() => {
		const canvas = document.getElementById('myChart') as HTMLCanvasElement;

		if (!canvas) {
			console.error('Canvas element not found!');
			return;
		}

		const ctx = canvas.getContext('2d');

		if (!ctx) {
			console.error('Context element not found!');
			return;
		}
		chartInstance = new Chart(ctx, {
			type: 'line',
			data: {
				datasets: [
					{
						label: 'Total Events',
						backgroundColor: 'rgb(255, 99, 132)',
						borderColor: 'rgb(255, 99, 132)',
						fill: false,
						data: dataPoints
					}
				]
			},
			options: {
				scales: {
					x: {
						type: 'linear',
						min: 0,
						max: 100,
						title: {
							text: 'Time (seconds)',
							display: true,
							font: { size: 16 },
							color: 'black'
						}
					},
					y: {
						beginAtZero: true,
						title: {
							text: 'Total events released',
							display: true,
							font: { size: 16 },
							color: 'black'
						}
					}
				},
				animation: {
					duration: 0
				},
				plugins: {
					title: {
						display: true,
						text: 'Total Events',
						position: 'top',
						align: 'center',
						color: 'black',
						font: {
							size: 24
						}
					}
				}
			}
		});

		const intervalId = setInterval(() => fetchPipelineData(), 10000);
		return () => clearInterval(intervalId);
	});

	async function fetchPipelineData() {
		try {
			if (!selectedPipeline || selectedPipeline == 'no_active_pipeline') {
				return;
			}
			console.log('Calling the API with pipeline ID as ', selectedPipeline);
			const response = await commands.describePipeline(selectedPipeline);

			if (response.status == 'ok') {
				const totalEvents = response.data.total_events;
				const currentTime = performance.now() / 1000;
				dataPoints.push({ x: currentTime % 100, y: totalEvents });
				dataPoints = dataPoints.filter((dp) => currentTime - dp.x <= 100);
				chartInstance.data.datasets[0].data = dataPoints;
				chartInstance.update();
			} else {
				throw new Error(`Unexpected response status: ${response.status}`);
			}
		} catch (error) {
			console.error('Error fetching pipeline data:', error);
			alert(`Failed to fetch pipeline data: ${error.message || error}`);
		}
	}
</script>

<!-- <main class="relative h-full w-full overflow-y-auto bg-blue-500 bg-image"> -->
<div class="mt-px space-y-4">
	<div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
		<div class="lg:col-span-2">
			<Card class="min-h-[500px] min-w-[900px] rounded-lg shadow-lg">
				<div class="space-y-4">
					<canvas id="myChart"></canvas>
				</div>
			</Card>
			<Card class="min-h-[500px] min-w-[900px] rounded-lg shadow-lg">
				<TopPairs />
			</Card>
		</div>
		<div class="lg:col-span-1">
			<Card class="min-h-[550px] rounded-lg shadow-lg">
				<Stats />
			</Card>
		</div>
	</div>
</div>
