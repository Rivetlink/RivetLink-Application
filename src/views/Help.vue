<template>
	<div class="rl-page">
		<header>
			<h1 class="rl-h1">
				{{ t("help.title") }}
			</h1>
			<p class="rl-lead">
				{{ t("help.intro") }}
			</p>
		</header>

		<section v-for="s in sections" :key="s.id" class="rl-group panel">
			<h2 class="rl-h2 d-flex align-center ga-3">
				<VIcon :icon="s.icon" color="primary" size="22" />
				{{ t(`help.${s.id}.title`) }}
			</h2>
			<p class="rl-row-sub ma-0">
				{{ t(`help.${s.id}.intro`) }}
			</p>
			<ol class="steps">
				<li v-for="n in s.steps" :key="n">
					<span class="step-no">{{ n }}</span>
					<span>{{ t(`help.${s.id}.step${n}`) }}</span>
				</li>
			</ol>
			<div v-if="s.actions.length" class="d-flex flex-wrap ga-2">
				<VBtn
					v-for="a in s.actions"
					:key="a.to"
					variant="outlined"
					:prepend-icon="a.icon"
					:to="a.to"
				>
					{{ t(`help.actions.${a.key}`) }}
				</VBtn>
			</div>
		</section>
	</div>
</template>

<script setup lang="ts">
	import { useI18n } from "vue-i18n";

	const { t } = useI18n();

	type Action = {
		key: string;
		to: string;
		icon: string
	};

	// The Ubuntu-console settings exist only on Linux.
	const onLinux = navigator.userAgent.includes("Linux");

	const sections: {
		id: string;
		icon: string;
		steps: number;
		actions: Action[]
	}[] = [
		{
			id: "lan",
			icon: "mdi-wifi",
			steps: 3,
			actions: [
				{
					key: "receiveHelp",
					to: "/device",
					icon: "mdi-lifebuoy",
				},
				{
					key: "scan",
					to: "/connect?scan=1",
					icon: "mdi-wifi",
				},
			],
		},
		{
			id: "relay",
			icon: "mdi-server-network",
			steps: 3,
			actions: [
				{
					key: "servers",
					to: "/settings?s=servers",
					icon: "mdi-server-plus",
				},
				{
					key: "devices",
					to: "/connect",
					icon: "mdi-monitor",
				},
			],
		},
		{
			id: "addHost",
			icon: "mdi-monitor-arrow-down",
			steps: 4,
			actions: [
				{
					key: "security",
					to: "/settings?s=security",
					icon: "mdi-key-outline",
				},
				...(onLinux
					? [{
						key: "console",
						to: "/settings?s=console",
						icon: "mdi-ubuntu",
					}]
					: []),
			],
		},
		{
			id: "routes",
			icon: "mdi-transit-connection-variant",
			steps: 3,
			actions: [],
		},
		{
			id: "trouble",
			icon: "mdi-wrench-outline",
			steps: 4,
			actions: [],
		},
	];
</script>

<style scoped>
	.panel {
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.steps {
		margin: 0;
		padding: 0;
		list-style: none;
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
		gap: 12px;
	}

	.steps li {
		padding: 14px;
		font-size: 14px;
		border-radius: 10px;
		background: var(--rl-sidebar);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.step-no {
		font-family: var(--rl-mono);
		font-size: 12px;
		color: var(--rl-accent);
	}
</style>
