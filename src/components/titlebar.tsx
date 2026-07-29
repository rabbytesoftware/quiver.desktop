import { isMacOS } from '@/lib/platform';

/**
 * The window's drag strip — on macOS, and only there.
 *
 * `tauri.macos.conf.json` sets `titleBarStyle: "Overlay"` with `hiddenTitle`,
 * so the webview extends under the system title bar and macOS draws nothing up
 * there but the traffic lights. That buys the native look at a cost: the window
 * loses every draggable surface it had, because the frontend now owns those
 * pixels. `data-tauri-drag-region` is what hands dragging back.
 *
 * Linux and Windows get none of that, deliberately. Those three settings are
 * macOS-only — which is why they live in the macOS config file rather than the
 * shared one — so there the OS draws a real title bar, with real controls and
 * a real drag surface. Rendering this strip as well would stack an empty 48px
 * band underneath the system one, with 80px of padding reserved for three
 * buttons that are somewhere else entirely. Nothing here can improve on a
 * native title bar, so nothing here is rendered.
 *
 * That makes `decorations` load-bearing off macOS: it is what draws the ONLY
 * chrome those windows have, and `null` here plus `decorations: false` there is
 * a window with no close button, no minimise and no way to drag it — which is
 * what `tauri.windows.conf.json` produced until it was deleted. It defaults to
 * true, no platform config overrides it, and titlebar.test.tsx asserts that for
 * every platform.
 *
 * TRAFFIC LIGHT COUPLING: the left padding here and `trafficLightPosition` in
 * tauri.macos.conf.json describe the same three buttons from two different
 * sides. The buttons are 12px across, so `y: 18` centers them in this 48px
 * (`h-12`) bar, and `pl-20` leaves the ~64px they occupy clear. Change the
 * bar's height and the `y` has to move with it, or the buttons end up visibly
 * off-centre — titlebar.test.tsx reads both sides and fails when they disagree.
 */
export function Titlebar() {
	if (!isMacOS()) return null;

	return <header data-tauri-drag-region className="flex h-12 shrink-0 select-none items-center pl-20 pr-3" />;
}
