<script lang="ts">
	import 'leaflet/dist/leaflet.css';
	import L, { Circle, type LatLngTuple, Map, Polyline } from 'leaflet';
	import { onMount } from 'svelte';
	import logo from '../assets/logo.svg';
	import { unfryEyes } from '$lib';
	let map: Map = null;

	function mapAction(container: HTMLElement) {
		map = L.map(container, { preferCanvas: true }).setView([35.6764, 139.65], 10);
		// L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
		// 	maxZoom: 19,
		// 	attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
		// }).addTo(map);

		L.tileLayer(
			'https://{s}.basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png',
			{
				attribution: `&copy;<a href="https://www.openstreetmap.org/copyright" target="_blank">OpenStreetMap</a>,
	        &copy;<a href="https://carto.com/attributions" target="_blank">CARTO</a>`,
				subdomains: 'abcd',
				maxZoom: 14
			}
		).addTo(map);


		return {
			destroy: () => {
				if (map) {
					map.remove();
					map = null;
				}
			}
		};
	}

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
			lon: number
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
		lineData = lineData.filter(l => l.relation.id !== lineId);
		selection = selection.filter(id => id !== lineId);
		selection = selection;
		lineData = lineData;
	}


	let points: Circle[] = [];
	let polylines: Polyline[] = [];
	$: {
		points.forEach(p => p.remove());
		points = [];
		polylines.forEach(p => p.remove());
		polylines = [];

		for (const line of lineData) {
			let waypoints: LatLngTuple[] = line.nodes.map(n => [n.lat / 10000000, n.lon / 10000000]);
			if (waypoints.length === 0) {
				continue;
			}
			let color = unfryEyes(line.relation.color);
			let polyline = L.polyline(waypoints, { color, weight: 10, stroke: true }).addTo(map);
			polylines.push(polyline);
			for (const node of line.nodes) {
				let lat = node.lat / 10000000;
				let lon = node.lon / 10000000;
				let point = L.circle([lat, lon], {
					color: 'white',
					radius: 30
				}).addTo(map);
				points.push(point);
				waypoints.push([lat, lon]);
			}

		}
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
					<input type="checkbox" checked={selection.includes(line.id)}
								 on:change={(e) => e.target.checked ? addLine(line.id) : removeLine(line.id) } value={line.id} />
					{line.name}
				</label>
			{/each}
		<button on:click={getAllLines}>Update line directory</button>
		</div>
	</div>
	<div class="map" use:mapAction style="height: 100vh"></div>
</main>

<style>
		:global(body) {
				margin: 0;
				font-family: "Open Sans", sans-serif;
		}
    main {
        display: flex;
    }

    .map {
        flex: 1;
        position: relative;
    }

    .map :global(.marker-text) {
        width: 100%;
        text-align: center;
        font-weight: 600;
        background-color: #444;
        color: #EEE;
        border-radius: 0.5rem;
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
				background-color: #438EFF;
				color: #EEEEEE;
		}
</style>
