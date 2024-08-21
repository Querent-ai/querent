export function getChartOptions(dark, type) {
	let mainChartColors;
	let title, subtitle, seriesData;

	if (dark) {
		mainChartColors = {
			borderColor: '#007bff',
			labelColor: '#9CA3AF',
			opacityFrom: 0.05,
			opacityTo: 0.4
		};
	} else {
		mainChartColors = {
			borderColor: '#007bff',
			labelColor: '#6B7280',
			opacityFrom: 0.45,
			opacityTo: 0.1
		};
	}

	if (type === 'vector') {
		title = 'Total Events';
		subtitle = 'Events over Time';
		seriesData = [
			{
				name: 'Events',
				data: [100, 164, 254, 368, 513, 652, 742]
			}
		];
	}

	return {
		chart: {
			height: 300,
			type: 'area',
			fontFamily: 'Inter, sans-serif',
			foreColor: mainChartColors.labelColor,
			toolbar: {
				show: false
			}
		},
		fill: {
			type: 'gradient',
			gradient: {
				enabled: true,
				opacityFrom: mainChartColors.opacityFrom,
				opacityTo: mainChartColors.opacityTo
			}
		},
		dataLabels: {
			enabled: false
		},
		tooltip: {
			style: {
				fontSize: '14px',
				fontFamily: 'Inter, sans-serif'
			}
		},
		grid: {
			show: true,
			borderColor: mainChartColors.borderColor,
			strokeDashArray: 1,
			padding: {
				left: 35,
				bottom: 15
			}
		},
		series: seriesData,
		xaxis: {
			categories: ['20', '40', '60', '80', '100', '120', '140'],
			labels: {
				style: {
					colors: [mainChartColors.labelColor],
					fontSize: '14px',
					fontWeight: 500
				}
			},
			axisBorder: {
				color: mainChartColors.borderColor
			},
			axisTicks: {
				color: mainChartColors.borderColor
			},
			crosshairs: {
				show: true,
				position: 'back',
				stroke: {
					color: mainChartColors.borderColor,
					width: 1,
					dashArray: 10
				}
			},
			title: {
				text: 'Time (seconds)',
				style: {
					fontSize: '14px',
					fontWeight: 600,
					color: mainChartColors.labelColor
				}
			}
		},
		yaxis: {
			labels: {
				style: {
					colors: [mainChartColors.labelColor],
					fontSize: '14px',
					fontWeight: 500
				},
				formatter: function (value) {
					return value;
				}
			},
			title: {
				text: 'Number of Events',
				style: {
					fontSize: '14px',
					fontWeight: 600,
					color: mainChartColors.labelColor
				}
			}
		},
		legend: {
			fontSize: '14px',
			fontWeight: 500,
			fontFamily: 'Inter, sans-serif',
			labels: {
				colors: [mainChartColors.labelColor]
			},
			itemMargin: {
				horizontal: 10
			}
		},
		responsive: [
			{
				breakpoint: 1024,
				options: {
					xaxis: {
						labels: {
							show: false
						}
					}
				}
			}
		]
	};
}
