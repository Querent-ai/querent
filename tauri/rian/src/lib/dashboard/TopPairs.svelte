<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';
	import { discoveryApiResponseStore } from '../../../src/stores/appState';

	let svg: SVGSVGElement;
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

	let discoveryResponse;
	let connections: { nodes?: any; links?: { source: string; target: string; type: string }[] };

	$: {
		discoveryResponse = $discoveryApiResponseStore.length > 0 ? $discoveryApiResponseStore : [];

		let currentCategory = discoveryResponse.filter((cat) => cat.tags === 'Filters');
		if (currentCategory.length > 0) {
			// call the function with sentences and take from filters
			let sentences = discoveryResponse[1].sentence;
			connections = parseData(currentCategory[0].top_pairs, sentences);
			console.log('Connections are    ', connections);
		} else {
			// take from first response
			if (discoveryResponse.length > 0) {
				connections = parseData(discoveryResponse[0].top_pairs, '');
			} else {
				connections = {};
			}
		}
	}

	function parseData(inputData: any[], sentence: string) {
		const nodes = new Set();
		const links = [];
		const sentenceNodes = new Set();
		inputData.forEach((pair) => {
			const [source, target] = pair.split(' - ').map((entity: string) => entity.trim());
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

	onMount(() => {
		const width = 800;
		const height = 600;

		if (!connections || !connections.nodes || connections.nodes.length < 1) {
			const svgElement = d3.select(svg).attr('width', width).attr('height', height);

			svgElement
				.append('text')
				.attr('x', width / 2)
				.attr('y', height / 2)
				.attr('text-anchor', 'middle')
				.text('Empty Discovery Graph');
		} else {
			const svgElement = d3.select(svg).attr('width', width).attr('height', height);
			const topPairNodes = connections.nodes.filter(
				(d: { type: string; degree: any }): d is { type: string; degree: number } =>
					d.type === 'top_pair' && typeof d.degree === 'number'
			);

			const maxDegree = d3.max(topPairNodes, (d: { degree: any }) => d.degree) || 0;

			const topPairColorScale = d3.scaleSequential(d3.interpolateBlues).domain([0, maxDegree]);
			const sentencePairColor = '#ff7f0e';

			const simulation = d3
				.forceSimulation(connections.nodes)
				.force(
					'link',
					d3
						.forceLink(connections.links)
						.id((d: { id: any }) => d.id)
						.distance(100)
				)
				.force('charge', d3.forceManyBody().strength(-100))
				.force('center', d3.forceCenter(width / 2, height / 2));

			const link = svgElement
				.append('g')
				.attr('class', 'links')
				.selectAll('line')
				.data(connections.links)
				.enter()
				.append('line')
				.attr('stroke-width', 2)
				.attr('stroke', '#999');

			const node = svgElement
				.append('g')
				.attr('class', 'nodes')
				.selectAll('circle')
				.data(connections.nodes)
				.enter()
				.append('circle')
				.attr('r', 15)
				.attr('fill', (d: { type: string; degree: any }) =>
					d.type === 'top_pair' ? topPairColorScale(d.degree) : sentencePairColor
				)
				.call(d3.drag().on('start', dragstarted).on('drag', dragged).on('end', dragended));

			const text = svgElement
				.append('g')
				.selectAll('text')
				.data(connections.nodes)
				.enter()
				.append('text')
				.attr('dy', 3)
				.attr('x', 20)
				.style('font-size', '14px')
				.text((d: { id: any }) => d.id);
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
							connections.nodes.filter((d: { type: string }) => d.type === 'top_pair'),
							(d: { degree: any }) => d.degree
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
					.attr('x1', (d: { source: { x: any } }) => d.source.x)
					.attr('y1', (d: { source: { y: any } }) => d.source.y)
					.attr('x2', (d: { target: { x: any } }) => d.target.x)
					.attr('y2', (d: { target: { y: any } }) => d.target.y);

				node.attr('cx', (d: { x: any }) => d.x).attr('cy', (d: { y: any }) => d.y);

				text.attr('x', (d: { x: any }) => d.x).attr('y', (d: { y: any }) => d.y);
			});

			function dragstarted(event: { active: any }, d: { fx: any; x: any; fy: any; y: any }) {
				if (!event.active) simulation.alphaTarget(0.3).restart();
				d.fx = d.x;
				d.fy = d.y;
			}

			function dragged(event: { x: any; y: any }, d: { fx: any; fy: any }) {
				d.fx = event.x;
				d.fy = event.y;
			}

			function dragended(event: { active: any }, d: { fx: null; fy: null }) {
				if (!event.active) simulation.alphaTarget(0);
				d.fx = null;
				d.fy = null;
			}
		}
	});
</script>

<h2 class="text-center text-[24px] font-semibold text-gray-900 dark:text-white">
	{'Most Frequent Connections'}
</h2>
<svg bind:this={svg}></svg>
