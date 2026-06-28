<template>
	<div class="viewer">
		<!-- The screen stage: the share fills it and every floating overlay (chips,
		     display picker, waiting state) sits over it. The toolbar below is a
		     separate row, so it never covers the shared screen. -->
		<div class="stage">
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
			<VChip
				v-if="controlling"
				class="controlling-chip"
				color="primary"
				size="small"
				variant="flat"
				prepend-icon="mdi-mouse"
			>
				{{ t("viewer.controlling") }}
			</VChip>
		</div>
		<!-- Always-on toolbar — never folds, so zoom/control/disconnect stay one
		     click away. Docked in its own row below the stage (no overlap). -->
		<div v-if="hasFrame" class="zoom-controls">
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
				:icon="controlling ? 'mdi-mouse' : 'mdi-mouse-off'"
				size="small"
				variant="text"
				:color="controlling ? 'primary' : undefined"
				:title="controlling ? t('viewer.releaseControl') : t('viewer.takeControl')"
				@click="toggleControl"
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
	// Height of the docked toolbar row, subtracted from the window when fitting the
	// frame so the screen share sits fully above it (the bar never overlaps it).
	const TOOLBAR_H = 52;
	// Remote control: when on, local mouse/keyboard over the canvas is captured and
	// forwarded to the host (which only acts on it if it granted control). The host
	// maps the platform command modifier itself, so we just flag ours as such.
	const controlling = ref(false);
	const isMac = navigator.platform.toUpperCase().includes("MAC");
	// Throttle pointer moves: only the latest position matters, and the host's
	// control channel shouldn't be flooded. ~120 Hz is smooth and cheap.
	const MOVE_INTERVAL_MS = 8;
	let lastMoveAt = 0;
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
		const stageH = Math.max(1, winH.value - TOOLBAR_H);
		const fit = Math.min(winW.value / frameW.value, stageH / frameH.value);
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

	// End the session from the viewer side and fully close this window, so a later
	// reconnect builds a brand-new window — GNOME/Wayland only reliably honours
	// always_on_top on a fresh window, so a reused one comes back up behind other
	// apps. The CloseRequested handler stops the stream on the backend.
	async function disconnect(): Promise<void> {
		await invoke("lan_disconnect").catch(() => { /* already gone */ });
		await getCurrentWindow().close().catch(() => { /* already gone */ });
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
	let closeTimer: ReturnType<typeof setTimeout> | undefined;
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
		// Frames are flowing again — if this window was lingering on a "connection
		// ended" screen (about to self-close in 5s) it's a live reconnect now, so
		// cancel that pending close instead of yanking the window mid-session.
		if (closeTimer !== undefined) {
			clearTimeout(closeTimer);
			closeTimer = undefined;
		}
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

	// Fire-and-forget: input is high-frequency and lossy by design (the backend
	// drops on a full queue), so never await or surface errors here.
	function sendInput(event: Record<string, unknown>): void {
		invoke("lan_send_input", { event }).catch(() => { /* stream gone */ });
	}

	// Map a pointer position to 0..10000 of the displayed frame (resolution-
	// independent). Returns null when the pointer is outside the screen image.
	function framePoint(e: PointerEvent): {
		x: number;
		y: number
	} | null {
		const canvas = canvasEl.value;
		if (!canvas) {
			return null;
		}
		const rect = canvas.getBoundingClientRect();
		if (rect.width === 0 || rect.height === 0) {
			return null;
		}
		const x = Math.round(((e.clientX - rect.left) / rect.width) * 10000);
		const y = Math.round(((e.clientY - rect.top) / rect.height) * 10000);
		if (x < 0 || x > 10000 || y < 0 || y > 10000) {
			return null;
		}
		return {
			x,
			y,
		};
	}

	function pointerButton(e: PointerEvent): string | null {
		switch (e.button) {
			case 0: return "left";
			case 1: return "middle";
			case 2: return "right";
			default: return null;
		}
	}

	function onPointerMove(e: PointerEvent): void {
		const now = performance.now();
		if (now - lastMoveAt < MOVE_INTERVAL_MS) {
			return;
		}
		const point = framePoint(e);
		if (point === null) {
			return;
		}
		lastMoveAt = now;
		sendInput({
			kind: "move",
			x: point.x,
			y: point.y,
		});
	}

	function onPointerDown(e: PointerEvent): void {
		const point = framePoint(e);
		const button = pointerButton(e);
		if (point === null || button === null) {
			return;
		}
		e.preventDefault();
		// Move first so the press lands exactly under the cursor.
		sendInput({
			kind: "move",
			x: point.x,
			y: point.y,
		});
		sendInput({
			kind: "button",
			button,
			down: true,
		});
	}

	function onPointerUp(e: PointerEvent): void {
		const button = pointerButton(e);
		if (button === null) {
			return;
		}
		e.preventDefault();
		sendInput({
			kind: "button",
			button,
			down: false,
		});
	}

	function onWheel(e: WheelEvent): void {
		e.preventDefault();
		// Browsers report deltas in pixels; the host wants wheel notches. One notch
		// per ~40px keeps fast flicks responsive without runaway scrolling.
		const dy = Math.trunc(e.deltaY / 40) || Math.sign(e.deltaY);
		const dx = Math.trunc(e.deltaX / 40) || Math.sign(e.deltaX);
		if (dx === 0 && dy === 0) {
			return;
		}
		sendInput({
			kind: "scroll",
			dx,
			dy,
		});
	}

	function onKey(e: KeyboardEvent, down: boolean): void {
		// Capture the key for the remote — don't let it trigger viewer shortcuts.
		e.preventDefault();
		let code = e.code;
		// Send the platform command modifier as a logical token; the host maps it
		// onto its own (⌘ on macOS, Ctrl elsewhere). On non-mac Ctrl IS that key.
		const isCommandMod = isMac
			? (code === "MetaLeft" || code === "MetaRight")
			: (code === "ControlLeft" || code === "ControlRight");
		if (isCommandMod) {
			code = "CommandMod";
		}
		sendInput({
			kind: "key",
			code,
			down,
		});
	}

	function onKeyDown(e: KeyboardEvent): void {
		onKey(e, true);
	}

	function onKeyUp(e: KeyboardEvent): void {
		onKey(e, false);
	}

	function onContextMenu(e: Event): void {
		e.preventDefault(); // right-click belongs to the remote while controlling
	}

	function attachControl(canvas: HTMLCanvasElement): void {
		canvas.addEventListener("pointermove", onPointerMove);
		canvas.addEventListener("pointerdown", onPointerDown);
		canvas.addEventListener("pointerup", onPointerUp);
		canvas.addEventListener("wheel", onWheel, { passive: false });
		canvas.addEventListener("contextmenu", onContextMenu);
		window.addEventListener("keydown", onKeyDown);
		window.addEventListener("keyup", onKeyUp);
	}

	function detachControl(canvas: HTMLCanvasElement | null): void {
		if (canvas) {
			canvas.removeEventListener("pointermove", onPointerMove);
			canvas.removeEventListener("pointerdown", onPointerDown);
			canvas.removeEventListener("pointerup", onPointerUp);
			canvas.removeEventListener("wheel", onWheel);
			canvas.removeEventListener("contextmenu", onContextMenu);
		}
		window.removeEventListener("keydown", onKeyDown);
		window.removeEventListener("keyup", onKeyUp);
	}

	function toggleControl(): void {
		const canvas = canvasEl.value;
		if (!canvas) {
			return;
		}
		controlling.value = !controlling.value;
		if (controlling.value) {
			attachControl(canvas);
		} else {
			detachControl(canvas);
		}
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
			// Stop capturing input — there's no host to drive anymore.
			if (controlling.value) {
				controlling.value = false;
				detachControl(canvasEl.value);
			}
			clearCanvas(); // drop the last frame instead of leaving it frozen
			// Keep the "connection ended" message up briefly so the viewer sees
			// what happened (e.g. the host hung up) before the window closes.
			if (closeTimer === undefined) {
				closeTimer = setTimeout(() => {
					getCurrentWindow().close().catch(() => { /* already gone */ });
				}, 5000);
			}
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
		// Raise above every app now the window is mapped — GNOME/Wayland honours
		// always_on_top reliably here but not always at build time, so a reconnect
		// (fresh window) would otherwise come up behind other apps.
		void invoke("viewer_raise").catch(() => { /* window gone */ });
	});

	onUnmounted(() => {
		unlistenFrame?.();
		unlistenEnd?.();
		unlistenDisplays?.();
		detachControl(canvasEl.value); // drop any input listeners
		window.removeEventListener("resize", onResize);
		if (slowTimer) {
			clearInterval(slowTimer);
		}
		if (closeTimer) {
			clearTimeout(closeTimer);
		}
	});
</script>

<style scoped>
	.viewer {
		display: flex;
		flex-direction: column;
		width: 100vw;
		height: 100vh;
		background: #000;
		overflow: hidden;
	}

	/* The screen stage takes all the height the toolbar doesn't. Positioning
	   context for the floating overlays (chips, picker, waiting) so they sit over
	   the share and never over the toolbar. */
	.stage {
		position: relative;
		flex: 1;
		min-height: 0;
	}

	/* The scroll layer pans the canvas when it's zoomed past the stage. As a
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

	.controlling-chip {
		position: absolute;
		top: 12px;
		left: 12px;
		opacity: 0.9;
	}

	/* Docked toolbar row below the stage — a real layout row, not an overlay, so
	   it can never cover the shared screen. */
	.zoom-controls {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 2px;
		padding: 4px;
		background: rgba(0, 0, 0, 0.85);
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
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		color: #fff;
		text-align: center;
		pointer-events: none;
	}
</style>
