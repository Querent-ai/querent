<script>
	import { onMount } from 'svelte';
	import * as d3 from 'd3';
	import { dataSources } from '../../stores/appState';
	import { countSourcesByType } from '../../stores/appState';

	const data = [
		{ source: 'Google Drive', value: countSourcesByType('drive') },
		{ source: 'Local Storage', value: countSourcesByType('files') }
	];

	let svg;

	onMount(() => {
		const margin = { top: 40, right: 40, bottom: 80, left: 60 };
		const width = 500 - margin.left - margin.right;
		const height = 400 - margin.top - margin.bottom;

		const x = d3.scaleBand().range([0, width]).padding(0.3);

		const y = d3.scaleLinear().range([height, 0]);

		const svgElement = d3
			.select(svg)
			.attr('width', width + margin.left + margin.right)
			.attr('height', height + margin.top + margin.bottom)
			.append('g')
			.attr('transform', `translate(${margin.left},${margin.top})`);

		x.domain(data.map((d) => d.source));
		y.domain([0, 5]);

		svgElement
			.selectAll('.bar')
			.data(data)
			.enter()
			.append('rect')
			.attr('class', 'bar')
			.attr('x', (d) => x(d.source))
			.attr('width', x.bandwidth())
			.attr('y', (d) => y(d.value))
			.attr('height', (d) => height - y(d.value))
			.attr('fill', '#4CAF50');

		svgElement
			.append('g')
			.attr('transform', `translate(0,${height})`)
			.call(d3.axisBottom(x))
			.selectAll('text')
			.attr('y', 10) // Move text down
			.attr('x', 0) // Center text
			.attr('dy', '.35em')
			.attr('transform', 'rotate(0)') // No rotation
			.style('text-anchor', 'middle');

		svgElement.append('g').call(d3.axisLeft(y).ticks(5));

		// Add X axis label
		svgElement
			.append('text')
			.attr('transform', `translate(${width / 2}, ${height + margin.bottom - 10})`)
			.style('text-anchor', 'middle')
			.text('Data Sources');

		// Add Y axis label
		svgElement
			.append('text')
			.attr('transform', 'rotate(-90)')
			.attr('y', 0 - margin.left)
			.attr('x', 0 - height / 2)
			.attr('dy', '1em')
			.style('text-anchor', 'middle')
			.text('Number of connections');
	});
</script>

<svg bind:this={svg}></svg>

<style>
	svg {
		margin: 0 auto;
		display: block;
	}
</style>
