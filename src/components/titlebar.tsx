/**
 * The window's drag strip.
 *
 * `tauri.conf.json` sets `titleBarStyle: "Overlay"` with `hiddenTitle`, so the
 * webview extends under the system title bar and macOS draws nothing up there
 * but the traffic lights. That buys the native look at a cost: the window loses
 * every draggable surface it had, because the frontend now owns those pixels.
 * `data-tauri-drag-region` is what hands dragging back.
 *
 * TRAFFIC LIGHT COUPLING: the left padding here and `trafficLightPosition` in
 * tauri.conf.json describe the same three buttons from two different sides, and
 * nothing checks that they agree. The buttons are 12px across, so `y: 18`
 * centers them in this 48px (`h-12`) bar, and `pl-20` leaves the ~64px they
 * occupy clear. Change the bar's height and the `y` has to move with it, or the
 * buttons end up visibly off-centre.
 */
export function Titlebar() {
	return <header data-tauri-drag-region className="flex h-12 shrink-0 select-none items-center pl-20 pr-3" />;
}
