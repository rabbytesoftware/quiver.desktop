/** `GET` and `POST /v0/system/path`. `files` is always an array on a real daemon. */
export interface PathStatusDTO {
	bin_dir: string;
	on_path: boolean;
	configured: boolean;
	files: string[] | null;
}

export interface PathStatus {
	binDir: string;
	/** Whether the daemon's own `PATH` already has the directory. */
	onPath: boolean;
	/** Whether the directory is set to stay on `PATH` in new terminals. */
	configured: boolean;
	/** Where that setting is checked and written: shell files, or the user `Path` on Windows. */
	files: string[];
}

export function toPathStatus(dto: PathStatusDTO): PathStatus {
	return {
		binDir: dto.bin_dir,
		onPath: dto.on_path === true,
		configured: dto.configured === true,
		files: dto.files ?? [],
	};
}
