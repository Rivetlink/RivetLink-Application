<template>
	<div class="frame" />
</template>

<script setup lang="ts">
	import { onMounted } from "vue";

	// Each Tauri window is its own document. The webview is transparent (set by
	// the window builder); force this document transparent too so only the red
	// frame paints and the host's desktop shows through the middle.
	onMounted(() => {
		const app = document.getElementById("app");
		for (const el of [document.documentElement, document.body, app]) {
			if (el) {
				el.style.background = "transparent";
				el.style.margin = "0";
				el.style.overflow = "hidden";
			}
		}
	});
</script>

<style scoped>
	/* Click-through is enforced on the OS window (set_ignore_cursor_events); the
	   pointer-events:none here is belt-and-braces so the frame never grabs input. */
	.frame {
		position: fixed;
		inset: 0;
		box-sizing: border-box;
		border: 5px solid #ff1744;
		pointer-events: none;
	}
</style>
