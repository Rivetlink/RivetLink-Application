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

	let ctx: CanvasRenderingContext2D | null = null;
	let pending: Promise<void> = Promise.resolve();
	let unlistenFrame: UnlistenFn | null = null;
	let unlistenEnd: UnlistenFn | null = null;

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
	}

	onMounted(async () => {
		unlistenFrame = await listen<FrameDelta>("lan://frame", (e) => {
			// Serialise frames so tile draws never interleave out of order.
			pending = pending.then(() => applyDelta(e.payload)).catch(() => { /* drop */ });
		});
		unlistenEnd = await listen("lan://disconnected", () => {
			ended.value = true;
			hasFrame.value = false;
		});
	});

	onUnmounted(() => {
		unlistenFrame?.();
		unlistenEnd?.();
	});
</script>

<style scoped>
	.viewer {
		width: 100vw;
		height: 100vh;
		background: #000;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
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
