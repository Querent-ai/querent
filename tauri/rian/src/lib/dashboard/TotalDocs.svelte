<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import {
		Chart,
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		CategoryScale
	} from 'chart.js';
	import { lineChartData } from '../../routes/(sidebar)/dashboard/chartData';

	function getChartColors(darkMode = false) {
		if (darkMode) {
			return {
				borderColor: '#374151',
				backgroundColor: 'rgba(55, 65, 81, 0.15)',
				labelColor: '#9CA3AF'
			};
		} else {
			return {
				borderColor: '#F3F4F6',
				backgroundColor: '#177FBF',
				labelColor: '#6B7280'
			};
		}
	}

	let dark = false;
	const colors = getChartColors(dark);

	let chart: Chart<'line', number[], number>;
	let canvas: HTMLCanvasElement;

	onMount(() => {
		if (canvas) {
			const ctx = canvas.getContext('2d');
			Chart.register(LineController, LineElement, PointElement, LinearScale, CategoryScale);
			if (ctx) {
				const modifiedData = {
					...lineChartData,
					datasets: lineChartData.datasets.map((dataset) => ({
						...dataset,
						borderColor: '#177FBF'
					}))
				};
				chart = new Chart(ctx, {
					type: 'line',
					data: modifiedData,
					options: {
						responsive: true,
						maintainAspectRatio: false,
						scales: {
							y: {
								beginAtZero: false,
								grid: {
									color: colors.borderColor,
									display: true
								},
								border: {
									color: colors.borderColor
								},
								ticks: {
									color: colors.labelColor
								},
								title: {
									display: true,
									text: 'Total Documents processed',
									color: colors.labelColor,
									font: {
										size: 12,
										weight: 'bold'
									}
								}
							},
							x: {
								grid: {
									color: colors.borderColor,
									display: true
								},
								border: {
									color: colors.borderColor
								},
								ticks: {
									// color: colors.labelColor,
									maxTicksLimit: 30
								},
								title: {
									display: true,
									text: 'Total time taken (in seconds)',
									color: colors.labelColor,
									font: {
										size: 12,
										weight: 'bold'
									}
								}
							}
						},
						plugins: {
							legend: {
								labels: {
									color: colors.labelColor
								}
							},
							tooltip: {
								backgroundColor: 'rgba(0, 0, 0, 0.1)'
							}
						}
					}
				});
			}
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
		min-width: 450px;
		width: 100%;
		padding: 16px;
		height: 400px;
	}
	canvas {
		width: 100% !important;
		height: 100% !important;
	}
</style>
