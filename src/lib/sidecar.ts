import { Child, Command } from "@tauri-apps/plugin-shell"

const DEFAULT_PORT = 6982

let sidecarProcess: Child | null = null

export async function startDaemon(port: number = DEFAULT_PORT): Promise<void> {
	if (sidecarProcess) {
		return
	}

	const command = Command.sidecar("binaries/quiver", [
		"daemon",
		"--port",
		String(port),
	])

	command.on("error", (error) => {
		console.error("Sidecar error:", error)
		sidecarProcess = null
	})

	command.on("close", ({ code }) => {
		console.log("Sidecar exited with code:", code)
		sidecarProcess = null
	})

	sidecarProcess = await command.spawn()
}

export async function stopDaemon(): Promise<void> {
	if (sidecarProcess) {
		await sidecarProcess.kill()
		sidecarProcess = null
	}
}
