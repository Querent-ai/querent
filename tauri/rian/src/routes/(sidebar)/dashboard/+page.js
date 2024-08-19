/** @type {import('./$types').PageLoad} */
export function load() {
	return {
		series: [
			{
				name: 'Total events',
				data: [0, 1, 5, 40, 68, 100, 120],
				color: '#EF562F'
			}
		]
	};
}
