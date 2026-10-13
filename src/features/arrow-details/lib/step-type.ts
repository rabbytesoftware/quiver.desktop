import type { StepType } from '@/domain/arrow';

import {
	BoxIcon,
	CircleHelpIcon,
	DownloadIcon,
	EyeOffIcon,
	ExternalLinkIcon,
	ListIcon,
	PackageOpenIcon,
	RadioIcon,
	UsbIcon,
	type LucideIcon,
} from 'lucide-react';

const STEP_TYPE_ICONS: Record<StepType, LucideIcon> = {
	dependencies: BoxIcon,
	extract: PackageOpenIcon,
	expose: ExternalLinkIcon,
	fetch: DownloadIcon,
	portable: UsbIcon,
	run: ListIcon,
	signal: RadioIcon,
	unexpose: EyeOffIcon,
};

export function isKnownStepType(type: string): type is StepType {
	return Object.prototype.hasOwnProperty.call(STEP_TYPE_ICONS, type);
}

/** `type` is a bare string off the wire, so an unrecognised kind gets a generic icon instead of an undefined component. */
export function iconForStepType(type: string): LucideIcon {
	return isKnownStepType(type) ? STEP_TYPE_ICONS[type] : CircleHelpIcon;
}
