<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import {
		Chart,
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		CategoryScale,
		type Point
	} from 'chart.js';
	import { lineChartData } from '../../routes/(sidebar)/dashboard/subjectData';

	let chart: Chart<'line', (number | Point | null)[], unknown>;
	let canvas: HTMLCanvasElement;

	onMount(() => {
		const ctx = canvas.getContext('2d');
		Chart.register(LineController, LineElement, PointElement, LinearScale, CategoryScale);
		if (ctx) {
			chart = new Chart(ctx, {
				type: 'line',
				data: lineChartData,
				options: {
					responsive: true,
					maintainAspectRatio: false,
					scales: {
						y: {
							position: 'left',
							beginAtZero: true,
							grid: {
								display: true,
								color: '#777'
							},
							ticks: {
								precision: 0
							},
							title: {
								display: true,
								text: 'Number of objects',
								color: '#666',
								font: {
									size: 14,
									weight: 'bold'
								}
							}
						},
						yRight: {
							position: 'right',
							beginAtZero: true,
							grid: {
								display: true,
								color: '#eaeaea'
							},
							ticks: {
								precision: 0
							},
							title: {
								display: true,
								text: 'Number of objects',
								color: '#666',
								font: {
									size: 14,
									weight: 'bold'
								}
							}
						},
						x: {
							grid: {
								display: false
							},
							title: {
								display: true,
								text: 'Time elapsed (in seconds)',
								color: '#666',
								font: {
									size: 14,
									weight: 'bold'
								}
							}
						}
					},
					plugins: {
						legend: {
							display: true,
							position: 'top'
						},
						tooltip: {
							mode: 'index',
							intersect: false
						}
					}
				}
			});
		}
	});

	onDestroy(() => {
		if (chart) {
			chart.destroy();
		}
	});
</script>

<div class="chart-container">
	<canvas bind:this={canvas}></canvas>
</div>

<style>
	.chart-container {
		min-width: 600px;
		width: 100%;
		height: 500px;
	}
	canvas {
		width: 100% !important;
		height: 100% !important;
	}
</style>
