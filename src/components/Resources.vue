<template>
	<section>
		<h3 class="rl-section-title">
			{{ t("resources.title") }}
		</h3>
		<div class="rl-group">
			<template v-if="usage">
				<div class="rl-row">
					<span class="label">{{ t("resources.cpu") }}</span>
					<VProgressLinear
						:model-value="usage.cpu_percent"
						color="secondary"
						height="6"
						rounded
						class="flex-grow-1"
						style="flex-basis: 120px"
					/>
					<span class="value rl-mono">{{ usage.cpu_percent.toFixed(1) }}%</span>
				</div>
				<div class="rl-row">
					<span class="label">{{ t("resources.memory") }}</span>
					<VProgressLinear
						:model-value="memPercent"
						color="success"
						height="6"
						rounded
						class="flex-grow-1"
						style="flex-basis: 120px"
					/>
					<span class="value rl-mono">{{ formatBytes(usage.mem_bytes) }}</span>
				</div>
				<div class="rl-row rl-row-sub">
					{{ t("resources.ofTotal", { total: formatBytes(usage.total_mem_bytes) }) }}
					· {{ t("resources.cores", { count: usage.cores }) }}
				</div>
			</template>
			<div v-else class="rl-row rl-row-sub">
				{{ t("resources.unavailable") }}
			</div>
		</div>
	</section>
</template>

<script setup lang="ts">
	import {
		computed, onMounted, onUnmounted, ref,
	} from "vue";
	import { invoke } from "@tauri-apps/api/core";
	import { useI18n } from "vue-i18n";

	type ResourceUsage = {
		cpu_percent: number;
		mem_bytes: number;
		total_mem_bytes: number;
		cores: number;
	};

	const { t } = useI18n();
	const usage = ref<ResourceUsage | null>(null);
	const timer = ref<ReturnType<typeof setInterval>>();

	const memPercent = computed(() => {
		if (!usage.value || usage.value.total_mem_bytes === 0) {
			return 0;
		}
		return (usage.value.mem_bytes / usage.value.total_mem_bytes) * 100;
	});

	function formatBytes(bytes: number): string {
		const mb = bytes / (1024 * 1024);
		if (mb >= 1024) {
			return `${(mb / 1024).toFixed(2)} GB`;
		}
		return `${Math.round(mb)} MB`;
	}

	async function sample(): Promise<void> {
		try {
			usage.value = await invoke<ResourceUsage>("resource_usage");
		} catch {
			usage.value = null;
		}
	}

	onMounted(async () => {
		await sample();
		// Poll a touch slower than a second — sysinfo needs a gap between samples
		// to compute CPU use, and this is a glanceable readout, not a profiler.
		timer.value = setInterval(sample, 1500);
	});

	onUnmounted(() => {
		if (timer.value) {
			clearInterval(timer.value);
		}
	});
</script>

<style scoped>
	.label {
		width: 80px;
		color: var(--rl-muted);
	}

	.value {
		min-width: 72px;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
</style>
