<script lang="ts">
	import { Card } from 'flowbite-svelte';
	import Stats from './Stats.svelte';
	import { onMount } from 'svelte';
	import Chart from 'chart.js/auto';
	import { commands } from '../../service/bindings';

	import TopPairs from './TopPairs.svelte';
	import {
		pipelineStartTime,
		pipelineState,
		statsDataTime,
		statsDataTotalEvents
	} from '../../stores/appState';

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
				labels: $statsDataTime,
				datasets: [
					{
						label: 'Total Events',
						backgroundColor: 'rgb(255, 99, 132)',
						borderColor: 'rgb(255, 99, 132)',
						fill: false,
						data: $statsDataTotalEvents
					}
				]
			},
			options: {
				scales: {
					x: {
						type: 'linear',
						title: {
							text: 'Time (seconds)',
							display: true,
							font: { size: 16 },
							color: 'black'
						},
						ticks: {
							callback: function (value, index, values) {
								return Math.round(Number(value));
							}
						}
					},
					y: {
						beginAtZero: true,
						title: {
							text: 'Total events released',
							display: true,
							font: { size: 16 },
							color: 'black'
						},
						suggestedMax: 10
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

		fetchPipelineData();

		const intervalId = setInterval(() => fetchPipelineData(), 10000);
		return () => clearInterval(intervalId);
	});

	async function fetchPipelineData() {
		try {
			if (!selectedPipeline || selectedPipeline == 'no_active_pipeline') {
				return;
			}
			console.log('Pipeline start time initially ', $pipelineStartTime);
			console.log('Calling the API with pipeline ID as ', selectedPipeline);
			const response = await commands.describePipeline(selectedPipeline);

			if (response.status == 'ok') {
				const totalEvents = response.data.total_events;
				const currentUnixTime = Math.floor(Date.now() / 1000);
				const timeSinceStart = currentUnixTime - $pipelineStartTime;

				statsDataTime.update((xData) => {
					xData.push(timeSinceStart);
					return xData.slice(-10);
				});

				statsDataTotalEvents.update((yData) => {
					yData.push(totalEvents);
					return yData.slice(-10);
				});

				chartInstance.data.labels = $statsDataTime;
				chartInstance.data.datasets[0].data = $statsDataTotalEvents;

				const maxEvents = Math.max(...$statsDataTotalEvents);
				const newMax = Math.ceil(maxEvents * 1.2);
				if (chartInstance.options?.scales?.y) {
					(chartInstance.options.scales.y as any).max = newMax;
				}

				chartInstance.update();
				console.log('Total events are ', totalEvents);
			} else {
				throw new Error(`Unexpected response status: ${response.status}`);
			}
		} catch (error) {
			console.error('Error fetching pipeline data:', error);
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
