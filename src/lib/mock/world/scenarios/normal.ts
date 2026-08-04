// The default world. Failures are data rather than knobs, because none of the
// interesting ones are 500s: no target for your platform is a 422, a
// rate-limited provider is a 429 inside a successful job, an unresolved
// collection member is a perfectly good 200.

import type { MockArrow, MockCollection, MockProvider } from '../types';
import {
	arrow,
	HOST_PLATFORM,
	INSTALL_STEPS,
	method,
	OTHER_PLATFORM,
	START_STEPS,
	stepsAllDone,
	stepsFailedAt,
	target,
	variable,
} from './kit';
import { DEMO_BANNER, DEMO_ICON } from './media';

const NS = 'github.com/rabbyte';

export const NORMAL_ARROWS: MockArrow[] = [
	// The showcase: media, variables, ports, a real method set.
	arrow({
		namespace: `${NS}/minecraft`,
		name: 'Minecraft Server',
		description: 'Vanilla Minecraft dedicated server with automatic world backups.',
		ref: 'v1.21.4',
		version: '1.21.4',
		state: 'running',
		tags: ['game', 'server', 'java'],
		icon: DEMO_ICON,
		banner: DEMO_BANNER,
		requirement: { cpu_cores: 4, memory_gb: 8, disk_gb: 40 },
		netbridge: [
			{ name: 'game', protocol: 'tcp', default: 25565, required: true },
			{ name: 'rcon', protocol: 'tcp', default: 25575, required: false },
		],
		variables: [
			variable('MOTD', 'Message shown in the server list', 'string', { default: 'A Quiver server' }),
			variable('MAX_PLAYERS', 'Maximum concurrent players', 'number', { default: '20', min: 1, max: 200 }),
			variable('DIFFICULTY', 'World difficulty', 'select', {
				default: 'normal',
				values: ['peaceful', 'easy', 'normal', 'hard'],
			}),
			variable('ONLINE_MODE', 'Verify players against Mojang auth', 'boolean', { default: 'true' }),
			variable('RCON_PASSWORD', 'Password for remote console access', 'string', { sensitive: true }),
		],
		targets: [
			target(HOST_PLATFORM, [
				method('start', 'Start the server', ['ready'], START_STEPS),
				method('stop', 'Stop the server', ['running'], ['Save world', 'Signal process', 'Await exit']),
				method(
					'backup',
					'Snapshot the world directory',
					['ready', 'running'],
					['Freeze world', 'Copy region files', 'Compress']
				),
				method('rcon', 'Open a remote console session', ['running'], ['Connect', 'Authenticate']),
			]),
		],
		active_run: {
			method: 'start',
			pid: 48213,
			variables: { MOTD: 'A Quiver server', MAX_PLAYERS: '20', DIFFICULTY: 'normal', ONLINE_MODE: 'true' },
			steps: stepsAllDone(START_STEPS),
		},
	}),

	// No target for the host platform — install would 422.
	arrow({
		namespace: `${NS}/valheim`,
		name: 'Valheim Server',
		description: 'Dedicated Valheim server. Linux only — no macOS build is published upstream.',
		ref: 'v0.218.15',
		version: '0.218.15',
		state: 'absent',
		tags: ['game', 'server'],
		icon: DEMO_ICON,
		requirement: { cpu_cores: 4, memory_gb: 8, disk_gb: 20 },
		netbridge: [{ name: 'game', protocol: 'udp', default: 2456, required: true }],
		variables: [
			variable('WORLD_NAME', 'Name of the world to load or create', 'string', { default: 'Dedicated' }),
			variable('SERVER_PASSWORD', 'Password required to join', 'string', { sensitive: true }),
		],
		targets: [target(OTHER_PLATFORM, [method('start', 'Start the server', ['ready'], START_STEPS)])],
	}),

	// Last install died at step 3 of 5.
	arrow({
		namespace: `${NS}/factorio`,
		name: 'Factorio Headless',
		description: 'Headless Factorio server for co-op games.',
		ref: 'v2.0.28',
		version: '2.0.28',
		state: 'absent',
		tags: ['game', 'server'],
		last_return: {
			method: 'install',
			outcome: 'failed',
			variables: {},
			steps: stepsFailedAt(INSTALL_STEPS, 2, 'checksum mismatch: expected sha256:9f2c…, got sha256:41ab…'),
		},
	}),

	// A newer ref exists upstream.
	arrow({
		namespace: `${NS}/terraria`,
		name: 'Terraria Server',
		description: 'tModLoader-compatible Terraria dedicated server.',
		ref: 'v1.4.4.9',
		version: '1.4.4.9',
		state: 'outdated',
		tags: ['game', 'server'],
		netbridge: [{ name: 'game', protocol: 'tcp', default: 7777, required: true }],
	}),

	// A package: no methods at all, so no Run button may be offered.
	arrow({
		namespace: `${NS}/pixelmon-assets`,
		name: 'Pixelmon Asset Pack',
		description: 'Texture and model assets. A package — there is nothing to run.',
		ref: 'v9.2.1',
		version: '9.2.1',
		state: 'ready',
		tags: ['assets', 'package'],
		requirement: { cpu_cores: 1, memory_gb: 1, disk_gb: 6 },
		targets: [target(HOST_PLATFORM, [])],
		last_return: { method: 'install', outcome: 'success', variables: {}, steps: stepsAllDone(INSTALL_STEPS) },
	}),

	// The remaining states, so all eleven are reachable at once.
	arrow({
		namespace: `${NS}/postgres`,
		name: 'PostgreSQL',
		description: 'PostgreSQL 17 with the timescale extension preinstalled.',
		ref: 'v17.2',
		version: '17.2',
		state: 'ready',
		tags: ['database', 'service'],
		netbridge: [{ name: 'sql', protocol: 'tcp', default: 5432, required: true }],
		variables: [
			variable('POSTGRES_USER', 'Superuser name', 'string', { default: 'quiver' }),
			variable('POSTGRES_PASSWORD', 'Superuser password', 'string', { sensitive: true }),
		],
		last_return: { method: 'install', outcome: 'success', variables: {}, steps: stepsAllDone(INSTALL_STEPS) },
	}),
	arrow({
		namespace: `${NS}/redis`,
		name: 'Redis',
		description: 'Redis 7 keystore.',
		ref: 'v7.4.1',
		version: '7.4.1',
		state: 'installing',
		tags: ['database', 'service'],
		active_run: {
			method: 'install',
			variables: {},
			steps: [
				{ index: 0, title: INSTALL_STEPS[0], status: 'completed', type: 'exec' },
				{ index: 1, title: INSTALL_STEPS[1], status: 'running', type: 'exec' },
				{ index: 2, title: INSTALL_STEPS[2], status: 'pending', type: 'exec' },
				{ index: 3, title: INSTALL_STEPS[3], status: 'pending', type: 'exec' },
				{ index: 4, title: INSTALL_STEPS[4], status: 'pending', type: 'exec' },
			],
		},
	}),
	arrow({
		namespace: `${NS}/caddy`,
		name: 'Caddy',
		description: 'Automatic-HTTPS reverse proxy.',
		ref: 'v2.8.4',
		version: '2.8.4',
		state: 'updating',
		tags: ['network', 'service'],
	}),
	arrow({
		namespace: `${NS}/grafana`,
		name: 'Grafana',
		description: 'Dashboards for the metrics your arrows emit.',
		ref: 'v11.4.0',
		version: '11.4.0',
		state: 'stopping',
		tags: ['observability', 'service'],
	}),
	arrow({
		namespace: `${NS}/nats`,
		name: 'NATS',
		description: 'Message broker. Draining connections before shutdown.',
		ref: 'v2.10.24',
		version: '2.10.24',
		state: 'draining',
		tags: ['network', 'service'],
	}),
	arrow({
		namespace: `${NS}/syncthing`,
		name: 'Syncthing',
		description: 'Running, but the daemon lost track of the process it started.',
		ref: 'v1.28.1',
		version: '1.28.1',
		state: 'detached',
		tags: ['storage', 'service'],
	}),
	arrow({
		namespace: `${NS}/jellyfin`,
		name: 'Jellyfin',
		description: 'Media server. Currently being removed.',
		ref: 'v10.10.3',
		version: '10.10.3',
		state: 'uninstalling',
		tags: ['media', 'service'],
	}),
	arrow({
		namespace: `${NS}/plex`,
		name: 'Plex',
		description: 'Removed from disk; the entry survives until the catalog prunes it.',
		ref: 'v1.41.3',
		version: '1.41.3',
		state: 'removed',
		tags: ['media', 'service'],
	}),

	// Not in the library. Only reachable through search.
	arrow({
		namespace: `${NS}/mariadb`,
		name: 'MariaDB',
		description: 'Drop-in MySQL replacement.',
		ref: 'v11.6.2',
		version: '11.6.2',
		state: 'absent',
		user_installed: false,
		tags: ['database', 'service'],
	}),
	arrow({
		namespace: `${NS}/rust-server`,
		name: 'Rust Dedicated Server',
		description: 'Dedicated server for Rust, with Oxide mod support.',
		ref: 'v2412.1',
		version: '2412.1',
		state: 'absent',
		user_installed: false,
		tags: ['game', 'server'],
		icon: DEMO_ICON,
		banner: DEMO_BANNER,
		requirement: { cpu_cores: 8, memory_gb: 16, disk_gb: 60 },
	}),
	arrow({
		namespace: `${NS}/satisfactory`,
		name: 'Satisfactory Server',
		description: 'Dedicated server for Satisfactory.',
		ref: 'v1.0.1',
		version: '1.0.1',
		state: 'absent',
		user_installed: false,
		tags: ['game', 'server'],
	}),
	arrow({
		namespace: `${NS}/vaultwarden`,
		name: 'Vaultwarden',
		description: 'Bitwarden-compatible password server.',
		ref: 'v1.32.7',
		version: '1.32.7',
		state: 'absent',
		user_installed: false,
		tags: ['security', 'service'],
	}),
];

export const NORMAL_COLLECTIONS: MockCollection[] = [
	{
		namespace: `${NS}/game-servers`,
		name: 'Game Servers',
		description: 'Everything needed to host a weekend with friends.',
		maintainers: ['rabbyte'],
		followed: true,
		arrows: [
			{ namespace: `${NS}/minecraft@v1.21.4`, resolved: true },
			{ namespace: `${NS}/terraria@v1.4.4.9`, resolved: true },
			{ namespace: `${NS}/factorio@v2.0.28`, resolved: true },
			{
				namespace: `${NS}/ark-survival@v3.1.0`,
				resolved: false,
				reason: 'ref v3.1.0 was yanked upstream',
			},
		],
	},
	{
		namespace: `${NS}/homelab`,
		name: 'Homelab Essentials',
		description: 'Reverse proxy, database, metrics.',
		maintainers: ['rabbyte', 'char2cs'],
		followed: false,
		arrows: [
			{ namespace: `${NS}/caddy@v2.8.4`, resolved: true },
			{ namespace: `${NS}/postgres@v17.2`, resolved: true },
			{ namespace: `${NS}/grafana@v11.4.0`, resolved: true },
		],
	},
];

/**
 * One host answers, one refuses. That second row is why the job returns
 * per-provider outcomes: "rate-limited, retry in 40s" and "nothing found" are
 * opposite facts, and rendering the first as the second is the most misleading
 * thing a search screen can do.
 */
export const NORMAL_PROVIDERS: MockProvider[] = [
	{ host: 'github.com', ok: true, returned: 4 },
	{ host: 'gitlab.com', ok: false, returned: 0, reason: 'rate limited', retry_after: 40 },
];
