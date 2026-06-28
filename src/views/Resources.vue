<template>
	<VContainer style="max-width: 880px">
		<VCard variant="tonal">
			<VCardTitle>{{ t("resources.title") }}</VCardTitle>
			<VCardSubtitle class="text-wrap">
				{{ t("resources.subtitle") }}
			</VCardSubtitle>
			<VCardText v-if="usage">
				<div class="text-overline text-medium-emphasis">
					{{ t("resources.cpu") }}
				</div>
				<div class="d-flex align-center ga-3 mb-4">
					<VProgressLinear
						:model-value="usage.cpu_percent"
						color="primary"
						height="10"
						rounded
					/>
					<span class="value">{{ usage.cpu_percent.toFixed(1) }}%</span>
				</div>

				<div class="text-overline text-medium-emphasis">
					{{ t("resources.memory") }}
				</div>
				<div class="d-flex align-center ga-3">
					<VProgressLinear
						:model-value="memPercent"
						color="teal"
						height="10"
						rounded
					/>
					<span class="value">{{ formatBytes(usage.mem_bytes) }}</span>
				</div>
				<p class="text-caption text-medium-emphasis mt-2 mb-0">
					{{ t("resources.ofTotal", { total: formatBytes(usage.total_mem_bytes) }) }}
					· {{ t("resources.cores", { count: usage.cores }) }}
				</p>
			</VCardText>
			<VCardText v-else>
				<p class="text-body-2 text-medium-emphasis mb-0">
					{{ t("resources.unavailable") }}
				</p>
			</VCardText>
		</VCard>
	</VContainer>
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
	.value {
		min-width: 96px;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
</style>
