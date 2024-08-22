<script>
	import { onMount } from 'svelte';
	import * as d3 from 'd3';

	let svg;
	let top_pairs = [
		'Sandstone - Shale',
		'Cretaceous - Jurassic',
		'Jurassic - Triassic',
		'Limestone - Shale',
		'Jurassic - Sandstone',
		'Limestone - Sandstone',
		'Fault - Jurassic',
		'Cretaceous - Sandstone',
		'Cretaceous - Shale',
		'Miocene - Pleistocene'
	];

	let sentence =
		"Explore rare and potentially significant connections within your semantic data fabric. These connections unveil the underlying patterns and dynamics woven into your data landscape. Noteworthy interactions are found between 'Horst' and 'Reef', along with the link between 'Jurassic' and 'Magnetic anomaly'.";
	function parseData(inputData, sentence) {
		const nodes = new Set();
		const links = [];
		const sentenceNodes = new Set();
		inputData.forEach((pair) => {
			const [source, target] = pair.split(' - ').map((entity) => entity.trim());
			nodes.add(source);
			nodes.add(target);
			links.push({ source, target, type: 'top_pair' });
		});
		const regex = /'([^']+)'/g;
		const extractedEntities = [];
		let match;
		while ((match = regex.exec(sentence)) !== null) {
			extractedEntities.push(match[1]);
		}
		for (let i = 0; i < extractedEntities.length; i += 2) {
			const source = extractedEntities[i];
			const target = extractedEntities[i + 1];
			nodes.add(source);
			nodes.add(target);
			sentenceNodes.add(source);
			sentenceNodes.add(target);
			links.push({ source, target, type: 'sentence_pair' });
		}
		const nodesArray = Array.from(nodes).map((id) => ({
			id,
			degree: 0,
			type: sentenceNodes.has(id) ? 'sentence_pair' : 'top_pair'
		}));
		links.forEach((link) => {
			const sourceNode = nodesArray.find((node) => node.id === link.source);
			const targetNode = nodesArray.find((node) => node.id === link.target);
			if (sourceNode) sourceNode.degree += 1;
			if (targetNode) targetNode.degree += 1;
		});

		return { nodes: nodesArray, links };
	}
	const graphData = parseData(top_pairs, sentence);

	onMount(() => {
		const width = 800;
		const height = 600;

		const svgElement = d3.select(svg).attr('width', width).attr('height', height);
		const topPairColorScale = d3.scaleSequential(d3.interpolateBlues).domain([
			0,
			d3.max(
				graphData.nodes.filter((d) => d.type === 'top_pair'),
				(d) => d.degree
			)
		]);
		const sentencePairColor = '#ff7f0e';

		const simulation = d3
			.forceSimulation(graphData.nodes)
			.force(
				'link',
				d3
					.forceLink(graphData.links)
					.id((d) => d.id)
					.distance(100)
			)
			.force('charge', d3.forceManyBody().strength(-100))
			.force('center', d3.forceCenter(width / 2, height / 2));

		const link = svgElement
			.append('g')
			.attr('class', 'links')
			.selectAll('line')
			.data(graphData.links)
			.enter()
			.append('line')
			.attr('stroke-width', 2)
			.attr('stroke', '#999');

		const node = svgElement
			.append('g')
			.attr('class', 'nodes')
			.selectAll('circle')
			.data(graphData.nodes)
			.enter()
			.append('circle')
			.attr('r', 15)
			.attr('fill', (d) =>
				d.type === 'top_pair' ? topPairColorScale(d.degree) : sentencePairColor
			)
			.call(d3.drag().on('start', dragstarted).on('drag', dragged).on('end', dragended));

		const text = svgElement
			.append('g')
			.selectAll('text')
			.data(graphData.nodes)
			.enter()
			.append('text')
			.attr('dy', 3)
			.attr('x', 20)
			.style('font-size', '14px')
			.text((d) => d.id);
		const legend = svgElement
			.append('g')
			.attr('class', 'legend')
			.attr('transform', `translate(${width - 150}, 20)`);

		legend
			.append('circle')
			.attr('cx', -60)
			.attr('cy', 10)
			.attr('r', 10)
			.attr(
				'fill',
				topPairColorScale(
					d3.max(
						graphData.nodes.filter((d) => d.type === 'top_pair'),
						(d) => d.degree
					)
				)
			);

		legend
			.append('text')
			.attr('x', -40)
			.attr('y', 15)
			.text('Most Frequent Connections')
			.style('font-size', '12px');

		legend
			.append('text')
			.attr('x', -40)
			.attr('y', 35)
			.text('(Gradient by Centrality)')
			.style('font-size', '10px');

		legend
			.append('circle')
			.attr('cx', -60)
			.attr('cy', 60)
			.attr('r', 10)
			.attr('fill', sentencePairColor);

		legend
			.append('text')
			.attr('x', -40)
			.attr('y', 65)
			.text('Rare and Significant Connections')
			.style('font-size', '12px');

		simulation.on('tick', () => {
			link
				.attr('x1', (d) => d.source.x)
				.attr('y1', (d) => d.source.y)
				.attr('x2', (d) => d.target.x)
				.attr('y2', (d) => d.target.y);

			node.attr('cx', (d) => d.x).attr('cy', (d) => d.y);

			text.attr('x', (d) => d.x).attr('y', (d) => d.y);
		});

		function dragstarted(event, d) {
			if (!event.active) simulation.alphaTarget(0.3).restart();
			d.fx = d.x;
			d.fy = d.y;
		}

		function dragged(event, d) {
			d.fx = event.x;
			d.fy = event.y;
		}

		function dragended(event, d) {
			if (!event.active) simulation.alphaTarget(0);
			d.fx = null;
			d.fy = null;
		}
	});
</script>

<h2 color="black">{'Most Frequent Connections'}</h2>
<svg bind:this={svg}></svg>
