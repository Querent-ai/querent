<script>
	import { onMount } from 'svelte';
	import * as d3 from 'd3';

	// Define your data
	const eventData = [
		{ name: 'Total Events', value: 1482 },
		{ name: 'Total Events Sent', value: 784 },
		{ name: 'Total Events Received', value: 784 },
		{ name: 'Total Events Processed', value: 1482 }
	];

	let svg;

	onMount(() => {
		const width = 600;
		const height = 400;

		const svgElement = d3.select(svg);

		const bubble = d3.pack().size([width, height]).padding(1.5);

		const root = d3.hierarchy({ children: eventData }).sum((d) => d.value);

		const nodes = bubble(root).descendants().slice(1);

		const node = svgElement
			.selectAll()
			.data(nodes)
			.enter()
			.append('g')
			.attr('transform', (d) => `translate(${d.x},${d.y})`);

		node
			.append('circle')
			.attr('r', (d) => d.r)
			.style('fill', (d, i) => d3.schemeCategory10[i]);

		node
			.append('text')
			.attr('dy', '.3em')
			.style('text-anchor', 'middle')
			.text((d) => `${d.data.name}\n${d.data.value}`)
			.style('font-size', (d) => `${d.r / 5}px`)
			.style('fill', 'white')
			.attr('y', -5)
			.call(wrap, 40);

		function wrap(text, width) {
			text.each(function () {
				let text = d3.select(this),
					words = text.text().split(/\s+/).reverse(),
					word,
					line = [],
					lineNumber = 0,
					lineHeight = 1.1,
					y = text.attr('y'),
					dy = parseFloat(text.attr('dy')),
					tspan = text
						.text(null)
						.append('tspan')
						.attr('x', 0)
						.attr('y', y)
						.attr('dy', dy + 'em');
				while ((word = words.pop())) {
					line.push(word);
					tspan.text(line.join(' '));
					if (tspan.node().getComputedTextLength() > width) {
						line.pop();
						tspan.text(line.join(' '));
						line = [word];
						tspan = text
							.append('tspan')
							.attr('x', 0)
							.attr('y', y)
							.attr('dy', ++lineNumber * lineHeight + dy + 'em')
							.text(word);
					}
				}
			});
		}
	});
</script>

<svg bind:this={svg} width="600" height="400"></svg>

<style>
	svg {
		margin: 0 auto;
		display: block;
	}
</style>
