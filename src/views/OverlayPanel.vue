<template>
	<div class="badge" :class="{ collapsed }">
		<span class="dot" />
		<template v-if="!collapsed">
			<span class="label">{{ peer || t("overlay.sharing") }}</span>
			<i
				class="mdi screens"
				:class="shareAll ? 'mdi-monitor-multiple' : 'mdi-monitor'"
				:title="shareAll ? t('overlay.allScreens') : t('overlay.oneScreen')"
			/>
		</template>
		<button
			type="button"
			class="toggle"
			:title="collapsed ? t('overlay.expand') : t('overlay.collapse')"
			@click="toggle"
		>
			<i class="mdi" :class="collapsed ? 'mdi-chevron-left' : 'mdi-chevron-right'" />
		</button>
	</div>
</template>

<script setup lang="ts">
	import {
		onMounted, onUnmounted, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import { invoke } from "@tauri-apps/api/core";
	import {
		listen, type UnlistenFn,
	} from "@tauri-apps/api/event";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { LogicalSize } from "@tauri-apps/api/dpi";

	type HostState = {
		pin: string | null;
		peer: string | null;
		share_all: boolean;
	};

	const { t } = useI18n();
	const peer = ref<string | null>(null);
	const shareAll = ref(true);
	const collapsed = ref(false);

	let unlistenConnected: UnlistenFn | null = null;
	let unlistenDisconnected: UnlistenFn | null = null;
	let unlistenShareAll: UnlistenFn | null = null;

	// Collapse folds the badge to a small tab against the screen edge. Resize the
	// OS window to match so its transparent area never becomes an invisible
	// click-trap. (Position is fixed by the compositor on Wayland — fine.)
	async function fit(): Promise<void> {
		const width = collapsed.value ? 56 : 230;
		try {
			await getCurrentWindow().setSize(new LogicalSize(width, 44));
		} catch {
			// Resize unsupported — the badge just keeps the builder's size.
		}
	}

	function toggle(): void {
		collapsed.value = !collapsed.value;
		void fit();
	}

	onMounted(async () => {
		for (const el of [document.documentElement, document.body]) {
			el.style.background = "transparent";
			el.style.margin = "0";
			el.style.overflow = "hidden";
		}
		await fit();
		// The connect event can fire before this window mounts, so pull the live
		// state up front, then track changes.
		try {
			const state = await invoke<HostState>("host_active");
			peer.value = state.peer;
			shareAll.value = state.share_all;
		} catch {
			// Backend unavailable — the badge still shows the generic label.
		}
		unlistenConnected = await listen<string>("host://connected", (e) => {
			peer.value = e.payload;
		});
		unlistenDisconnected = await listen("host://disconnected", () => {
			peer.value = null;
		});
		unlistenShareAll = await listen<boolean>("host://share-all", (e) => {
			shareAll.value = e.payload;
		});
	});

	onUnmounted(() => {
		unlistenConnected?.();
		unlistenDisconnected?.();
		unlistenShareAll?.();
	});
</script>

<style scoped>
	.badge {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 44px;
		padding: 0 8px 0 12px;
		box-sizing: border-box;
		border-radius: 10px;
		background: rgba(20, 20, 22, 0.92);
		color: #fff;
		font-family: system-ui, sans-serif;
		font-size: 0.85rem;
		white-space: nowrap;
		overflow: hidden;
		user-select: none;
	}

	.dot {
		flex: none;
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: #ff1744;
		box-shadow: 0 0 6px #ff1744;
		animation: pulse 1.6s ease-in-out infinite;
	}

	@keyframes pulse {
		50% {
			opacity: 0.35;
		}
	}

	.label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.screens {
		flex: none;
		font-size: 1.1rem;
		opacity: 0.7;
	}

	/* Only this collapse control is interactive — everything else is a label. */
	.toggle {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		color: #fff;
		background: rgba(255, 255, 255, 0.08);
		border: none;
		border-radius: 6px;
		cursor: pointer;
	}

	.toggle:hover {
		background: rgba(255, 255, 255, 0.18);
	}

	.toggle .mdi {
		font-size: 1.2rem;
	}
</style>
