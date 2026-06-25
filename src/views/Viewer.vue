<template>
	<div class="viewer">
		<div class="scroll">
			<canvas
				ref="canvasEl"
				class="screen"
				:class="{ hidden: !hasFrame }"
				:style="canvasStyle"
			/>
		</div>
		<VSelect
			v-if="displays.length > 1 && hasFrame"
			v-model="currentDisplay"
			:items="displays"
			item-title="name"
			item-value="id"
			density="compact"
			variant="solo"
			hide-details
			prepend-inner-icon="mdi-monitor-multiple"
			class="display-picker"
			@update:model-value="switchDisplay"
		/>
		<div v-if="!hasFrame" class="waiting">
			<VProgressCircular
				indeterminate
				color="primary"
				size="48"
				class="mb-3"
			/>
			<p>{{ ended ? t("viewer.ended") : t("viewer.connecting") }}</p>
		</div>
		<VChip
			v-if="slow && hasFrame"
			class="poor"
			color="warning"
			size="small"
			variant="flat"
			prepend-icon="mdi-wifi-alert"
		>
			{{ t("viewer.poor") }}
		</VChip>
		<div v-if="hasFrame" class="zoom-controls">
			<VBtn
				:icon="panelOpen ? 'mdi-chevron-right' : 'mdi-chevron-left'"
				size="small"
				variant="text"
				:title="panelOpen ? t('viewer.collapse') : t('viewer.expand')"
				@click="panelOpen = !panelOpen"
			/>
			<template v-if="panelOpen">
				<VBtn
					icon="mdi-minus"
					size="small"
					variant="text"
					:disabled="zoom <= MIN_ZOOM"
					@click="zoomBy(-ZOOM_STEP)"
				/>
				<button
					type="button"
					class="zoom-label"
					:title="t('viewer.resetZoom')"
					@click="resetZoom"
				>
					{{ Math.round(zoom * 100) }}%
				</button>
				<VBtn
					icon="mdi-plus"
					size="small"
					variant="text"
					:disabled="zoom >= MAX_ZOOM"
					@click="zoomBy(ZOOM_STEP)"
				/>
				<div class="sep" />
				<VBtn
					icon="mdi-close-circle-outline"
					size="small"
					variant="text"
					color="error"
					:title="t('viewer.disconnect')"
					@click="disconnect"
				/>
			</template>
		</div>
	</div>
</template>

<script setup lang="ts">
	import {
		computed, onMounted, onUnmounted, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		listen, type UnlistenFn,
	} from "@tauri-apps/api/event";
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";

	type TilePatch = {
		i: number;
		jpeg_b64: string;
	};

	type DisplayInfo = {
		id: number;
		name: string;
	};

	type FrameDelta = {
		w: number;
		h: number;
		tile: number;
		keyframe: boolean;
		tiles: TilePatch[];
	};

	const { t } = useI18n();
	const canvasEl = ref<HTMLCanvasElement | null>(null);
	const hasFrame = ref(false);
	const ended = ref(false);
	const slow = ref(false);
	// Screens the host offers. Empty/one on Linux hosts (the portal already
	// picked a screen); two or more on macOS, where the picker can switch.
	const displays = ref<DisplayInfo[]>([]);
	const currentDisplay = ref<number | null>(null);

	// Zoom: 1 = fit the whole screen in the window (the default). Above that the
	// canvas overflows and the `.scroll` container pans. Useful for an oddly-
	// shaped source (e.g. a portrait monitor) that's tiny when letterboxed.
	const MIN_ZOOM = 0.25;
	const MAX_ZOOM = 5;
	const ZOOM_STEP = 0.25;
	const zoom = ref(1);
	// The floating control bar can be folded to a single chevron, TeamViewer-style.
	const panelOpen = ref(true);
	// The source frame's pixel size and the live window size — together they give
	// the "fit" scale that zoom multiplies.
	const frameW = ref(0);
	const frameH = ref(0);
	const winW = ref(window.innerWidth);
	const winH = ref(window.innerHeight);

	// Explicit display size = fit-to-window × zoom. At zoom 1 this matches an
	// object-fit:contain (one dimension fills the window); zooming in overflows
	// the scroll container so the user can pan.
	const canvasStyle = computed(() => {
		if (!frameW.value || !frameH.value) {
			return {};
		}
		const fit = Math.min(winW.value / frameW.value, winH.value / frameH.value);
		const scale = fit * zoom.value;
		return {
			width: `${Math.round(frameW.value * scale)}px`,
			height: `${Math.round(frameH.value * scale)}px`,
		};
	});

	function zoomBy(delta: number): void {
		zoom.value = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Math.round((zoom.value + delta) * 100) / 100));
	}

	function resetZoom(): void {
		zoom.value = 1;
	}

	// End the session from the viewer side. The backend stops the stream and
	// closes this window, so no local cleanup is needed here.
	async function disconnect(): Promise<void> {
		await invoke("lan_disconnect").catch(() => { /* already gone */ });
	}

	function onResize(): void {
		winW.value = window.innerWidth;
		winH.value = window.innerHeight;
	}

	let ctx: CanvasRenderingContext2D | null = null;
	let pending: Promise<void> = Promise.resolve();
	let unlistenFrame: UnlistenFn | null = null;
	let unlistenEnd: UnlistenFn | null = null;
	let unlistenDisplays: UnlistenFn | null = null;
	// The host sends a heartbeat frame ~every second; if nothing arrives for a
	// while the link is slow/stalled rather than just a static screen.
	let lastFrameAt = 0;
	let slowTimer: ReturnType<typeof setInterval> | undefined;
	const SLOW_AFTER_MS = 2000;

	function base64ToBytes(b64: string): Uint8Array {
		const bin = atob(b64);
		const bytes = new Uint8Array(bin.length);
		for (let i = 0; i < bin.length; i++) {
			bytes[i] = bin.charCodeAt(i);
		}
		return bytes;
	}

	async function applyDelta(delta: FrameDelta): Promise<void> {
		const canvas = canvasEl.value;
		if (!canvas) {
			return;
		}
		if (canvas.width !== delta.w || canvas.height !== delta.h) {
			canvas.width = delta.w;
			canvas.height = delta.h;
			// Drive the fit/zoom sizing off the real frame dimensions (they change
			// when the host switches to a differently-shaped screen).
			frameW.value = delta.w;
			frameH.value = delta.h;
		}
		if (!ctx) {
			ctx = canvas.getContext("2d");
		}
		const context = ctx;
		if (!context) {
			return;
		}

		const cols = Math.ceil(delta.w / delta.tile);
		await Promise.all(delta.tiles.map(async (patch) => {
			const blob = new Blob([base64ToBytes(patch.jpeg_b64)], { type: "image/jpeg" });
			const bitmap = await createImageBitmap(blob);
			const col = patch.i % cols;
			const row = Math.floor(patch.i / cols);
			context.drawImage(bitmap, col * delta.tile, row * delta.tile);
			bitmap.close();
		}));

		hasFrame.value = true;
		ended.value = false;
		lastFrameAt = performance.now();
		slow.value = false;
	}

	function clearCanvas(): void {
		const canvas = canvasEl.value;
		if (canvas && ctx) {
			ctx.clearRect(0, 0, canvas.width, canvas.height);
		}
	}

	async function switchDisplay(id: number | null): Promise<void> {
		if (id === null) {
			return;
		}
		await invoke("lan_switch_display", { display: id }).catch(() => { /* stream gone */ });
	}

	onMounted(async () => {
		unlistenFrame = await listen<FrameDelta>("lan://frame", (e) => {
			// Serialise frames so tile draws never interleave out of order.
			pending = pending.then(() => applyDelta(e.payload)).catch(() => { /* drop */ });
		});
		unlistenEnd = await listen("lan://disconnected", () => {
			ended.value = true;
			hasFrame.value = false;
			slow.value = false;
			clearCanvas(); // drop the last frame instead of leaving it frozen
			// The session is over — close this standalone viewer window.
			getCurrentWindow().close().catch(() => { /* already gone */ });
		});
		unlistenDisplays = await listen<DisplayInfo[]>("lan://displays", (e) => {
			displays.value = e.payload;
			// The stream opens on the host's first screen, so reflect that.
			if (e.payload.length > 0 && currentDisplay.value === null) {
				currentDisplay.value = e.payload[0].id;
			}
		});
		// Flag a slow link when no frame (not even a heartbeat) arrives in time.
		slowTimer = setInterval(() => {
			if (hasFrame.value && !ended.value) {
				slow.value = performance.now() - lastFrameAt > SLOW_AFTER_MS;
			}
		}, 500);
		window.addEventListener("resize", onResize);
	});

	onUnmounted(() => {
		unlistenFrame?.();
		unlistenEnd?.();
		unlistenDisplays?.();
		window.removeEventListener("resize", onResize);
		if (slowTimer) {
			clearInterval(slowTimer);
		}
	});
</script>

<style scoped>
	.viewer {
		position: relative;
		width: 100vw;
		height: 100vh;
		background: #000;
		overflow: hidden;
	}

	/* The scroll layer pans the canvas when it's zoomed past the window. As a
	   flex container it centers the canvas via the child's `margin: auto`, which
	   (unlike justify-content) collapses to 0 on overflow so panning never clips
	   the top/left edge. */
	.scroll {
		position: absolute;
		inset: 0;
		overflow: auto;
		display: flex;
	}

	.poor {
		position: absolute;
		right: 12px;
		bottom: 12px;
	}

	.zoom-controls {
		position: absolute;
		bottom: 12px;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 2px 4px;
		border-radius: 8px;
		background: rgba(0, 0, 0, 0.55);
		opacity: 0.85;
	}

	.sep {
		width: 1px;
		height: 20px;
		margin: 0 4px;
		background: rgba(255, 255, 255, 0.2);
	}

	.zoom-label {
		min-width: 48px;
		color: #fff;
		font-size: 0.8rem;
		text-align: center;
		background: none;
		border: none;
		cursor: pointer;
	}

	.display-picker {
		position: absolute;
		top: 12px;
		left: 50%;
		transform: translateX(-50%);
		width: 240px;
		max-width: 70vw;
		opacity: 0.85;
	}

	.screen {
		display: block;
		/* Auto margins center the canvas in the flex scroll layer and collapse on
		   overflow (no clipped edge when zoomed in). Size is the inline style. */
		margin: auto;
		flex: none;
	}

	.hidden {
		display: none;
	}

	.waiting {
		color: #fff;
		text-align: center;
	}
</style>
