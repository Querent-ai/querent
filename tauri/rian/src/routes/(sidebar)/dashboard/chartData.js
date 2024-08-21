// chartData.js
export const lineChartData = {
	labels: Array.from({ length: 12 }, (_, i) => i * 5),
	datasets: [
		{
			label: 'Total Documents',
			data: [2, 5, 8, 10, 12, 12, 13, 14, 14, 14, 18, 19],
			fill: true,
			borderColor: 'rgb(75, 192, 192)',
			tension: 0.1
		}
	]
};
