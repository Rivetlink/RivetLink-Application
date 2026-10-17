<template>
	<VCard
		class="share-panel"
		elevation="8"
	>
		<VCardTitle class="d-flex align-center">
			<VIcon
				icon="mdi-folder-swap-outline"
				class="mr-2"
			/>
			{{ t("share.title") }}
			<VSpacer />
			<VBtn
				icon="mdi-close"
				size="small"
				variant="text"
				@click="emit('close')"
			/>
		</VCardTitle>
		<VCardText>
			<p class="text-caption mb-2">
				{{ clipboard ? t("share.clipboardOn") : t("share.clipboardOff") }}
			</p>

			<div class="section-title">
				{{ t("share.send") }}
			</div>
			<div class="d-flex ga-2 mb-1">
				<VBtn
					size="small"
					variant="tonal"
					prepend-icon="mdi-file-upload-outline"
					@click="pick(false)"
				>
					{{ t("share.files") }}
				</VBtn>
				<VBtn
					size="small"
					variant="tonal"
					prepend-icon="mdi-folder-upload-outline"
					@click="pick(true)"
				>
					{{ t("share.folder") }}
				</VBtn>
			</div>
			<p class="text-caption mb-3">
				{{ t("share.sendHint") }}
			</p>

			<div class="section-title d-flex align-center">
				{{ t("share.hostFiles") }}
				<VSpacer />
				<VBtn
					icon="mdi-refresh"
					size="x-small"
					variant="text"
					:title="t('share.refresh')"
					@click="refresh"
				/>
			</div>
			<div class="host-list">
				<p
					v-if="entries.length === 0"
					class="text-caption"
				>
					{{ t("share.hostEmpty") }}
				</p>
				<div
					v-for="entry in entries"
					:key="entry.path"
					class="entry"
					:style="{ paddingLeft: `${depth(entry.path) * 12}px` }"
				>
					<VIcon
						:icon="entry.dir ? 'mdi-folder-outline' : 'mdi-file-outline'"
						size="small"
						class="mr-1"
					/>
					<span class="name">{{ basename(entry.path) }}</span>
					<span
						v-if="!entry.dir"
						class="size"
					>{{ formatBytes(entry.size) }}</span>
					<VBtn
						icon="mdi-download"
						size="x-small"
						variant="text"
						:title="t('share.download')"
						@click="download(entry)"
					/>
				</div>
			</div>
			<VBtn
				size="small"
				variant="text"
				prepend-icon="mdi-folder-open-outline"
				class="mt-1"
				@click="openLocal"
			>
				{{ t("share.openLocal") }}
			</VBtn>

			<template v-if="transfers.length">
				<div class="section-title mt-3">
					{{ t("share.transfers") }}
				</div>
				<div
					v-for="transfer in transfers"
					:key="`${transfer.direction}-${transfer.id}`"
					class="transfer"
				>
					<div class="d-flex align-center">
						<VIcon
							:icon="transfer.direction === 'up' ? 'mdi-arrow-up' : 'mdi-arrow-down'"
							size="small"
							class="mr-1"
						/>
						<span class="name">{{ transfer.name }}</span>
						<VIcon
							v-if="transfer.state === 'done'"
							icon="mdi-check"
							color="success"
							size="small"
						/>
						<VIcon
							v-else-if="transfer.state === 'error'"
							icon="mdi-alert-circle-outline"
							color="error"
							size="small"
							:title="transfer.message"
						/>
					</div>
					<VProgressLinear
						v-if="transfer.state === 'active'"
						:model-value="transfer.total ? (transfer.bytes / transfer.total) * 100 : 0"
						height="4"
						rounded
					/>
					<p
						v-if="transfer.state === 'error'"
						class="text-caption text-error"
					>
						{{ transfer.message }}
					</p>
				</div>
			</template>
		</VCardText>
	</VCard>
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

	type SharedEntry = {
		path: string;
		size: number;
		dir: boolean;
	};

	type Transfer = {
		id: number;
		name: string;
		direction: "up" | "down";
		bytes: number;
		total: number;
		state: "active" | "done" | "error";
		message?: string;
	};

	defineProps<{
		clipboard: boolean;
	}>();
	const emit = defineEmits<{
		close: [];
	}>();

	const { t } = useI18n();
	const entries = ref<SharedEntry[]>([]);
	const transfers = ref<Transfer[]>([]);
	// Only the most recent transfers stay listed.
	const MAX_TRANSFERS = 50;
	const unlisten: UnlistenFn[] = [];

	function basename(path: string): string {
		return path.split("/").pop() ?? path;
	}

	function depth(path: string): number {
		return path.split("/").length - 1;
	}

	function formatBytes(bytes: number): string {
		const units = ["B", "KB", "MB", "GB"];
		let value = bytes;
		let unit = 0;
		while (value >= 1024 && unit < units.length - 1) {
			value /= 1024;
			unit++;
		}
		return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}

	async function refresh(): Promise<void> {
		await invoke("lan_share_list").catch(() => { /* stream gone */ });
	}

	async function pick(folder: boolean): Promise<void> {
		await invoke("lan_share_pick", { folder }).catch(() => { /* cancelled or gone */ });
		// Uploaded files show up in the host listing.
		await refresh();
	}

	async function download(entry: SharedEntry): Promise<void> {
		const paths = entry.dir
			? entries.value
				.filter((e) => !e.dir && e.path.startsWith(`${entry.path}/`))
				.map((e) => e.path)
			: [entry.path];
		await invoke("lan_share_download", { paths }).catch(() => { /* stream gone */ });
	}

	async function openLocal(): Promise<void> {
		await invoke("lan_share_open_folder").catch(() => { /* no file manager */ });
	}

	function track(transfer: Transfer): void {
		const index = transfers.value.findIndex((x) => x.id === transfer.id && x.direction === transfer.direction);
		if (index >= 0) {
			transfers.value[index] = transfer;
		} else {
			transfers.value.unshift(transfer);
			transfers.value.splice(MAX_TRANSFERS);
		}
	}

	onMounted(async () => {
		unlisten.push(await listen<SharedEntry[]>("lan://shared-files", (e) => {
			entries.value = e.payload;
		}));
		unlisten.push(await listen<Transfer>("lan://transfer", (e) => {
			track(e.payload);
			if (e.payload.direction === "up" && e.payload.state === "done") {
				void refresh();
			}
		}));
		await refresh();
	});

	onUnmounted(() => {
		unlisten.forEach((stop) => stop());
	});
</script>

<style scoped>
	.share-panel {
		position: absolute;
		top: 12px;
		right: 12px;
		bottom: 12px;
		width: min(360px, calc(100% - 24px));
		z-index: 5;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.share-panel :deep(.v-card-text) {
		overflow-y: auto;
	}

	.section-title {
		font-weight: 600;
		font-size: 0.85rem;
		margin-bottom: 4px;
	}

	.host-list {
		max-height: 40vh;
		overflow-y: auto;
	}

	.entry,
	.transfer {
		display: flex;
		flex-direction: column;
		font-size: 0.85rem;
	}

	.entry {
		flex-direction: row;
		align-items: center;
		min-height: 28px;
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.size {
		opacity: 0.7;
		margin: 0 4px;
		font-size: 0.75rem;
	}
</style>
