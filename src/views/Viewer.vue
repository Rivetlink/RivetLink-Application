<template>
	<div class="viewer">
		<img
			v-if="frame"
			:src="frame"
			class="screen"
			alt=""
		>
		<div v-else class="waiting">
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

	const { t } = useI18n();
	const frame = ref<string | null>(null);
	const ended = ref(false);

	let unlistenFrame: UnlistenFn | null = null;
	let unlistenEnd: UnlistenFn | null = null;

	onMounted(async () => {
		unlistenFrame = await listen<string>("lan://frame", (e) => {
			frame.value = e.payload;
			ended.value = false;
		});
		unlistenEnd = await listen("lan://disconnected", () => {
			ended.value = true;
			frame.value = null;
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

.waiting {
	color: #fff;
	text-align: center;
}
</style>
