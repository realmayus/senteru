<script lang="ts">
	import { onMount } from 'svelte';
	import logo from '../assets/logo.svg';
	import { unfryEyes } from '$lib';
	import { CircleLayer, GeoJSON, LineLayer, MapLibre } from 'svelte-maplibre';

	type LineRel = {
		id: number;
		name: string;
		color: string;
	};

	let lineData: {
		relation: LineRel;
		nodes: {
			id: number;
			name: string;
			lat: number; // latitude in decimicro degrees
			lon: number;
		}[];
	}[] = [];

	let selection: number[] = [];
	let lineList: LineRel[] = [];

	async function getAllLines() {
		let req = await fetch('http://localhost:3000/subway_lines');
		lineList = await req.json();
	}

	async function addLine(lineId: number) {
		let req = await fetch(`http://localhost:3000/subway_lines/${lineId}`);
		lineData.push(await req.json());
		selection.push(lineId);
		selection = selection;
		lineData = lineData;
		console.log(selection);
	}

	function removeLine(lineId: number) {
		lineData = lineData.filter((l) => l.relation.id !== lineId);
		selection = selection.filter((id) => id !== lineId);
		selection = selection;
		lineData = lineData;
	}

	onMount(async () => {
		await getAllLines();
	});
</script>

<main>
	<div class="left">
		<div class="logo">
			<img src={logo} alt="logo" width="100px" />
			<h1>線テル</h1>
		</div>
		<div class="linelist">
			{#each lineList as line}
				<label>
					<input
						type="checkbox"
						checked={selection.includes(line.id)}
						on:change={(e) => (e.target?.checked ? addLine(line.id) : removeLine(line.id))}
						value={line.id}
					/>
					{line.name}
				</label>
			{/each}
			<button on:click={getAllLines}>Update line directory</button>
		</div>
	</div>
	<MapLibre style="https://basemaps.cartocdn.com/gl/positron-gl-style/style.json" class="map" hash>
		{#each lineData as line}
			{@const color = unfryEyes(line.relation.color)}
			<GeoJSON
				id={line.relation.id.toString()}
				data={{
					type: 'Feature',
					geometry: {
						type: 'LineString',
						coordinates: line.nodes.map((n) => [n.lon / 10000000, n.lat / 10000000])
					},
					properties: {}
				}}
			>
				<LineLayer
					layout={{ 'line-cap': 'round', 'line-join': 'round' }}
					paint={{
						'line-width': 7,
						'line-color': color,
						'line-opacity': 0.8
					}}
				/>
			</GeoJSON>
			<GeoJSON
				id={line.relation.id.toString() + 'nodes'}
				data={{
					type: 'FeatureCollection',
					features: line.nodes.map((n) => ({
						type: 'Feature',
						geometry: {
							type: 'Point',
							coordinates: [n.lon / 10000000, n.lat / 10000000]
						},
						properties: {}
					}))
				}}
			>
				<CircleLayer
					paint={{
						'circle-radius': 3.5,
						'circle-color': 'white'
					}}
				/>
			</GeoJSON>
		{/each}
	</MapLibre>
</main>

<style>
	:global(body) {
		margin: 0;
		font-family: 'Open Sans', sans-serif;
	}
	main {
		display: flex;
	}

	:global(.map) {
		height: 100vh;
		width: calc(100vw - 200px);
	}

	.linelist {
		padding: 10px;

		display: flex;
		flex-direction: column;
		width: 200px;
		height: calc(100vh - 120px);
		overflow-y: scroll;
		overflow-x: scroll;
	}

	button {
		padding: 10px 20px;
		border: none;
		border-radius: 5px;
		background-color: #444;
		color: white;
		font-size: 12pt;
		cursor: pointer;
	}

	.logo {
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: #438eff;
		color: #eeeeee;
	}
</style>
