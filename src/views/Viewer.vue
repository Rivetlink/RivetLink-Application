<template>
	<div class="viewer">
		<!-- The screen stage: the share fills it and every floating overlay (chips,
		     display picker, waiting state) sits over it. The toolbar below is a
		     separate row, so it never covers the shared screen. -->
		<div ref="stageEl" class="stage">
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
				<p>{{ errorMessage ? t("viewer.error", { error: errorMessage }) : (ended ? t("viewer.ended") : t("viewer.connecting")) }}</p>
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
			<VChip
				v-if="consoleHandoff"
				class="handoff-chip"
				color="info"
				size="small"
				variant="flat"
				prepend-icon="mdi-login-variant"
			>
				{{ t("viewer.loginHandoff") }}
			</VChip>
			<VChip
				v-if="clipboardReceived"
				class="clipboard-chip"
				color="success"
				size="small"
				variant="flat"
				prepend-icon="mdi-clipboard-check-outline"
			>
				{{ t("share.clipboardReceived") }}
			</VChip>
			<VChip
				v-if="dropRefused"
				class="clipboard-chip"
				color="warning"
				size="small"
				variant="flat"
				prepend-icon="mdi-folder-alert-outline"
			>
				{{ t("share.unavailable") }}
			</VChip>
			<SharePanel
				v-if="showShare && sharing?.files"
				:clipboard="sharing.clipboard"
				@close="showShare = false"
			/>
		</div>
		<!-- Always-on toolbar — never folds, so zoom/control/disconnect stay one
		     click away. Docked in its own row below the stage (no overlap). -->
		<div v-if="hasFrame" class="zoom-controls">
			<div class="tools">
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
				<template v-if="sharing?.files">
					<div class="sep" />
					<VBtn
						icon="mdi-folder-swap-outline"
						size="small"
						variant="text"
						:color="showShare ? 'primary' : undefined"
						:title="t('share.title')"
						@click="showShare = !showShare"
					/>
				</template>
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
			<span class="stats" :title="t('viewer.statsHint')">{{ statsLabel }}</span>
		</div>
	</div>
</template>

<script setup lang="ts">
	import {
		computed, markRaw, onMounted, onUnmounted, ref, shallowRef,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		listen, type UnlistenFn,
	} from "@tauri-apps/api/event";
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { getCurrentWebview } from "@tauri-apps/api/webview";
	import SharePanel from "../components/SharePanel.vue";
	import { H264_CODEC } from "../video";
	import {
		store, type ScrollSpeed,
	} from "../store";

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

	// One H.264 access unit from the host (Annex-B, base64).
	type VideoPacket = {
		w: number;
		h: number;
		keyframe: boolean;
		data_b64: string;
	};

	type SharingState = {
		clipboard: boolean;
		files: boolean;
	};

	type ConsoleFrame = {
		image: string;
		consoleState: string | null;
	};

	const { t } = useI18n();
	const canvasEl = ref<HTMLCanvasElement | null>(null);
	const stageEl = ref<HTMLElement | null>(null);
	const hasFrame = ref(false);
	const ended = ref(false);
	// A console capture can be authenticated and connected while the host's
	// compositor refuses to provide a frame (notably GDM lockdown). Keep the
	// actual backend error visible instead of leaving the user on a spinner.
	const errorMessage = ref<string | null>(null);
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
	// Remote control: when on, local mouse/keyboard over the canvas is captured and
	// forwarded to the host (which only acts on it if it granted control). The host
	// maps the platform command modifier itself, so we just flag ours as such.
	const controlling = ref(false);
	const isMac = navigator.platform.toUpperCase().includes("MAC");
	// Throttle pointer moves: only the latest position matters, and the host's
	// control channel shouldn't be flooded. ~120 Hz is smooth and cheap.
	const MOVE_INTERVAL_MS = 8;
	const lastMoveAt = ref(0);
	// Convert the browser's high-resolution trackpad deltas into the discrete
	// wheel steps understood by both remote input backends. In particular, do
	// not turn every tiny macOS trackpad event into a full wheel click.
	const WHEEL_PIXELS_PER_STEP: Record<ScrollSpeed, number> = {
		slow: 100,
		normal: 60,
		fast: 30,
	};
	const wheelRemainderX = ref(0);
	const wheelRemainderY = ref(0);
	// The source frame's pixel size and the live window size — together they give
	// the "fit" scale that zoom multiplies.
	const frameW = ref(0);
	const frameH = ref(0);
	const stageW = ref(window.innerWidth);
	const stageH = ref(window.innerHeight);
	const stageObserver = ref<ResizeObserver | null>(null);

	// Explicit display size = fit-to-window × zoom. At zoom 1 this matches an
	// object-fit:contain (one dimension fills the window); zooming in overflows
	// the scroll container so the user can pan.
	const canvasStyle = computed(() => {
		if (!frameW.value || !frameH.value) {
			return {};
		}
		const fit = Math.min(stageW.value / frameW.value, stageH.value / frameH.value);
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
	// reconnect builds a brand-new window. The CloseRequested handler stops the
	// stream on the backend.
	async function disconnect(): Promise<void> {
		await invoke("lan_disconnect").catch(() => { /* already gone */ });
		await getCurrentWindow().close().catch(() => { /* already gone */ });
	}

	function onResize(): void {
		const rect = stageEl.value?.getBoundingClientRect();
		stageW.value = Math.max(1, rect?.width ?? window.innerWidth);
		stageH.value = Math.max(1, rect?.height ?? window.innerHeight);
	}

	// shallowRef (not ref) for the canvas context and the frame-chain promise:
	// a deep reactive proxy of a CanvasRenderingContext2D / Promise breaks their
	// internal-slot methods (drawImage / .then). These never drive the template.
	const ctx = shallowRef<CanvasRenderingContext2D | null>(null);
	const pending = shallowRef<Promise<void>>(Promise.resolve());
	const unlistenFrame = ref<UnlistenFn | null>(null);
	const unlistenVideo = ref<UnlistenFn | null>(null);
	const unlistenConsoleFrame = ref<UnlistenFn | null>(null);
	const unlistenEnd = ref<UnlistenFn | null>(null);
	const unlistenError = ref<UnlistenFn | null>(null);
	const unlistenConsoleHandoff = ref<UnlistenFn | null>(null);
	const unlistenConsoleUpgrade = ref<UnlistenFn | null>(null);
	const unlistenDesktopFallback = ref<UnlistenFn | null>(null);
	const unlistenDisplays = ref<UnlistenFn | null>(null);
	const unlistenBlur = ref<UnlistenFn | null>(null);
	const unlistenSharing = ref<UnlistenFn | null>(null);
	const unlistenClipboard = ref<UnlistenFn | null>(null);
	const unlistenDrop = ref<UnlistenFn | null>(null);

	// Clipboard/file sharing: set once the host enabled it for this stream.
	const sharing = ref<SharingState | null>(null);
	const showShare = ref(false);
	// Briefly confirm that text copied on the host is now on our clipboard.
	const clipboardReceived = ref(false);
	const clipboardTimer = ref<ReturnType<typeof setTimeout> | undefined>(undefined);
	// A drop while the host doesn't share files: say so instead of ignoring it.
	const dropRefused = ref(false);
	const dropTimer = ref<ReturnType<typeof setTimeout> | undefined>(undefined);
	// The host sends a heartbeat frame ~every second; if nothing arrives for a
	// while the link is slow/stalled rather than just a static screen.
	const lastFrameAt = ref(0);
	const slowTimer = ref<ReturnType<typeof setInterval>>();
	const closeTimer = ref<ReturnType<typeof setTimeout>>();
	const SLOW_AFTER_MS = 2000;
	const consoleHandoff = ref(false);

	type ConsoleUpgradeTarget = {
		name: string;
		address: string;
		port: number;
		deviceId: string;
		publicKey?: string;
	};

	function base64ToBytes(b64: string): Uint8Array {
		const bin = atob(b64);
		const bytes = new Uint8Array(bin.length);
		for (let i = 0; i < bin.length; i++) {
			bytes[i] = bin.charCodeAt(i);
		}
		return bytes;
	}

	// Size the canvas to the stream's frame and return its 2D context.
	function canvasFor(w: number, h: number): CanvasRenderingContext2D | null {
		const canvas = canvasEl.value;
		if (!canvas) {
			return null;
		}
		if (canvas.width !== w || canvas.height !== h) {
			canvas.width = w;
			canvas.height = h;
			// Drive the fit/zoom sizing off the real frame dimensions (they change
			// when the host switches to a differently-shaped screen).
			frameW.value = w;
			frameH.value = h;
		}
		if (!ctx.value) {
			ctx.value = canvas.getContext("2d");
		}
		return ctx.value;
	}

	async function applyDelta(delta: FrameDelta): Promise<void> {
		const context = canvasFor(delta.w, delta.h);
		if (!context) {
			return;
		}

		countFrame("JPEG", delta.tiles.reduce((n, patch) => n + patch.jpeg_b64.length, 0));
		const cols = Math.ceil(delta.w / delta.tile);
		await Promise.all(delta.tiles.map(async (patch) => {
			const blob = new Blob([base64ToBytes(patch.jpeg_b64)], { type: "image/jpeg" });
			const bitmap = await createImageBitmap(blob);
			const col = patch.i % cols;
			const row = Math.floor(patch.i / cols);
			context.drawImage(bitmap, col * delta.tile, row * delta.tile);
			bitmap.close();
		}));
		frameShown();
	}

	// A frame reached the screen: clear waiting/error states.
	function frameShown(): void {
		hasFrame.value = true;
		ended.value = false;
		consoleHandoff.value = false;
		errorMessage.value = null;
		// Frames are flowing again — if this window was lingering on a "connection
		// ended" screen (about to self-close in 5s) it's a live reconnect now, so
		// cancel that pending close instead of yanking the window mid-session.
		if (closeTimer.value !== undefined) {
			clearTimeout(closeTimer.value);
			closeTimer.value = undefined;
		}
		lastFrameAt.value = performance.now();
		slow.value = false;
	}

	// H.264 via WebCodecs. Deltas only decode after their keyframe, so after a
	// decoder error (or before the first keyframe) ask the host for a new one
	// and drop deltas until it arrives.
	const decoder = shallowRef<VideoDecoder | null>(null);
	const videoTimestamp = ref(0);
	const lastKeyframeRequest = ref(0);
	const KEYFRAME_RETRY_MS = 1000;
	// Decoder errors in a row with no frame shown in between. At the limit the
	// webview's H.264 is treated as broken: stop asking for keyframes and make
	// the next connection fall back to JPEG tiles.
	const videoFailures = ref(0);
	const MAX_VIDEO_FAILURES = 3;

	function requestKeyframe(): void {
		const now = performance.now();
		if (now - lastKeyframeRequest.value < KEYFRAME_RETRY_MS) {
			return;
		}
		lastKeyframeRequest.value = now;
		void invoke("lan_request_keyframe").catch(() => { /* stream gone */ });
	}

	function closeDecoder(): void {
		if (decoder.value && decoder.value.state !== "closed") {
			decoder.value.close();
		}
		decoder.value = null;
	}

	function videoFailed(): void {
		closeDecoder();
		videoFailures.value += 1;
		if (videoFailures.value < MAX_VIDEO_FAILURES) {
			requestKeyframe();
			return;
		}
		void invoke("lan_set_decoders", { codecs: [] }).catch(() => { /* app closing */ });
		hasFrame.value = false;
		errorMessage.value = t("viewer.videoFailed");
	}

	function drawVideoFrame(frame: VideoFrame): void {
		videoFailures.value = 0;
		const context = canvasFor(frame.displayWidth, frame.displayHeight);
		context?.drawImage(frame, 0, 0);
		frame.close();
		frameShown();
	}

	function applyVideo(packet: VideoPacket): void {
		countFrame("H.264", packet.data_b64.length);
		if (videoFailures.value >= MAX_VIDEO_FAILURES) {
			return;
		}
		if (!decoder.value) {
			if (!packet.keyframe) {
				requestKeyframe();
				return;
			}
			const next = new VideoDecoder({
				output: drawVideoFrame,
				error: () => {
					// A late error from a decoder already replaced is moot.
					if (decoder.value === next) {
						videoFailed();
					}
				},
			});
			next.configure({
				codec: H264_CODEC,
				optimizeForLatency: true,
			});
			decoder.value = next;
		}
		try {
			decoder.value.decode(new EncodedVideoChunk({
				type: packet.keyframe ? "key" : "delta",
				timestamp: videoTimestamp.value,
				data: base64ToBytes(packet.data_b64),
			}));
			videoTimestamp.value += 33_333;
		} catch {
			videoFailed();
		}
	}

	// Toolbar readout: codec, frames and (base64) megabits per second.
	const statsLabel = ref("");
	const statsCodec = ref("");
	const statsFrames = ref(0);
	const statsChars = ref(0);
	const statsSince = ref(performance.now());

	function countFrame(codec: string, chars: number): void {
		statsCodec.value = codec;
		statsFrames.value += 1;
		statsChars.value += chars;
	}

	function updateStats(): void {
		const seconds = (performance.now() - statsSince.value) / 1000;
		if (seconds < 1) {
			return;
		}
		// Base64 carries 6 bits per character.
		const mbps = (statsChars.value * 6) / seconds / 1_000_000;
		const fps = Math.round(statsFrames.value / seconds);
		statsLabel.value = statsCodec.value
			? `${statsCodec.value} · ${fps} fps · ${mbps.toFixed(1)} Mb/s`
			: "";
		statsFrames.value = 0;
		statsChars.value = 0;
		statsSince.value = performance.now();
	}

	async function applyConsoleFrame(frame: ConsoleFrame): Promise<void> {
		const canvas = canvasEl.value;
		if (!canvas) {
			return;
		}
		const encoded = frame.image.split(",", 2)[1];
		if (!encoded) {
			return;
		}
		const bitmap = await createImageBitmap(new Blob([base64ToBytes(encoded)], { type: "image/png" }));
		canvas.width = bitmap.width;
		canvas.height = bitmap.height;
		frameW.value = bitmap.width;
		frameH.value = bitmap.height;
		if (!ctx.value) {
			ctx.value = canvas.getContext("2d");
		}
		ctx.value?.drawImage(bitmap, 0, 0);
		bitmap.close();
		hasFrame.value = true;
		ended.value = false;
		consoleHandoff.value = false;
		errorMessage.value = null;
		lastFrameAt.value = performance.now();
		slow.value = false;
	}

	function clearCanvas(): void {
		const canvas = canvasEl.value;
		if (canvas && ctx.value) {
			ctx.value.clearRect(0, 0, canvas.width, canvas.height);
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
		if (now - lastMoveAt.value < MOVE_INTERVAL_MS) {
			return;
		}
		const point = framePoint(e);
		if (point === null) {
			return;
		}
		lastMoveAt.value = now;
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
		syncModifiers(e);
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
		const dx = consumeWheelDelta(normalizeWheelDelta(e.deltaX, e.deltaMode), "x");
		const dy = consumeWheelDelta(normalizeWheelDelta(e.deltaY, e.deltaMode), "y");
		if (dx === 0 && dy === 0) {
			return;
		}
		sendInput({
			kind: "scroll",
			dx,
			dy,
		});
	}

	function normalizeWheelDelta(delta: number, mode: number): number {
		if (mode === WheelEvent.DOM_DELTA_LINE) {
			return delta * 40;
		}
		if (mode === WheelEvent.DOM_DELTA_PAGE) {
			return delta * 800;
		}
		return delta;
	}

	function consumeWheelDelta(delta: number, axis: "x" | "y"): number {
		const remainder = axis === "x" ? wheelRemainderX : wheelRemainderY;
		// A direction change must not first pay off a stale partial gesture in the
		// opposite direction; that would make the trackpad feel briefly unresponsive.
		if (remainder.value !== 0 && delta !== 0 && Math.sign(remainder.value) !== Math.sign(delta)) {
			remainder.value = 0;
		}
		const threshold = WHEEL_PIXELS_PER_STEP[store.settings.scroll_speed] ?? WHEEL_PIXELS_PER_STEP.slow;
		const total = remainder.value + delta;
		const unboundedSteps = Math.trunc(total / threshold);
		remainder.value = total - (unboundedSteps * threshold);
		// Match the host-side safety limit and discard implausibly large synthetic
		// wheel bursts instead of overflowing the signed protocol field.
		return Math.max(-120, Math.min(120, unboundedSteps));
	}

	// Keys the host currently holds down. A key released while this window
	// has no focus (Alt+Tab, the Windows key, a dialog) never sends its keyup,
	// so the host would keep it pressed and turn later letters into shortcuts.
	const heldKeys = new Set<string>();

	function releaseHeldKeys(): void {
		heldKeys.forEach((code) => {
			sendInput({
				kind: "key",
				code,
				down: false,
			});
		});
		heldKeys.clear();
	}

	// Which event flag says a held modifier is really still down.
	function modifierFlag(code: string): "shiftKey" | "altKey" | "ctrlKey" | "metaKey" | null {
		if (code === "ShiftLeft" || code === "ShiftRight") {
			return "shiftKey";
		}
		if (code === "AltLeft" || code === "AltRight") {
			return "altKey";
		}
		if (code === "ControlLeft" || code === "ControlRight") {
			return "ctrlKey";
		}
		if (code === "MetaLeft" || code === "MetaRight") {
			return "metaKey";
		}
		if (code === "CommandMod") {
			return isMac ? "metaKey" : "ctrlKey";
		}
		return null;
	}

	// Release modifiers the host holds but the OS reports as up. Catches a
	// keyup the viewer never saw even without a focus change (e.g. the
	// Windows key, whose release the Start menu swallows).
	function syncModifiers(e: MouseEvent | KeyboardEvent): void {
		heldKeys.forEach((code) => {
			const flag = modifierFlag(code);
			if (flag && !e[flag]) {
				heldKeys.delete(code);
				sendInput({
					kind: "key",
					code,
					down: false,
				});
			}
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
		syncModifiers(e);
		if (down) {
			heldKeys.add(code);
		} else if (!heldKeys.delete(code)) {
			// Its keydown went elsewhere (e.g. the shortcut that focused us).
			return;
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
		window.addEventListener("blur", releaseHeldKeys);
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
		window.removeEventListener("blur", releaseHeldKeys);
		releaseHeldKeys();
		wheelRemainderX.value = 0;
		wheelRemainderY.value = 0;
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
		unlistenFrame.value = await listen<FrameDelta>("lan://frame", (e) => {
			// Serialise frames so tile draws never interleave out of order.
			pending.value = pending.value.then(() => applyDelta(e.payload)).catch(() => { /* drop */ });
		});
		unlistenVideo.value = await listen<VideoPacket>("lan://video", (e) => {
			// The decoder queues work itself and delivers frames in order.
			applyVideo(e.payload);
		});
		unlistenConsoleFrame.value = await listen<ConsoleFrame>("lan://console-frame", (e) => {
			pending.value = pending.value.then(() => applyConsoleFrame(e.payload)).catch(() => { /* drop */ });
		});
		unlistenError.value = await listen<string>("lan://error", (e) => {
			errorMessage.value = e.payload || "unknown error";
		});
		unlistenConsoleHandoff.value = await listen("lan://console-handoff", () => {
			consoleHandoff.value = true;
			// Keyboard events immediately after Enter must not leak into the new
			// desktop worker while the authenticated session is being replaced.
			if (controlling.value) {
				controlling.value = false;
				detachControl(canvasEl.value);
			}
		});
		unlistenConsoleUpgrade.value = await listen<ConsoleUpgradeTarget>("lan://console-upgrade", (e) => {
			consoleHandoff.value = true;
			errorMessage.value = null;
			// Keep this viewer window and its input surface alive. The backend
			// swaps only the authenticated video/control sub-session from the
			// LightDM physical console to the normal desktop stream.
			void invoke("lan_console_upgrade", { target: e.payload }).catch((error: unknown) => {
				errorMessage.value = String(error);
			});
		});
		unlistenDesktopFallback.value = await listen<ConsoleUpgradeTarget>("lan://desktop-fallback", (e) => {
			consoleHandoff.value = true;
			errorMessage.value = null;
			// Logout replaces the GNOME worker with LightDM. Re-open the same
			// trusted physical-console route automatically; its dedicated viewer
			// setup safely rebuilds the window if the platform requires it.
			void invoke("lan_console_connect", { target: e.payload }).catch((error: unknown) => {
				errorMessage.value = String(error);
			});
		});
		unlistenSharing.value = await listen<SharingState>("lan://sharing", (e) => {
			sharing.value = e.payload;
		});
		// The host may have answered before this window started listening.
		void invoke<SharingState | null>("lan_share_state").then((state) => {
			if (state) {
				sharing.value = state;
			}
		}).catch(() => { /* no stream */ });
		unlistenClipboard.value = await listen("lan://clipboard", () => {
			clipboardReceived.value = true;
			clearTimeout(clipboardTimer.value);
			clearTimeout(dropTimer.value);
			clipboardTimer.value = setTimeout(() => {
				clipboardReceived.value = false;
			}, 2000);
		});
		// Dropping files/folders on the viewer sends them to the host.
		unlistenDrop.value = await getCurrentWebview().onDragDropEvent((e) => {
			if (e.payload.type !== "drop" || !e.payload.paths.length) {
				return;
			}
			if (!sharing.value?.files) {
				dropRefused.value = true;
				clearTimeout(dropTimer.value);
				dropTimer.value = setTimeout(() => {
					dropRefused.value = false;
				}, 4000);
				return;
			}
			showShare.value = true;
			void invoke("lan_share_upload", { paths: e.payload.paths }).catch(() => { /* stream gone */ });
		});
		unlistenEnd.value = await listen("lan://disconnected", () => {
			ended.value = true;
			closeDecoder();
			sharing.value = null;
			showShare.value = false;
			hasFrame.value = false;
			slow.value = false;
			consoleHandoff.value = false;
			// Stop capturing input — there's no host to drive anymore.
			if (controlling.value) {
				controlling.value = false;
				detachControl(canvasEl.value);
			}
			clearCanvas(); // drop the last frame instead of leaving it frozen
			// Keep the "connection ended" message up briefly so the viewer sees
			// what happened (e.g. the host hung up) before the window closes.
			if (closeTimer.value === undefined) {
				closeTimer.value = setTimeout(() => {
					getCurrentWindow().close().catch(() => { /* already gone */ });
				}, 5000);
			}
		});
		unlistenDisplays.value = await listen<DisplayInfo[]>("lan://displays", (e) => {
			displays.value = e.payload;
			// The stream opens on the host's first screen, so reflect that.
			if (e.payload.length > 0 && currentDisplay.value === null) {
				currentDisplay.value = e.payload[0].id;
			}
		});
		// Flag a slow link when no frame (not even a heartbeat) arrives in time.
		slowTimer.value = setInterval(() => {
			if (hasFrame.value && !ended.value) {
				slow.value = performance.now() - lastFrameAt.value > SLOW_AFTER_MS;
			}
			updateStats();
		}, 500);
		window.addEventListener("resize", onResize);
		onResize();
		stageObserver.value = markRaw(new ResizeObserver(onResize));
		if (stageEl.value) {
			stageObserver.value.observe(stageEl.value);
		}
		// Bring the new window to the front once now it is mapped, so a
		// reconnect doesn't open behind other apps. It stays a normal window.
		void invoke("viewer_raise").catch(() => { /* window gone */ });
		unlistenBlur.value = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
			if (!focused) {
				releaseHeldKeys();
			}
		});
	});

	onUnmounted(() => {
		unlistenFrame.value?.();
		unlistenVideo.value?.();
		closeDecoder();
		unlistenConsoleFrame.value?.();
		unlistenEnd.value?.();
		unlistenError.value?.();
		unlistenConsoleHandoff.value?.();
		unlistenConsoleUpgrade.value?.();
		unlistenDesktopFallback.value?.();
		unlistenDisplays.value?.();
		unlistenBlur.value?.();
		unlistenSharing.value?.();
		unlistenClipboard.value?.();
		unlistenDrop.value?.();
		clearTimeout(clipboardTimer.value);
		detachControl(canvasEl.value); // drop any input listeners
		window.removeEventListener("resize", onResize);
		stageObserver.value?.disconnect();
		stageObserver.value = null;
		if (slowTimer.value) {
			clearInterval(slowTimer.value);
		}
		if (closeTimer.value) {
			clearTimeout(closeTimer.value);
		}
	});
</script>

<style scoped>
	.viewer {
		position: fixed;
		inset: 0;
		display: flex;
		flex-direction: column;
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

	.handoff-chip {
		position: absolute;
		top: 12px;
		right: 12px;
		opacity: 0.9;
	}

	.clipboard-chip {
		position: absolute;
		bottom: 12px;
		left: 50%;
		transform: translateX(-50%);
		opacity: 0.9;
	}

	/* Docked toolbar row below the stage — a real layout row, not an overlay, so
	   it can never cover the shared screen. Three columns keep the tools centered
	   and the stats at the right edge without overlapping. */
	.zoom-controls {
		flex: none;
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		padding: 4px;
		background: rgba(0, 0, 0, 0.85);
	}

	.tools {
		grid-column: 2;
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.sep {
		width: 1px;
		height: 20px;
		margin: 0 4px;
		background: rgba(255, 255, 255, 0.2);
	}

	.stats {
		justify-self: end;
		padding-right: 8px;
		color: rgba(255, 255, 255, 0.6);
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
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
