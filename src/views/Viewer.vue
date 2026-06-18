<template>
	<div class="viewer">
		<canvas ref="canvasEl" class="screen" :class="{ hidden: !hasFrame }" />
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
	</div>
</template>

<script setup lang="ts">
	import {
		onMounted, onUnmounted, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		listen, type UnlistenFn,
	} from "@tauri-apps/api/event";
	import { getCurrentWindow } from "@tauri-apps/api/window";

	type TilePatch = {
		i: number;
		jpeg_b64: string;
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

	let ctx: CanvasRenderingContext2D | null = null;
	let pending: Promise<void> = Promise.resolve();
	let unlistenFrame: UnlistenFn | null = null;
	let unlistenEnd: UnlistenFn | null = null;
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
		// Flag a slow link when no frame (not even a heartbeat) arrives in time.
		slowTimer = setInterval(() => {
			if (hasFrame.value && !ended.value) {
				slow.value = performance.now() - lastFrameAt > SLOW_AFTER_MS;
			}
		}, 500);
	});

	onUnmounted(() => {
		unlistenFrame?.();
		unlistenEnd?.();
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
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}

	.poor {
		position: absolute;
		right: 12px;
		bottom: 12px;
	}

	.screen {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	.hidden {
		display: none;
	}

	.waiting {
		color: #fff;
		text-align: center;
	}
</style>
